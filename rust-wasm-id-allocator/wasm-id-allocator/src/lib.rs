use distributed_id_allocator::{
    compressor::IdCompressor as IdCompressorCore,
    id_types::{LocalId, SessionSpaceId},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct IdCompressor {
    compressor: IdCompressorCore,
    error_string: Option<String>,
}
/*
TODO:
- doc which methods may populate error field
*/

#[wasm_bindgen]
impl IdCompressor {
    pub fn is_error(&self) -> bool {
        self.error_string.is_some()
    }

    pub fn get_error(&mut self) -> String {
        let error = match &self.error_string {
            None => String::from(""),
            Some(e) => (*e).clone(),
        };
        self.error_string = None;
        error
    }

    fn set_error(&mut self, error: &str) {
        self.error_string = Some(String::from(error));
    }

    pub fn generate_next_id(&mut self) -> f64 {
        let next_id = self.compressor.generate_next_id().id();
        debug_assert!(next_id <= 2 ^ 53 - 1 && next_id >= -2 ^ 53);
        next_id as f64
    }

    pub fn decompress(&mut self, id_to_decompress: f64) -> String {
        if id_to_decompress.fract() != 0.0 {
            self.set_error("Non-integer ID passed to decompress.");
            return String::from("");
        }
        let session_space_id = SessionSpaceId::from_id(id_to_decompress as i64);
        match session_space_id.decompress(&self.compressor) {
            Ok(stable_id) => stable_id.to_uuid_string(),
            Err(e) => {
                self.set_error(e.get_error_string());
                String::from("")
            }
        }
    }
}

#[wasm_bindgen]
pub fn make_compressor() -> IdCompressor {
    IdCompressor {
        compressor: IdCompressorCore::new(),
        error_string: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i64_as_f64() {
        let num: i64 = -5;
        dbg!(num as f64);
    }
}
