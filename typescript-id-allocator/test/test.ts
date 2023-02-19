import { addTs } from "../id-compressor";
import { expect } from "chai";

describe('Add function', () => {
    it('should add numbers correctly', () => {
        const result = addTs(1, 2);
        expect(result).to.equal(3);
    });
});