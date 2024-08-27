mod decoder;

use byteorder::{LittleEndian, WriteBytesExt};
use decoder::Decoder;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xa_files = vec![
        "bass.xa",
        // "hightom.xa",
        // "midtom.xa",
        // "floortom.xa",
        // "hihatclose.xa",
    ];

    for xa_file in xa_files {
        println!("Processing file: {}", xa_file);

        match process_file(xa_file) {
            Ok(_) => println!("File processed successfully"),
            Err(e) => println!("Error processing file: {}", e),
        }

        println!();
    }

    Ok(())
}

fn process_file(xa_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut xa = File::open(xa_file)?;
    let mut decoder = Decoder::new();
    let format = decoder.read_from_file(&mut xa).map_err(|e| e.to_string())?;

    println!("Format: {:?}", format);

    let xa_data_size = format.blocks as usize * format.block_size_xa as usize;
    let mut xa_data = vec![0u8; xa_data_size];
    xa.read_exact(&mut xa_data)?;

    let pcm_data_size = format.data_length_pcm as usize / 2; // 16-bit samples
    let mut pcm_data: Vec<i16> = vec![0i16; pcm_data_size];

    let blocks_decoded = decoder.decode(&xa_data, &mut pcm_data)?;

    println!("Decoded {} blocks", blocks_decoded);

    // Print first 200 samples
    for sample in &pcm_data[..200] {
        print!("{} ", sample);
    }

    let wav_file = xa_file.replace(".xa", ".wav");
    let mut wav = File::create(&wav_file)?;
    decoder.write_wav_header(&mut wav)?;

    for sample in &pcm_data[..pcm_data_size] {
        wav.write_i16::<LittleEndian>(*sample)?;
    }

    println!("WAV file written to: {}", wav_file);

    Ok(())
}
