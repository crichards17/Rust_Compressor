import { expect } from "chai";
import { IdCompressor } from "../IdCompressor";

describe("Compressor creation", () => {
	it("can create and free a compressor", () => {
		const compressor = IdCompressor.create();
		compressor.dispose();
	});
});

describe("ID generation", () => {
	it("can generate an ID", () => {
		const compressor = IdCompressor.create();
		let id = compressor.generateCompressedId();
		expect(id).to.equal(-1);
		compressor.dispose();
	});
});
