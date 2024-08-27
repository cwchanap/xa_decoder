mod decoder;

use std::panic;

use decoder::{Decoder, Format};
use wasm_bindgen::prelude::*;
use web_sys::console;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format!($($t)*)))
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

#[wasm_bindgen]
impl WasmXADecoder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        WasmXADecoder(Decoder::new())
    }

    #[wasm_bindgen]
    pub fn read_header(&mut self, src: &[u8]) -> Result<WasmXAFormat, JsValue> {
        let fmt = self
            .0
            .read_header(src)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WasmXAFormat(fmt))
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

    fn internal_decode(&mut self, src: &[u8]) -> Result<Vec<i16>, String> {
        let fmt = self
            .0
            .read_header(src)
            .map_err(|e| format!("Error reading header: {}", e))?;

        console_log!(
            "Header read successfully. PCM data length: {}",
            fmt.data_length_pcm
        );

        let mut pcm = vec![0i16; fmt.data_length_pcm as usize / 2];

        self.0
            .decode(src, &mut pcm)
            .map_err(|e| format!("Error during decoding: {}", e))?;

        Ok(pcm)
    }
}

// Add this function at the end of the file
#[wasm_bindgen]
pub fn handle_panic(error: JsValue) {
    console::error_1(&error);
}
