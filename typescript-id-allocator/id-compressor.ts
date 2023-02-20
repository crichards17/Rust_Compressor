import { add, IdCompressorWrapper } from "wasm-id-allocator";

export function makeCompressor(): IdCompressorWrapper {
    return add();
};