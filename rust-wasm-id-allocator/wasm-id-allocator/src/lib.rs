use distributed_id_allocator::compressor::IdCompressor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct IdCompressorWrapper {
    compressor: IdCompressor,
}

#[wasm_bindgen]
pub fn add() -> IdCompressorWrapper {
    IdCompressorWrapper {
        compressor: IdCompressor::new(),
    }
}
