// Happy-path test for the WasmXADecoder wrapper, runnable with plain `cargo
// test`. Error-path and full wasm32 coverage lives in src/lib.rs (`wasm_tests`
// module) and runs under `wasm-pack test --node`.
use xa_decoder::WasmXADecoder;

static BASS_XA: &[u8] = include_bytes!("../bass.xa");

#[test]
fn decode_bass_xa_and_read_format() {
    let mut decoder = WasmXADecoder::new();
    let pcm_data = decoder.decode(BASS_XA).expect("Failed to decode XA data");
    assert!(!pcm_data.is_empty(), "PCM data should not be empty");

    let format = decoder.get_format().expect("Failed to get format");
    assert!(format.samples_rate() > 0);
    assert!(format.channels() == 1 || format.channels() == 2);
    assert!(format.data_length_pcm() > 0);
}
