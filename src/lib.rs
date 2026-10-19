pub mod decoder;

use std::panic;

use decoder::{Decoder, Format, HEADER_SIZE_XA};
use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Debug version of console_log macro (browser only; on other targets the
// wasm-bindgen import would panic when called)
#[cfg(all(debug_assertions, target_arch = "wasm32"))]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

// Release / non-wasm version of console_log macro (does nothing)
#[cfg(not(all(debug_assertions, target_arch = "wasm32")))]
macro_rules! console_log {
    ($($t:tt)*) => {};
}

#[wasm_bindgen(start)]
pub fn set_panic_hook() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub struct WasmXADecoder(Decoder);

#[wasm_bindgen]
pub struct WasmXAFormat(Format);

#[wasm_bindgen]
impl WasmXAFormat {
    #[wasm_bindgen(getter)]
    pub fn samples_rate(&self) -> u32 {
        self.0.samples_rate as u32
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> u8 {
        self.0.channels as u8
    }

    #[wasm_bindgen(getter)]
    pub fn data_length_pcm(&self) -> u32 {
        self.0.data_length_pcm
    }
}

impl Default for WasmXADecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmXADecoder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        WasmXADecoder(Decoder::new())
    }

    #[wasm_bindgen]
    pub fn decode(&mut self, src: &[u8]) -> Result<Vec<i16>, JsValue> {
        console_log!("Decoding started. Source length: {}", src.len());

        match self.internal_decode(src) {
            Ok(pcm) => {
                console_log!("Decoding successful. PCM length: {}", pcm.len());
                Ok(pcm)
            }
            Err(e) => {
                console_log!("Error during decoding: {:?}", e);
                Err(JsValue::from_str(&e))
            }
        }
    }

    pub fn get_format(&mut self) -> Result<WasmXAFormat, JsValue> {
        let format = self.0.fmt.as_ref().unwrap();
        Ok(WasmXAFormat(format.clone()))
    }

    fn internal_decode(&mut self, src: &[u8]) -> Result<Vec<i16>, String> {
        let fmt = self.0.read_header(src).map_err(|e| e.to_string())?;

        console_log!(
            "Header read successfully. PCM data length: {}",
            fmt.data_length_pcm
        );

        let xa_data_size = fmt.blocks as usize * fmt.block_size_xa as usize;
        let xa_data = src
            .get(HEADER_SIZE_XA..HEADER_SIZE_XA + xa_data_size)
            .ok_or_else(|| {
                format!(
                    "XA data truncated: expected {} bytes after header, got {}",
                    xa_data_size,
                    src.len().saturating_sub(HEADER_SIZE_XA)
                )
            })?;

        let pcm_data_size = fmt.data_length_pcm as usize / 2; // 16-bit samples
        let mut pcm_data = vec![0i16; pcm_data_size];

        console_log!("Src len: {}", xa_data.len());
        console_log!("Dst len: {}", pcm_data.len());

        self.0
            .decode(xa_data, &mut pcm_data)
            .map_err(|e| format!("Error during decoding: {}", e))?;

        Ok(pcm_data)
    }
}

// Add this function at the end of the file
#[wasm_bindgen]
pub fn handle_panic(error: JsValue) {
    console::error_1(&error);
}

// Real wasm32 tests: JsValue errors can only be constructed on wasm, so these
// run under `wasm-pack test --node`.
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use crate::decoder::HEADER_SIZE_XA;
    use crate::WasmXADecoder;
    use wasm_bindgen_test::wasm_bindgen_test;

    static BASS_XA: &[u8] = include_bytes!("../bass.xa");

    #[wasm_bindgen_test]
    fn decode_bass_xa_and_read_format() {
        let mut decoder = WasmXADecoder::new();
        let pcm_data = decoder.decode(BASS_XA).expect("Failed to decode XA data");
        assert!(!pcm_data.is_empty(), "PCM data should not be empty");

        let format = decoder.get_format().expect("Failed to get format");
        assert!(format.samples_rate() > 0);
        assert!(format.channels() == 1 || format.channels() == 2);
        assert!(format.data_length_pcm() > 0);
    }

    #[wasm_bindgen_test]
    fn empty_input_is_error() {
        let mut decoder = WasmXADecoder::new();
        assert!(decoder.decode(&[]).is_err());
    }

    #[wasm_bindgen_test]
    fn header_only_input_is_error() {
        let mut decoder = WasmXADecoder::new();
        // Header only: no XA payload follows, so decode must return Err, not panic.
        assert!(decoder.decode(&BASS_XA[..HEADER_SIZE_XA]).is_err());
    }

    #[wasm_bindgen_test]
    fn truncated_input_is_error() {
        let mut decoder = WasmXADecoder::new();
        // One byte short of the full payload.
        assert!(decoder.decode(&BASS_XA[..BASS_XA.len() - 1]).is_err());
    }
}
