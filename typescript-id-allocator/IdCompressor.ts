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
import { assert, generateStableId } from "./util";
import { getIds } from "./util/idRange";

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly wasmCompressor: WasmIdCompressor;
	public readonly localSessionId: SessionId;
	private readonly localSessionToken: number;
	private readonly sessionTokens: Map<SessionId, number> = new Map();
	constructor() {
		this.localSessionId = generateStableId() as SessionId;
		this.wasmCompressor = new WasmIdCompressor(this.localSessionId);
		this.localSessionToken = this.getOrCreateSessionToken(this.localSessionId);
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
		if (wasmRange.count === 0) {
			range = { sessionId: this.localSessionId };
		} else {
			const { first_local, count } = wasmRange;
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
		if (idNum === Number.NaN) {
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
		sessionIdIfLocal?: SessionId | undefined,
	): SessionSpaceCompressedId;
	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		sessionIdIfLocal?: SessionId,
	): SessionSpaceCompressedId {
		const originatorToken =
			sessionIdIfLocal === undefined
				? this.localSessionToken
				: this.getOrCreateSessionToken(sessionIdIfLocal);
		return this.idOrError<SessionSpaceCompressedId>(
			this.wasmCompressor.normalize_to_session_space(originatorToken, id),
		);
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

	public static deserialize(serialized: SerializedIdCompressorWithOngoingSession): IdCompressor;
	public static deserialize(
		serialized: SerializedIdCompressorWithNoSession,
		newSessionId: SessionId,
	): IdCompressor;
	public static deserialize(
		...args:
			| [serialized: SerializedIdCompressorWithNoSession, newSessionIdMaybe: SessionId]
			| [serialized: SerializedIdCompressorWithOngoingSession, newSessionIdMaybe?: undefined]
	): IdCompressor {
		throw new Error("Method not implemented.");
	}

	public dispose(): void {
		this.wasmCompressor.free();
	}
}
