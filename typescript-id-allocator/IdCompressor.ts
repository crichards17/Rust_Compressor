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
	UnackedLocalId,
} from "./types";
import { currentWrittenVersion } from "./types/persisted-types/0.0.1";
import { assert, generateStableId } from "./util";
import { getIds } from "./util/idRange";
import { fail } from "./util/utilities";

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly sessionTokens: Map<SessionId, number> = new Map();

	private constructor(
		public readonly wasmCompressor: WasmIdCompressor,
		public readonly localSessionId: SessionId,
	) {}

	public static create(): IdCompressor {
		const localSessionId = generateStableId() as SessionId;
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
		const { first, last, overrides } = ids;
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
			const { first_local, count } = wasmRange.ids;
			range = {
				sessionId: this.localSessionId,
				ids: {
					first: first_local as UnackedLocalId,
					last: (first_local - count + 1) as UnackedLocalId,
				},
			};
		}
		return range;
	}

	public generateCompressedId(): SessionSpaceCompressedId {
		return this.wasmCompressor.generate_next_id() as SessionSpaceCompressedId;
	}

	private idOrError<TId extends CompressedId>(idNum: number): TId {
		if (Object.is(idNum, Number.NaN)) {
			throw new Error(this.wasmCompressor.get_hotpath_error());
		}
		return idNum as TId;
	}

	public normalizeToOpSpace(id: SessionSpaceCompressedId): OpSpaceCompressedId {
		return this.idOrError<OpSpaceCompressedId>(this.wasmCompressor.normalize_to_op_space(id));
	}

	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId;
	public normalizeToSessionSpace(id: FinalCompressedId): SessionSpaceCompressedId;
	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		sessionIdIfLocal?: SessionId,
	): SessionSpaceCompressedId {
		let normalizedId: number;
		if (id < 0) {
			normalizedId = this.wasmCompressor.normalize_local_to_session_space(
				this.getOrCreateSessionToken(sessionIdIfLocal ?? fail("No session ID supplied.")),
				id,
			);
		} else {
			normalizedId = this.wasmCompressor.normalize_final_to_session_space(id);
		}
		return this.idOrError<SessionSpaceCompressedId>(normalizedId);
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
