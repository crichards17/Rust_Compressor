require("wasm-tracing-allocator");
import { IdCompressor } from "../src/IdCompressor";
import { SessionId } from "../src/types";
import { createSessionId } from "../src/utilities";

describe.only("IdCompressor memory", () => {
	it("Trace allocations", async () => {
		const numSessions = 10000;
		const capacity = 10;
		const numClusters = 3;
		const compressor = IdCompressor.create();
		compressor.clusterCapacity = capacity;
		const sessions: SessionId[] = [];
		for (let i = 0; i < numSessions; i++) {
			sessions.push(createSessionId());
		}
		for (let i = 0; i < numSessions * numClusters; i++) {
			const sessionId = sessions[i % numSessions];
			if (Math.random() > 0.1) {
				for (let j = 0; j < Math.round(capacity / 2); j++) {
					compressor.generateCompressedId();
				}
				compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			}
			compressor.finalizeCreationRange({
				sessionId,
				ids: {
					firstGenCount: Math.floor(i / numSessions) * capacity + 1,
					count: capacity,
				},
			});
		}
		dumpAllocations(true);
		dumpAllocations(false);
		console.log(
			"Total WASM heap size: " + require("wasm-id-allocator").__wasm.memory.buffer.byteLength,
		);
		compressor.dispose();
	});
});

function dumpAllocations(showCount: boolean): void {
	(global as any).WasmTracingAllocator.dumpLiveAllocations({
		keyLabel: "Live Allocations",
		valueLabel: showCount ? "Count" : "Bytes",
		getKey: (entry) =>
			entry.stack
				.split(/[\n\s]/)
				.filter(
					(s) =>
						s.indexOf("distributed_id_allocator::") >= 0 ||
						s.indexOf("wasm_id_allocator::") >= 0 ||
						s.indexOf("vec::") >= 0 ||
						s.toLowerCase().indexOf("btree") >= 0,
				)
				.map((str) => {
					return str
						.replace(`distributed_id_allocator::`, "")
						.replace(`wasm_id_allocator::`, "");
				})
				.slice(-5)
				.join("   "),
		getValue: showCount ? (_entry) => 1 : (_entry) => _entry.size,
	});
}
