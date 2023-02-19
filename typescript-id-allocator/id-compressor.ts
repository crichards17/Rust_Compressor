import { add } from "wasm-id-allocator";

export function addTs(left: number, right: number): number {
    return add(left, right);
};