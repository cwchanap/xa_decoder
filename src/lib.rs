mod decoder;

use std::panic;

use decoder::{Decoder, Format, HEADER_SIZE_XA};
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
        let mut xa_data = vec![0u8; xa_data_size];
        xa_data.copy_from_slice(&src[HEADER_SIZE_XA..]);

        let pcm_data_size = fmt.data_length_pcm as usize / 2; // 16-bit samples
        let mut pcm_data = vec![0i16; pcm_data_size];

        console_log!("Src len: {}", xa_data.len());
        console_log!("Dst len: {}", pcm_data.len());

        let decoded_block = self.0
            .decode(&xa_data, &mut pcm_data)
            .map_err(|e| format!("Error during decoding: {}", e))?;

        console_log!("Decoded {} blocks", decoded_block);

        Ok(pcm_data)
    }
}

// Add this function at the end of the file
#[wasm_bindgen]
pub fn handle_panic(error: JsValue) {
    console::error_1(&error);
}
