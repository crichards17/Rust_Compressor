import { IdCompressor } from "wasm-id-allocator";

/* 
TODO: 
- Write wrapper IdCompressor class exposing the same interface as existing (FF) ID Compressor
*/

export function makeCompressor(): IdCompressor {
    return new IdCompressor();
};