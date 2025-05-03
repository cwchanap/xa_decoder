use std::fs::File;
use std::io::Read;
use xa_decoder::decoder::{Decoder, HEADER_SIZE_XA};

#[test]
fn test_decoder_initialization() {
    let decoder = Decoder::new();
    assert!(decoder.fmt.is_none());
    // Cannot directly test decoder.state as DecoderState is private
}

#[test]
fn test_read_header() {
    let mut decoder = Decoder::new();
    
    // Test with a valid header from bass.xa file
    let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    
    let format = decoder.read_header(&header).expect("Failed to parse header");
    
    // Verify format properties match expected values for bass.xa
    assert_eq!(format.channels, 1);
    assert_eq!(format.sample_bits, 16);
    assert!(format.samples_rate > 0);
    assert!(format.blocks > 0);
    assert!(format.data_length_pcm > 0);
    assert!(format.block_size_xa > 0);
    assert!(format.block_size_pcm > 0);
}

#[test]
fn test_invalid_header_magic() {
    let mut decoder = Decoder::new();
    
    // Create an invalid header with wrong magic number
    let mut invalid_header = [0u8; HEADER_SIZE_XA];
    // The first 4 bytes are the magic number, set them to something invalid
    invalid_header[0..4].copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
    
    let result = decoder.read_header(&invalid_header);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid XA header magic"));
    }
}

#[test]
fn test_invalid_bit_depth() {
    let mut decoder = Decoder::new();
    
    // Test with a valid header from bass.xa file but modify bit depth
    let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    
    // Modify bit depth to invalid value (should be 4, 6, or 8)
    header[14] = 5;
    
    let result = decoder.read_header(&header);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid bit depth"));
    }
}

#[test]
fn test_invalid_channels() {
    let mut decoder = Decoder::new();
    
    // Test with a valid header from bass.xa file but modify channels
    let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    
    // Modify channels to invalid value (should be 1 or 2)
    header[15] = 3;
    
    let result = decoder.read_header(&header);
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid number of channels"));
    }
}

#[test]
fn test_full_decoding() {
    let mut decoder = Decoder::new();
    
    // Open test file
    let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
    
    // Read and validate header
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    let format = decoder.read_header(&header).expect("Failed to parse header");
    
    // Read XA data
    let xa_data_size = format.blocks as usize * format.block_size_xa as usize;
    let mut xa_data = vec![0u8; xa_data_size];
    xa_file.read_exact(&mut xa_data).expect("Failed to read XA data");
    
    // Prepare PCM buffer
    let pcm_data_size = format.data_length_pcm as usize / 2; // 16-bit samples
    let mut pcm_data = vec![0i16; pcm_data_size];
    
    // Decode the data
    let blocks_decoded = decoder.decode(&xa_data, &mut pcm_data).expect("Failed to decode XA data");
    
    // Validate results
    assert_eq!(blocks_decoded, format.blocks);
    
    // Verify the output is not all zeros (simple sanity check)
    let non_zero_count = pcm_data.iter().filter(|&&sample| sample != 0).count();
    assert!(non_zero_count > 0, "Decoded output contains no non-zero samples");
}

#[test]
fn test_write_wav_header() {
    use std::io::{Seek, SeekFrom};
    use std::fs::remove_file;
    
    let test_wav_path = "test_output.wav";
    let mut decoder = Decoder::new();
    
    // Read header from test file
    let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    decoder.read_header(&header).expect("Failed to parse header");
    
    // Create a test WAV file
    let mut wav_file = File::create(test_wav_path).expect("Failed to create test WAV file");
    decoder.write_wav_header(&mut wav_file).expect("Failed to write WAV header");
    
    // Verify the file was created and has the correct header
    let mut wav_file = File::open(test_wav_path).expect("Failed to open test WAV file");
    let mut header_buf = [0u8; 4];
    wav_file.read_exact(&mut header_buf).expect("Failed to read WAV header");
    
    // Check RIFF magic
    assert_eq!(&header_buf, b"RIFF");
    
    // Skip 4 bytes (file size)
    wav_file.seek(SeekFrom::Current(4)).expect("Failed to seek in WAV file");
    
    // Check WAVE magic
    wav_file.read_exact(&mut header_buf).expect("Failed to read WAV header");
    assert_eq!(&header_buf, b"WAVE");
    
    // Clean up test file
    drop(wav_file);
    if let Err(e) = remove_file(test_wav_path) {
        println!("Warning: Failed to remove test file: {}", e);
    }
} 