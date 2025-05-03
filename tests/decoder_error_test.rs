use std::io::Read;
use std::fs::File;
use std::panic;
use xa_decoder::decoder::{Decoder, HEADER_SIZE_XA};

// Helper function to create an invalid header with proper magic but invalid internal values
fn create_invalid_header() -> [u8; HEADER_SIZE_XA] {
    let mut header = [0u8; HEADER_SIZE_XA];
    
    // Set valid magic number
    header[0] = 0x4B; // K
    header[1] = 0x57; // W
    header[2] = 0x44; // D
    header[3] = 0x31; // 1
    
    // Set invalid values that should fail validation
    // Zero data length
    header[4] = 0;
    header[5] = 0;
    header[6] = 0;
    header[7] = 0;
    
    // Some samples count
    header[8] = 0x10;
    header[9] = 0;
    header[10] = 0;
    header[11] = 0;
    
    // Valid sample rate
    header[12] = 0x44;
    header[13] = 0xAC; // 44100
    
    // Valid bit depth
    header[14] = 4;
    
    // Valid channels
    header[15] = 1;
    
    header
}

#[test]
fn test_decode_with_no_state() {
    let mut decoder = Decoder::new();
    let sample_data = vec![0u8; 100];
    let mut pcm_data = vec![0i16; 100];
    
    // Attempting to decode without initializing the decoder state
    let result = decoder.decode(&sample_data, &mut pcm_data);
    
    // Should fail because the decoder is not initialized
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.contains("not initialized"), "Error should mention 'not initialized' but was: {}", e);
    }
}

#[test]
fn test_invalid_header_verification() {
    let mut decoder = Decoder::new();
    let header = create_invalid_header();
    
    // Should fail due to validation in is_valid() 
    let result = decoder.read_header(&header);
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid XA header data"), 
                "Error should mention 'Invalid XA header data' but was: {}", e);
    }
}

#[test]
fn test_short_header() {
    let mut decoder = Decoder::new();
    let short_header = [0u8; 16]; // Too short for a header
    
    // Should fail because the header is too short
    let result = decoder.read_header(&short_header);
    
    assert!(result.is_err());
}

#[test]
fn test_invalid_decode_buffer_size() {
    let mut decoder = Decoder::new();
    
    // Create a valid header from the helper functions in decoder_misc_test
    let mut xa_file = std::fs::File::open("bass.xa").expect("Failed to open test file");
    let mut header = [0u8; HEADER_SIZE_XA];
    xa_file.read_exact(&mut header).expect("Failed to read header");
    
    decoder.read_header(&header).expect("Failed to parse header");
    
    // Create a buffer that's too small
    let src_data = vec![0u8; 100];
    let mut pcm_data = vec![0i16; 10]; // Too small for even one block
    
    // Should return 0 blocks decoded but not error
    let result = decoder.decode(&src_data, &mut pcm_data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_wav_header_no_format() {
    // The current implementation of write_wav_header will panic when fmt is None
    // So we need to test that it panics when expected
    let decoder = Decoder::new(); // Uninitialized with fmt=None
    
    // Create a temp file
    let temp_file_path = "test_no_format.wav";
    let mut temp_file = File::create(temp_file_path).expect("Failed to create temp file");
    
    // Use catch_unwind to catch the expected panic
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        // This should panic since decoder.fmt is None
        decoder.write_wav_header(&mut temp_file).unwrap();
    }));
    
    // Clean up the file
    drop(temp_file);
    let _ = std::fs::remove_file(temp_file_path);
    
    // Verify it panicked as expected
    assert!(result.is_err(), "Function should have panicked with None fmt but didn't");
} 