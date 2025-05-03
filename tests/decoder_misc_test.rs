use std::fs::File;
use std::io::Read;
use xa_decoder::decoder::{Decoder, HEADER_SIZE_XA};

// Helper function to create a valid XA header based on analysis of real XA files
// and the validation logic in the decoder
fn create_valid_header(bit_depth: u8, channels: u8) -> [u8; HEADER_SIZE_XA] {
    // Magic value: 0x3144574B (KWD1 in little endian)
    let _magic: u32 = 0x3144574B; // Not used directly as we write the bytes manually
    
    // Calculate proper values that will pass validation
    // Using values similar to real XA files
    let block_size = bit_depth as u32 * 4 + 1;
    let data_blocks = 100; // Number of blocks
    
    // For stereo, the data length needs to account for both channels
    let channels_u32 = channels as u32;
    
    // In the original code, block_size_xa includes both channels
    // data_length needs to be for *all* channels
    let data_length_xa = data_blocks * block_size * channels_u32;
    
    // For samples, we need to satisfy the validation conditions in is_valid()
    let samples_per_block = 32; // BLOCK_SAMPLES constant
    let samples = data_blocks * samples_per_block;
    
    let samples_rate: u16 = 22050; // Common sample rate
    
    let mut header = [0u8; HEADER_SIZE_XA];
    
    // Write magic number (KWD1 in little endian)
    header[0] = 0x4B;
    header[1] = 0x57;
    header[2] = 0x44;
    header[3] = 0x31;
    
    // Write data length
    header[4] = (data_length_xa & 0xFF) as u8;
    header[5] = ((data_length_xa >> 8) & 0xFF) as u8;
    header[6] = ((data_length_xa >> 16) & 0xFF) as u8;
    header[7] = ((data_length_xa >> 24) & 0xFF) as u8;
    
    // Write samples
    header[8] = (samples & 0xFF) as u8;
    header[9] = ((samples >> 8) & 0xFF) as u8;
    header[10] = ((samples >> 16) & 0xFF) as u8;
    header[11] = ((samples >> 24) & 0xFF) as u8;
    
    // Write samples rate
    header[12] = (samples_rate & 0xFF) as u8;
    header[13] = ((samples_rate >> 8) & 0xFF) as u8;
    
    // Write bit depth and channels
    header[14] = bit_depth;
    header[15] = channels;
    
    // Set previous values for channel states to 0
    // Bytes 16-19 appear to be reserved (0)
    // Bytes 20-27 are previous sample values
    for i in 16..28 {
        header[i] = 0;
    }
    
    // Bytes 28-31 appear to be reserved (0)
    for i in 28..32 {
        header[i] = 0;
    }
    
    header
}

// Helper function to create a small valid XA data buffer to go with header
fn create_xa_data(bit_depth: u8, blocks: u32, channels: u8) -> Vec<u8> {
    let block_size = bit_depth as u32 * 4 + 1;
    let total_size = block_size * blocks * channels as u32;
    
    // Create a buffer filled with pattern data based on bit depth
    let mut buffer = vec![0u8; total_size as usize];
    
    for ch in 0..channels {
        for block in 0..blocks {
            let block_start = (ch as u32 * blocks * block_size + block * block_size) as usize;
            
            // Set profile byte (first byte of each block)
            buffer[block_start] = 0x02; // Simple profile value for testing
            
            // Fill the rest with pattern data
            for i in 1..block_size as usize {
                // Different pattern for each channel for better testing
                buffer[block_start + i] = ((i % 255) as u8).wrapping_add(ch + 1);
            }
        }
    }
    
    buffer
}

#[test]
fn test_valid_headers() {
    let mut decoder = Decoder::new();
    
    // Test with valid 4-bit mono header
    let header_4bit_mono = create_valid_header(4, 1);
    let result = decoder.read_header(&header_4bit_mono);
    assert!(result.is_ok(), "Failed to parse valid 4-bit mono header");
    
    if let Ok(format) = result {
        assert_eq!(format.channels, 1);
        assert_eq!(format.sample_bits, 16);
        assert_eq!(format.samples_rate, 22050);
    }
    
    // Test with valid 6-bit stereo header
    let header_6bit_stereo = create_valid_header(6, 2);
    let result = decoder.read_header(&header_6bit_stereo);
    assert!(result.is_ok(), "Failed to parse valid 6-bit stereo header");
    
    if let Ok(format) = result {
        assert_eq!(format.channels, 2);
        assert_eq!(format.sample_bits, 16);
        assert_eq!(format.samples_rate, 22050);
    }
    
    // Test with valid 8-bit mono header
    let header_8bit_mono = create_valid_header(8, 1);
    let result = decoder.read_header(&header_8bit_mono);
    assert!(result.is_ok(), "Failed to parse valid 8-bit mono header");
    
    if let Ok(format) = result {
        assert_eq!(format.channels, 1);
        assert_eq!(format.sample_bits, 16);
        assert_eq!(format.samples_rate, 22050);
    }
}

#[test]
fn test_decode_synthetic_data() {
    let mut decoder = Decoder::new();
    
    // Create a valid 4-bit mono header and data
    let header = create_valid_header(4, 1);
    let format = decoder.read_header(&header).expect("Failed to parse header");
    
    // Create synthetic XA data
    let blocks = 5;
    let channels = 1; // Mono for this test
    let xa_data = create_xa_data(4, blocks, channels);
    
    // Create PCM buffer
    let pcm_data_size = format.data_length_pcm as usize / 2; // 16-bit samples
    let mut pcm_data = vec![0i16; pcm_data_size];
    
    // Only decode the blocks we created
    let pcm_slice = &mut pcm_data[0..(blocks * 32) as usize];
    
    // Decode the data
    let blocks_decoded = decoder.decode(&xa_data, pcm_slice).expect("Failed to decode XA data");
    
    // Validate results
    assert_eq!(blocks_decoded, blocks, "Should decode exactly the number of blocks we created");
    
    // Verify the output is not all zeros (simple sanity check)
    let non_zero_count = pcm_slice.iter().filter(|&&sample| sample != 0).count();
    assert!(non_zero_count > 0, "Decoded output should contain non-zero samples");
}

#[test]
fn test_wav_header_format() {
    use std::io::{Cursor, Seek, SeekFrom};
    use byteorder::{LittleEndian, ReadBytesExt};
    
    let mut decoder = Decoder::new();
    
    // Create a valid header and parse it
    let header = create_valid_header(4, 1);
    decoder.read_header(&header).expect("Failed to parse header");
    
    // Create a temp file for the WAV header
    let temp_file_path = "temp_test_wav_header.wav";
    let mut wav_file = File::create(temp_file_path).expect("Failed to create temp file");
    
    // Write the WAV header
    decoder.write_wav_header(&mut wav_file).expect("Failed to write WAV header");
    
    // Close the file and reopen it for reading
    drop(wav_file);
    let mut wav_file = File::open(temp_file_path).expect("Failed to open temp file");
    let mut wav_buffer = Cursor::new(Vec::new());
    wav_file.read_to_end(&mut wav_buffer.get_mut()).expect("Failed to read temp file");
    
    // Clean up temp file
    std::fs::remove_file(temp_file_path).expect("Failed to remove temp file");
    
    // Rewind the buffer to read from the beginning
    wav_buffer.seek(SeekFrom::Start(0)).expect("Failed to rewind buffer");
    
    // Read and verify WAV header
    let mut riff_chunk = [0u8; 4];
    wav_buffer.read_exact(&mut riff_chunk).expect("Failed to read RIFF chunk");
    assert_eq!(&riff_chunk, b"RIFF", "WAV header should start with RIFF");
    
    // Skip chunk size (4 bytes)
    wav_buffer.seek(SeekFrom::Current(4)).expect("Failed to seek");
    
    // Verify WAVE format
    let mut wave_format = [0u8; 4];
    wav_buffer.read_exact(&mut wave_format).expect("Failed to read WAVE format");
    assert_eq!(&wave_format, b"WAVE", "WAV header should have WAVE format");
    
    // Verify fmt chunk
    let mut fmt_chunk = [0u8; 4];
    wav_buffer.read_exact(&mut fmt_chunk).expect("Failed to read fmt chunk");
    assert_eq!(&fmt_chunk, b"fmt ", "WAV header should have fmt chunk");
    
    // Read chunk size
    let chunk_size = wav_buffer.read_u32::<LittleEndian>().expect("Failed to read chunk size");
    assert_eq!(chunk_size, 16, "fmt chunk size should be 16");
    
    // Read audio format
    let audio_format = wav_buffer.read_u16::<LittleEndian>().expect("Failed to read audio format");
    assert_eq!(audio_format, 1, "Audio format should be PCM (1)");
    
    // Read num channels
    let num_channels = wav_buffer.read_u16::<LittleEndian>().expect("Failed to read num channels");
    assert_eq!(num_channels, 1, "Number of channels should be 1");
    
    // Read sample rate
    let sample_rate = wav_buffer.read_u32::<LittleEndian>().expect("Failed to read sample rate");
    assert_eq!(sample_rate, 22050, "Sample rate should be 22050");
    
    // Verify data chunk
    wav_buffer.seek(SeekFrom::Current(6)).expect("Failed to seek to data chunk"); // Skip byte rate and block align
    
    // Read bits per sample
    let bits_per_sample = wav_buffer.read_u16::<LittleEndian>().expect("Failed to read bits per sample");
    assert_eq!(bits_per_sample, 16, "Bits per sample should be 16");
    
    // Read data chunk identifier
    let mut data_chunk = [0u8; 4];
    wav_buffer.read_exact(&mut data_chunk).expect("Failed to read data chunk");
    assert_eq!(&data_chunk, b"data", "WAV header should have data chunk");
} 