const path = require("node:path");
const fs = require("fs");

function readStatsOrError(fileName) {
	try {
		return fs.statSync(fileName);
	} catch {
		throw new Error(`File ${fileName} not found`);
	}
}

const stats = readStatsOrError("./dist/wasm/wasm_id_allocator_bg.wasm");
if (stats.size > 37000) {
	throw Error("WASM binary size unexpectedly increased.");
}
