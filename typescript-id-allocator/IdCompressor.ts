import { IdCompressor as WasmIdCompressor } from "wasm-id-allocator";
import {
	CompressedId,
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
import { currentWrittenVersion } from "./types/persisted-types/0.0.1";
import { assert, generateStableId } from "./util";
import { getIds } from "./util/idRange";
import { createSessionId, fail } from "./util/utilities";

export const defaultClusterCapacity = WasmIdCompressor.get_default_cluster_capacity();

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly sessionTokens: Map<SessionId, number> = new Map();

	private constructor(
		public readonly wasmCompressor: WasmIdCompressor,
		public readonly localSessionId: SessionId,
	) {}

	public static create(): IdCompressor {
		const localSessionId = createSessionId();
		return new IdCompressor(new WasmIdCompressor(localSessionId), localSessionId);
	}

	private getOrCreateSessionToken(sessionId: SessionId): number {
		let token = this.sessionTokens.get(sessionId);
		if (token === undefined) {
			token = this.wasmCompressor.get_token(sessionId);
			this.sessionTokens.set(sessionId, token);
		}
		return token;
	}

	public finalizeCreationRange(range: IdCreationRange): void {
		const ids = getIds(range);
		if (ids === undefined) {
			return;
		}
		const { firstGenCount: first, lastGenCount: last, overrides } = ids;
		assert(overrides === undefined, "Overrides not yet supported.");
		this.wasmCompressor.finalize_range(
			this.getOrCreateSessionToken(range.sessionId),
			first,
			first - last + 1,
		);
	}

	public takeNextCreationRange(): IdCreationRange {
		const wasmRange = this.wasmCompressor.take_next_range();
		let range: IdCreationRange;
		if (wasmRange.ids === undefined) {
			range = { sessionId: this.localSessionId };
		} else {
			const { first_local_gen_count, count } = wasmRange.ids;
			range = {
				sessionId: this.localSessionId,
				ids: {
					firstGenCount: first_local_gen_count,
					lastGenCount: first_local_gen_count + count - 1,
				},
			};
		}
		return range;
	}

	public generateCompressedId(override?: string): SessionSpaceCompressedId {
		return this.wasmCompressor.generate_next_id() as SessionSpaceCompressedId;
	}

	private idOrError<TId extends CompressedId>(idNum: number): TId {
		if (Object.is(idNum, Number.NaN)) {
			throw new Error(this.wasmCompressor.get_error_string());
		}
		return idNum as TId;
	}

	public normalizeToOpSpace(id: SessionSpaceCompressedId): OpSpaceCompressedId {
		return this.idOrError<OpSpaceCompressedId>(this.wasmCompressor.normalize_to_op_space(id));
	}

	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId {
		let session_token = this.getOrCreateSessionToken(originSessionId);
		let normalizedId = this.wasmCompressor.normalize_to_session_space(id, session_token);
		return this.idOrError<SessionSpaceCompressedId>(normalizedId);
	}

	public decompress(id: FinalCompressedId | SessionSpaceCompressedId): string | StableId {
		return this.tryDecompress(id) ?? fail("Could not decompress.");
	}

	public tryDecompress(
		id: FinalCompressedId | SessionSpaceCompressedId,
	): string | StableId | undefined {
		// TODO: log error string to telemetry if undefined
		return this.wasmCompressor.decompress(id);
	}

	public recompress(uncompressed: string): SessionSpaceCompressedId {
		return this.tryRecompress(uncompressed) ?? fail("Could not recompress.");
	}

	public tryRecompress(uncompressed: string): SessionSpaceCompressedId | undefined {
		// TODO: log error string to telemetry if undefined
		return this.wasmCompressor.recompress(uncompressed) as SessionSpaceCompressedId | undefined;
	}

	public serialize(withSession: true): SerializedIdCompressorWithOngoingSession;
	public serialize(withSession: false): SerializedIdCompressorWithNoSession;
	public serialize(withSession: boolean): SerializedIdCompressor {
		return {
			bytes: this.wasmCompressor.serialize(withSession),
			version: currentWrittenVersion,
		} as SerializedIdCompressor;
	}

	public static deserialize(serialized: SerializedIdCompressorWithOngoingSession): IdCompressor;
	public static deserialize(
		serialized: SerializedIdCompressorWithNoSession,
		newSessionId: SessionId,
	): IdCompressor;
	public static deserialize(
		serialized: SerializedIdCompressor,
		sessionId?: SessionId,
	): IdCompressor {
		assert(
			serialized.version === currentWrittenVersion,
			"Unknown serialized compressor version found.",
		);
		const localSessionId = sessionId ?? (generateStableId() as SessionId);
		return new IdCompressor(
			WasmIdCompressor.deserialize(serialized.bytes, localSessionId),
			localSessionId,
		);
	}

	public dispose(): void {
		this.wasmCompressor.free();
	}
}
