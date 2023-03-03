import { expect } from "chai";
import { IdCompressor } from "../IdCompressor";

describe("Compressor creation", () => {
	it("can create and free a compressor", () => {
		const compressor = new IdCompressor();
		compressor.dispose();
	});
});

describe("ID generation", () => {
	it("can generate an ID", () => {
		const compressor = new IdCompressor();
		let id = compressor.generateCompressedId();
		expect(id).to.equal(-1);
		compressor.dispose();
	});
});
