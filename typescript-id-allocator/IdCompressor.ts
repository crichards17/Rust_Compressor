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
import { generateStableId } from "./util";

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly wasmCompressor: WasmIdCompressor;
	public readonly localSessionId: SessionId;
	constructor() {
		this.localSessionId = generateStableId() as SessionId;
		this.wasmCompressor = new WasmIdCompressor(this.localSessionId);
	}

	public dispose(): void {
		this.wasmCompressor.free();
	}

	public finalizeCreationRange(range: IdCreationRange): void {
		throw new Error("Method not implemented.");
	}

	public takeNextCreationRange(): IdCreationRange {
		throw new Error("Method not implemented.");
	}

	public generateCompressedId(): SessionSpaceCompressedId {
		return this.wasmCompressor.generate_next_id() as SessionSpaceCompressedId;
	}

	public normalizeToOpSpace(id: SessionSpaceCompressedId): OpSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId;
	public normalizeToSessionSpace(id: FinalCompressedId): SessionSpaceCompressedId;
	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		sessionIdIfLocal?: SessionId | undefined,
	): SessionSpaceCompressedId;
	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		sessionIdIfLocal?: SessionId,
	): SessionSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	public decompress(id: FinalCompressedId | SessionSpaceCompressedId): string | StableId {
		throw new Error("Method not implemented.");
	}

	public tryDecompress(
		id: FinalCompressedId | SessionSpaceCompressedId,
	): string | StableId | undefined {
		throw new Error("Method not implemented.");
	}

	public recompress(uncompressed: string): SessionSpaceCompressedId {
		throw new Error("Method not implemented.");
	}

	public tryRecompress(uncompressed: string): SessionSpaceCompressedId | undefined {
		throw new Error("Method not implemented.");
	}

	public serialize(
		withSession: boolean,
	): SerializedIdCompressorWithOngoingSession | SerializedIdCompressorWithNoSession;
	public serialize(withSession: true): SerializedIdCompressorWithOngoingSession;
	public serialize(withSession: false): SerializedIdCompressorWithNoSession;
	public serialize(withSession: boolean): SerializedIdCompressor {
		throw new Error("Method not implemented.");
	}
}
