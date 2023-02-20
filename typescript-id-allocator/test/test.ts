import { expect } from "chai";
import { makeCompressor } from "../id-compressor";

describe('Compressor creation', () => {
    it('can create and free a compressor', () => {
        const compressor = makeCompressor();
        compressor.free();
    });
});

describe('ID generation', () => {
    it('can generate an ID', () => {
        const compressor = makeCompressor();
        let id = compressor.generate_next_id();
        expect(id).to.equal(-1);
        compressor.free();
    });
});