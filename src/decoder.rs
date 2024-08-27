
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{Write};

pub const HEADER_SIZE_XA: usize = 32;
const HEADER_SIZE_RIFF: usize = 44;
const HEADER_MAGIC: u32 = 0x3144574b;
const BLOCK_SAMPLES: u32 = 32;
const WAVE_HEADER_LEN: u32 = 16;
const WAVE_FORMAT_PCM: u16 = 1;
const GAIN_FACTOR: [[i16; 2]; 5] = [[0, 0], [240, 0], [460, -208], [392, -220], [488, -240]];

#[derive(Debug, Clone)]
pub struct Format {
    pub data_length_pcm: u32,
    pub blocks: u32,
    pub block_size_pcm: u32,
    pub block_size_xa: u32,
    pub samples_rate: u16,
    pub sample_bits: u16,
    pub channels: u16,
}

impl Format {
    fn riff_header_length(&self) -> u32 {
        HEADER_SIZE_RIFF as u32 - 8 + self.data_length_pcm
    }

    fn wave_byte_rate(&self) -> u32 {
        self.samples_rate as u32 * self.block_size_pcm / BLOCK_SAMPLES
    }

    fn wave_block_align(&self) -> u16 {
        self.channels * self.sample_bits / 8
    }
}

#[derive(Clone)]
struct ChannelState {
    prev0: i16,
    prev1: i16,
}

struct DecoderState {
    data_length: u32,
    samples: u32,
    samples_rate: u16,
    block_size: u32,
    channels: u16,
    lr: [ChannelState; 2],
    inflate_func: fn(&DecoderState, &mut [i16], usize, &[u8]) -> u8,
}

impl DecoderState {
    fn is_valid(&self) -> bool {
        if self.data_length == 0
            || self.samples == 0
            || self.samples_rate == 0
            || self.block_size == 0
        {
            return false;
        }

        if self.channels != 1 && self.channels != 2 {
            return false;
        }

        let blocks = self.data_length / self.block_size;
        let max_samples =
            (BLOCK_SAMPLES * self.data_length) / (self.block_size * self.channels as u32);

        if blocks * self.block_size != self.data_length {
            return false;
        }

        if self.samples > max_samples {
            return false;
        }

        if max_samples - self.samples >= BLOCK_SAMPLES {
            return false;
        }

        true
    }

    fn to_format(&self) -> Format {
        Format {
            data_length_pcm: self.samples * self.channels as u32 * 2,
            blocks: self.data_length / (self.block_size * self.channels as u32),
            block_size_pcm: BLOCK_SAMPLES * self.channels as u32,
            block_size_xa: self.block_size * self.channels as u32,
            samples_rate: self.samples_rate,
            sample_bits: 16,
            channels: self.channels,
        }
    }
}


pub struct Decoder {
    pub state: Option<DecoderState>,
    pub fmt: Option<Format>,
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            state: None,
            fmt: None,
        }
    }

    pub fn read_header(&mut self, header: &[u8]) -> Result<Format, std::io::Error> {
        println!("Header contents:");
        for (i, &byte) in header.iter().enumerate() {
            print!("{:02X} ", byte);
            if (i + 1) % 16 == 0 {
                println!();
            }
        }
        println!();

        let magic = (&header[0..4]).read_u32::<LittleEndian>()?;
        println!("Magic: {:X}", magic);
        let data_length_xa = (&header[4..8]).read_u32::<LittleEndian>()?;
        println!("Data Length XA: {}", data_length_xa);
        let samples = (&header[8..12]).read_u32::<LittleEndian>()?;
        let data_length_pcm = samples * 2; // 16-bit samples
        println!("Data Length PCM: {}", data_length_pcm);
        let samples_rate = (&header[12..14]).read_u16::<LittleEndian>()?;
        println!("Sample rate: {}", samples_rate);
        let bits = header[14];
        println!("Bit depth: {}", bits);
        let channels = header[15] as u16;
        println!("Channels: {}", channels);

        if magic != HEADER_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid XA header magic: {:X}", magic),
            ));
        }

        if bits != 4 && bits != 6 && bits != 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid bit depth: {}", bits),
            ));
        }

        if channels != 1 && channels != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid number of channels: {}", channels),
            ));
        }

        let lr0_prev0 = (&header[20..22]).read_i16::<LittleEndian>()?;
        let lr0_prev1 = (&header[22..24]).read_i16::<LittleEndian>()?;
        let lr1_prev0 = (&header[24..26]).read_i16::<LittleEndian>()?;
        let lr1_prev1 = (&header[26..28]).read_i16::<LittleEndian>()?;

        let inflate_func = Self::block_inflater(bits);
        let block_size = bits as u32 * 4 + 1;

        let tmp = DecoderState {
            data_length: data_length_xa,
            samples,
            samples_rate,
            block_size,
            channels,
            lr: [
                ChannelState {
                    prev0: lr0_prev0,
                    prev1: lr0_prev1,
                },
                ChannelState {
                    prev0: lr1_prev0,
                    prev1: lr1_prev1,
                },
            ],
            inflate_func,
        };

        if !tmp.is_valid() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid XA header data",
            ));
        }

        let fmt = tmp.to_format();
        self.state = Some(tmp);
        self.fmt = Some(fmt.clone());

        Ok(fmt)
    }

    fn block_inflater(bits: u8) -> fn(&DecoderState, &mut [i16], usize, &[u8]) -> u8 {
        println!("Entering block_inflater with bits: {}", bits);
        match bits {
            4 => Self::inflate_4bit,
            6 => Self::inflate_6bit,
            8 => Self::inflate_8bit,
            _ => {
                println!("Unsupported bit depth in block_inflater: {}", bits);
                panic!("Unsupported bit depth: {}", bits)
            }
        }
    }

    fn inflate_4bit(dec: &DecoderState, dst: &mut [i16], off: usize, src: &[u8]) -> u8 {
        let profile = src[0];
        let mut src_off = 1;

        for n in (0..BLOCK_SAMPLES).step_by(2) {
            let s = src[src_off] as u16;
            src_off += 1;

            dst[off + n as usize * dec.channels as usize] = ((s & 0xf0) << 8) as i16;
            dst[off + (n + 1) as usize * dec.channels as usize] = ((s & 0x0f) << 12) as i16;
        }

        profile
    }

    fn inflate_6bit(state: &DecoderState, pcm: &mut [i16], ch: usize, xa: &[u8]) -> u8 {
        assert!(ch == 0 || ch == 1);
        assert_eq!(BLOCK_SAMPLES as usize * state.channels as usize, pcm.len());

        let profile = xa[0];
        let mut src_off = 1;
        let mut dst_off = ch;

        for _ in (0..BLOCK_SAMPLES).step_by(4) {
            let s = ((xa[src_off] as u32) << 16)
                | ((xa[src_off + 1] as u32) << 8)
                | (xa[src_off + 2] as u32);
            src_off += 3;

            pcm[dst_off] = ((s & 0x00fc0000) >> 8) as i16;
            dst_off += state.channels as usize;
            pcm[dst_off] = ((s & 0x0003f000) >> 2) as i16;
            dst_off += state.channels as usize;
            pcm[dst_off] = ((s & 0x00000fc0) << 4) as i16;
            dst_off += state.channels as usize;
            pcm[dst_off] = ((s & 0x0000003f) << 10) as i16;
            dst_off += state.channels as usize;
        }

        profile
    }

    fn inflate_8bit(dec: &DecoderState, dst: &mut [i16], off: usize, src: &[u8]) -> u8 {
        let profile = src[0];
        let mut src_off = 1;

        for n in 0..BLOCK_SAMPLES {
            dst[off + n as usize * dec.channels as usize] = ((src[src_off] as i16) << 8) as i16;
            src_off += 1;
        }

        profile
    }

    fn decode_inflated(&mut self, pcm: &mut [i16], ch: usize, prof: u8) {
        assert!(ch == 0 || ch == 1);

        let state = self.state.as_mut().unwrap();
        assert_eq!(BLOCK_SAMPLES as usize * state.channels as usize, pcm.len());

        let factor = (prof >> 4) as usize;
        let range = prof & 0x0f;

        if factor >= GAIN_FACTOR.len() {
            panic!("Invalid factor: {}", factor);
        }

        let k0 = GAIN_FACTOR[factor][0];
        let k1 = GAIN_FACTOR[factor][1];

        let channels = state.channels as usize;
        let mut off = ch;

        let mut prev0 = state.lr[ch].prev0;
        let mut prev1 = state.lr[ch].prev1;

        for _ in 0..BLOCK_SAMPLES {
            let ranged = pcm[off] >> range;
            let gain = (prev0 as i32 * k0 as i32 + prev1 as i32 * k1 as i32) / 256;
            let mut sample = ranged as i32 + gain;

            sample = sample.clamp(i16::MIN as i32, i16::MAX as i32);

            pcm[off] = sample as i16;
            prev1 = prev0;
            prev0 = sample as i16;

            off += channels;
        }

        state.lr[ch].prev0 = prev0;
        state.lr[ch].prev1 = prev1;
    }

    pub fn decode(&mut self, src: &[u8], pcm: &mut [i16]) -> Result<u32, String> {
        let format = self
            .fmt
            .as_ref()
            .ok_or_else(|| "Format not initialized".to_string())?;
        let blocks = format.blocks;
        let block_size_xa = format.block_size_xa as usize;
        let block_size_pcm = format.block_size_pcm as usize;

        let mut blocks_decoded = 0;
        let mut src_offset = 0;
        let mut pcm_offset = 0;
        let pcm_data_size = format.data_length_pcm as usize;
        println!("Blocks: {}", blocks);
        println!("Block size XA: {}", block_size_xa);
        println!("Block size PCM: {}", block_size_pcm);
        println!("Src len: {}", src.len());
        println!("Pcm len: {}", pcm.len());
        println!("Pcm data size: {}", pcm_data_size);

        while blocks_decoded < blocks
            && src_offset + block_size_xa <= src.len()
            && pcm_offset + block_size_pcm <= pcm.len()
        {
            let src_block = &src[src_offset..src_offset + block_size_xa];
            let pcm_block: &mut [i16] = &mut pcm[pcm_offset..pcm_offset + block_size_pcm];

            let prof = {
                let state = self
                    .state
                    .as_ref()
                    .ok_or_else(|| "Decoder state not initialized".to_string())?;
                (state.inflate_func)(state, pcm_block, 0, src_block)
            };

            self.decode_inflated(pcm_block, 0, prof);

            src_offset += block_size_xa;
            pcm_offset += block_size_pcm;
            blocks_decoded += 1;
        }

        Ok(blocks_decoded)
    }

    pub fn write_wav_header(&self, wav: &mut File) -> Result<(), std::io::Error> {
        let fmt = self.fmt.as_ref().unwrap();

        wav.write_all(b"RIFF")?;
        wav.write_u32::<LittleEndian>(fmt.riff_header_length())?;
        wav.write_all(b"WAVE")?;
        wav.write_all(b"fmt ")?;
        wav.write_u32::<LittleEndian>(WAVE_HEADER_LEN)?;
        wav.write_u16::<LittleEndian>(WAVE_FORMAT_PCM)?;
        wav.write_u16::<LittleEndian>(fmt.channels)?;
        wav.write_u32::<LittleEndian>(fmt.samples_rate as u32)?;
        wav.write_u32::<LittleEndian>(fmt.wave_byte_rate())?;
        wav.write_u16::<LittleEndian>(fmt.wave_block_align())?;
        wav.write_u16::<LittleEndian>(fmt.sample_bits)?;
        wav.write_all(b"data")?;
        wav.write_u32::<LittleEndian>(fmt.data_length_pcm)?;

        Ok(())
    }
}
