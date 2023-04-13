import { IdCompressor as WasmIdCompressor, InteropIds, InteropIdStats } from "wasm-id-allocator";
import { assert } from "./copied-utils";
import {
	CompressedId,
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
import { createSessionId, fail, isNaN } from "./util/utilities";
import { ITelemetryLogger } from "@fluidframework/common-definitions";

export const defaultClusterCapacity = WasmIdCompressor.get_default_cluster_capacity();

export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private readonly sessionTokens: Map<SessionId, number> = new Map();
	public readonly localSessionId: SessionId;

	private constructor(
		private readonly wasmCompressor: WasmIdCompressor,
		private readonly logger?: ITelemetryLogger,
	) {
		this.localSessionId = wasmCompressor.get_local_session_id() as SessionId;
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
		if (isNaN(this.sessionTokens.get(sessionId))) {
			this.sessionTokens.delete(sessionId);
		}
		if (ids !== undefined) {
			let idStats: InteropIdStats | undefined;
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

	private idOrError<TId extends CompressedId>(idNum: number): TId {
		if (isNaN(idNum)) {
			throw new Error(this.wasmCompressor.get_normalization_error_string());
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
		let session_token = this.sessionTokens.get(originSessionId);
		if (session_token === undefined) {
			session_token = this.wasmCompressor.get_token(originSessionId);
			this.sessionTokens.set(originSessionId, session_token);
		}
		assert(
			!isNaN(session_token) || id >= 0,
			"No IDs have ever been finalized by the supplied session.",
		);
		let normalizedId = this.wasmCompressor.normalize_to_session_space(id, session_token);
		return this.idOrError<SessionSpaceCompressedId>(normalizedId);
	}

	public decompress(id: SessionSpaceCompressedId): StableId {
		return (
			this.tryDecompress(id) ?? fail("Compressed ID was not generated by this compressor.")
		);
	}

	public tryDecompress(id: SessionSpaceCompressedId): StableId | undefined {
		const uuidBytes = this.wasmCompressor.decompress(id);
		if (uuidBytes === undefined) {
			return undefined;
		}
		let uuidString = "";
		for (let i = 0; i < 36; i++) {
			uuidString += String.fromCharCode(uuidBytes[i]);
		}
		return uuidString as StableId;
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
		return {
			bytes,
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
		const localSessionId = sessionId ?? createSessionId();
		return new IdCompressor(WasmIdCompressor.deserialize(serialized.bytes, localSessionId));
	}
}
