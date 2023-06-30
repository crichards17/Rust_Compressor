import { ITelemetryLogger } from "@fluidframework/common-definitions";
import {
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
import { createSessionId } from "./utilities";
import { assert } from "./copied-utils";

export const defaultClusterCapacity = 512;

/**
 * See {@link IIdCompressor} and {@link IIdCompressorCore}
 */
export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	private constructor(
		public readonly localSessionId: SessionId,
		private readonly logger?: ITelemetryLogger,
	) {}

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
		const compressor = new IdCompressor(localSessionId, logger);
		return compressor;
	}

	/**
	 * The size of each newly created ID cluster.
	 */
	public get clusterCapacity(): number {
		assert(this.logger !== undefined, "");
		throw new Error("Not implemented.");
	}

	/**
	 * Must only be set with a value upon which consensus has been reached. Value must be greater than zero and less than
	 * `IdCompressor.maxClusterSize`.
	 */
	public set clusterCapacity(value: number) {
		throw new Error("Not implemented.");
	}

	public finalizeCreationRange(range: IdCreationRange): void {
		throw new Error("Not implemented.");
	}

	public takeNextCreationRange(): IdCreationRange {
		throw new Error("Not implemented.");
	}

	public generateCompressedId(): SessionSpaceCompressedId {
		throw new Error("Not implemented.");
	}

	public normalizeToOpSpace(id: SessionSpaceCompressedId): OpSpaceCompressedId {
		throw new Error("Not implemented.");
	}

	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId {
		throw new Error("Not implemented.");
	}

	public decompress(id: SessionSpaceCompressedId): StableId {
		throw new Error("Not implemented.");
	}

	public tryDecompress(id: SessionSpaceCompressedId): StableId | undefined {
		throw new Error("Not implemented.");
	}

	public recompress(uncompressed: StableId): SessionSpaceCompressedId {
		throw new Error("Not implemented.");
	}

	public tryRecompress(uncompressed: StableId): SessionSpaceCompressedId | undefined {
		throw new Error("Not implemented.");
	}

	public dispose(): void {}

	public serialize(withSession: true): SerializedIdCompressorWithOngoingSession;
	public serialize(withSession: false): SerializedIdCompressorWithNoSession;
	public serialize(withSession: boolean): SerializedIdCompressor {
		throw new Error("Not implemented.");
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
		throw new Error("Not implemented.");
	}
}
