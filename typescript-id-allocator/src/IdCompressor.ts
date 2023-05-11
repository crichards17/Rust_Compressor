import {
	IdCompressor as WasmIdCompressor,
	InteropIds,
	InteropTelemetryStats,
} from "wasm-id-allocator";
import { ITelemetryLogger } from "@fluidframework/common-definitions";
import { assert, fail } from "./copied-utils";
import {
	currentWrittenVersion,
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
import { createSessionId, isNaN, uuidStringFromBytes } from "./utilities";

export const defaultClusterCapacity = WasmIdCompressor.get_default_cluster_capacity();
const nilToken = WasmIdCompressor.get_nil_token();
const tokenCacheMaxSize = 300;
const tokenCacheTarget = 10;

/**
 * See {@link IIdCompressor} and {@link IIdCompressorCore}
 */
export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly sessionTokens: Map<SessionId, number> = new Map();
	public readonly localSessionId: SessionId;
	private lastUsedToken = nilToken;
	private lastUsedSessionId: SessionId | undefined = undefined;

	private constructor(
		private readonly wasmCompressor: WasmIdCompressor,
		private readonly logger?: ITelemetryLogger,
	) {
		const sessionBytes = this.wasmCompressor.get_local_session_id();
		this.localSessionId = uuidStringFromBytes(sessionBytes) as SessionId;
	}

	public static create(logger?: ITelemetryLogger): IdCompressor;
	public static create(sessionId: SessionId, logger?: ITelemetryLogger): IdCompressor;
	public static create(
		sessionIdOrLogger?: SessionId | ITelemetryLogger,
		loggerOrUndefined?: ITelemetryLogger,
	): IdCompressor {
		let localSessionId: SessionId;
		let logger: ITelemetryLogger | undefined;
		if (sessionIdOrLogger === undefined) {
			localSessionId = createSessionId();
		} else {
			if (typeof sessionIdOrLogger === "string") {
				localSessionId = sessionIdOrLogger;
				logger = loggerOrUndefined;
			} else {
				localSessionId = createSessionId();
				logger = loggerOrUndefined;
			}
		}
		const compressor = new IdCompressor(new WasmIdCompressor(localSessionId), logger);
		return compressor;
	}

	/**
	 * The size of each newly created ID cluster.
	 */
	public get clusterCapacity(): number {
		return this.wasmCompressor.get_cluster_capacity();
	}

	/**
	 * Must only be set with a value upon which consensus has been reached. Value must be greater than zero and less than
	 * `IdCompressor.maxClusterSize`.
	 */
	public set clusterCapacity(value: number) {
		this.wasmCompressor.set_cluster_capacity(value);
	}

	public finalizeCreationRange(range: IdCreationRange): void {
		const { sessionId, ids } = range;
		if (this.sessionTokens.get(sessionId) === nilToken) {
			this.sessionTokens.delete(sessionId);
			if (sessionId === this.lastUsedSessionId) {
				this.lastUsedSessionId = undefined;
				this.lastUsedToken = nilToken;
			}
		}
		if (ids !== undefined) {
			let idStats: InteropTelemetryStats | undefined;
			try {
				idStats = this.wasmCompressor.finalize_range(
					sessionId,
					ids.firstGenCount,
					ids.count,
				);

				// Log telemetry
				if (
					idStats !== undefined &&
					sessionId === this.localSessionId &&
					this.logger !== undefined
				) {
					const {
						eager_final_count,
						cluster_creation_count,
						expansion_count,
						local_id_count,
					} = idStats;
					this.logger.sendTelemetryEvent({
						eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
						eagerFinalIdCount: eager_final_count,
						localIdCount: local_id_count,
						rangeSize: ids.count,
						clusterCapacity: this.wasmCompressor.get_cluster_capacity(),
						clusterChange:
							cluster_creation_count > 0
								? "Creation"
								: expansion_count > 0
								? "Expansion"
								: "None",
						sessionId: this.localSessionId,
					});
				}
			} finally {
				idStats?.free();
			}
		}
	}

	public takeNextCreationRange(): IdCreationRange {
		let wasmRange: InteropIds | undefined;
		try {
			wasmRange = this.wasmCompressor.take_next_range();
			let range: IdCreationRange;
			if (wasmRange === undefined) {
				range = { sessionId: this.localSessionId };
			} else {
				const { first_local_gen_count, count } = wasmRange;
				range = {
					sessionId: this.localSessionId,
					ids: {
						firstGenCount: first_local_gen_count,
						count,
					},
				};
			}
			return range;
		} finally {
			wasmRange?.free();
		}
	}

	public generateCompressedId(): SessionSpaceCompressedId {
		return this.wasmCompressor.generate_next_id() as SessionSpaceCompressedId;
	}

	private idOrError<TId extends number>(idNum: number): TId {
		if (isNaN(idNum)) {
			throw new TypeError("Invalid ID to normalize.");
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
		let sessionToken = this.lastUsedToken;
		if (originSessionId !== this.lastUsedSessionId) {
			let wasmSessionToken = this.sessionTokens.get(originSessionId);
			if (wasmSessionToken === undefined) {
				wasmSessionToken = this.wasmCompressor.get_token(originSessionId);
				this.sessionTokens.set(originSessionId, wasmSessionToken);
				if (this.sessionTokens.size > tokenCacheMaxSize) {
					for (const key of this.sessionTokens.keys()) {
						if (this.sessionTokens.size > tokenCacheTarget) {
							this.sessionTokens.delete(key);
						} else {
							break;
						}
					}
				}
			}
			sessionToken = wasmSessionToken;
			this.lastUsedToken = sessionToken;
			this.lastUsedSessionId = originSessionId;
			if (wasmSessionToken === nilToken) {
				assert(id >= 0, "No IDs have ever been finalized by the supplied session.");
			}
		}
		const normalizedId = this.wasmCompressor.normalize_to_session_space(id, sessionToken);
		return this.idOrError<SessionSpaceCompressedId>(normalizedId);
	}

	public decompress(id: SessionSpaceCompressedId): StableId {
		return (
			this.tryDecompress(id) ?? fail("Compressed ID was not generated by this compressor.")
		);
	}

	public tryDecompress(id: SessionSpaceCompressedId): StableId | undefined {
		const uuidBytes = this.wasmCompressor.decompress(id);
		return uuidStringFromBytes(uuidBytes) as StableId;
	}

	public recompress(uncompressed: StableId): SessionSpaceCompressedId {
		return this.tryRecompress(uncompressed) ?? fail("Could not recompress.");
	}

	public tryRecompress(uncompressed: StableId): SessionSpaceCompressedId | undefined {
		return this.wasmCompressor.recompress(uncompressed) as SessionSpaceCompressedId | undefined;
	}

	public dispose(): void {
		this.wasmCompressor.free();
	}

	public serialize(withSession: true): SerializedIdCompressorWithOngoingSession;
	public serialize(withSession: false): SerializedIdCompressorWithNoSession;
	public serialize(withSession: boolean): SerializedIdCompressor {
		const bytes = this.wasmCompressor.serialize(withSession);
		this.logger?.sendTelemetryEvent({
			eventName: "RuntimeIdCompressor:SerializedIdCompressorSize",
			size: bytes.length,
		});
		// Use branding utility
		// eslint-disable-next-line @typescript-eslint/consistent-type-assertions
		return {
			bytes,
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
		const localSessionId = sessionId ?? createSessionId();
		return new IdCompressor(WasmIdCompressor.deserialize(serialized.bytes, localSessionId));
	}
}
