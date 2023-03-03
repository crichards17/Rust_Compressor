import { IdCompressor as WasmIdCompressor } from "wasm-id-allocator";
import {
	FinalCompressedId,
	IdCreationRange,
	IIdCompressor,
	IIdCompressorCore,
	OpSpaceCompressedId,
	SerializedIdCompressor,
	SerializedIdCompressorWithNoSession,
	SerializedIdCompressorWithOngoingSession,
	SessionId,
	SessionSpaceCompressedId,
	StableId,
} from "./types";

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly wasmCompressor: WasmIdCompressor;
	constructor() {
		this.wasmCompressor = new WasmIdCompressor();
	}

	dispose(): void {
		this.wasmCompressor.free();
	}

	get localSessionId(): SessionId {
		throw new Error("Method not implemented.");
	}

	finalizeCreationRange(range: IdCreationRange): void {
		throw new Error("Method not implemented.");
	}

	takeNextCreationRange(): IdCreationRange {
		throw new Error("Method not implemented.");
	}

	generateCompressedId(): SessionSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	normalizeToOpSpace(id: SessionSpaceCompressedId): OpSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId;
	normalizeToSessionSpace(id: FinalCompressedId): SessionSpaceCompressedId;
	normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		sessionIdIfLocal?: SessionId | undefined,
	): SessionSpaceCompressedId;
	normalizeToSessionSpace(
		id: unknown,
		sessionIdIfLocal?: unknown,
	): import("./types").SessionSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	decompress(id: FinalCompressedId | SessionSpaceCompressedId): string | StableId {
		throw new Error("Method not implemented.");
	}

	tryDecompress(id: FinalCompressedId | SessionSpaceCompressedId): string | StableId | undefined {
		throw new Error("Method not implemented.");
	}

	recompress(uncompressed: string): SessionSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	tryRecompress(uncompressed: string): SessionSpaceCompressedId | undefined {
		throw new Error("Method not implemented.");
	}

	serialize(
		withSession: boolean,
	): SerializedIdCompressorWithOngoingSession | SerializedIdCompressorWithNoSession;
	serialize(withSession: true): SerializedIdCompressorWithOngoingSession;
	serialize(withSession: false): SerializedIdCompressorWithNoSession;
	serialize(withSession: boolean): SerializedIdCompressor;
	serialize(
		withSession: unknown,
	):
		| import("./types").SerializedIdCompressorWithOngoingSession
		| import("./types").SerializedIdCompressor
		| import("./types").SerializedIdCompressorWithNoSession {
		throw new Error("Method not implemented.");
	}
}
