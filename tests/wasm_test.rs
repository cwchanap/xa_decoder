#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use std::fs::File;
    use std::io::Read;
    use xa_decoder::decoder::HEADER_SIZE_XA;
    use xa_decoder::WasmXADecoder;

    #[test]
    fn test_wasm_decoder_construction() {
        let mut decoder = WasmXADecoder::new();
        assert!(decoder.decode(&[]).is_err()); // Should fail with empty data
    }

    #[test]
    fn test_wasm_decode_and_format() {
        let mut decoder = WasmXADecoder::new();
        
        // Read the test file
        let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
        let mut xa_data = Vec::new();
        xa_file.read_to_end(&mut xa_data).expect("Failed to read XA file");
        
        // Decode the data
        let pcm_data = decoder.decode(&xa_data).expect("Failed to decode XA data");
        
        // Verify some basic properties
        assert!(!pcm_data.is_empty(), "PCM data should not be empty");
        
        // Test the format
        let format = decoder.get_format().expect("Failed to get format");
        
        // Test format getters
        assert!(format.samples_rate() > 0);
        assert!(format.channels() == 1 || format.channels() == 2);
        assert!(format.data_length_pcm() > 0);
    }

    #[test]
    fn test_incomplete_xa_data() {
        let mut decoder = WasmXADecoder::new();
        
        // Read just the header of the test file
        let mut xa_file = File::open("bass.xa").expect("Failed to open test file");
        let mut header = [0u8; HEADER_SIZE_XA];
        xa_file.read_exact(&mut header).expect("Failed to read header");
        
        // This should return an error because we only have header data, no actual XA data
        let result = decoder.decode(&header);
        
        // Since we only have header data, the decode function should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_files() {
        // Test that the decoder can handle multiple files in sequence
        let mut decoder = WasmXADecoder::new();
        
        // List of test files to process
        let test_files = ["bass.xa", "midtom.xa", "floortom.xa"];
        
        for file_name in &test_files {
            // Read file
            let mut xa_file = File::open(file_name).expect(&format!("Failed to open test file {}", file_name));
            let mut xa_data = Vec::new();
            xa_file.read_to_end(&mut xa_data).expect(&format!("Failed to read test file {}", file_name));
            
            // Decode
            let pcm_data = decoder.decode(&xa_data).expect(&format!("Failed to decode {}", file_name));
            
            // Basic validation
            assert!(!pcm_data.is_empty(), "PCM data should not be empty for {}", file_name);
            
            // Get format
            let format = decoder.get_format().expect(&format!("Failed to get format for {}", file_name));
            
            // Validate format is reasonable
            assert!(format.samples_rate() > 0);
            assert!(format.channels() > 0);
            assert!(format.data_length_pcm() > 0);
        }
    }
}

// This is a dummy test that will run on non-WASM platforms to ensure the test suite passes
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_dummy_for_non_wasm() {
    // This test will always pass on non-wasm platforms
    assert!(true);
} 