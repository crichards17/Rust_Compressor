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
import { createSessionId, localIdToGenCount } from "./utilities";
import { assert } from "./copied-utils";
import { Session, Sessions } from "./sessions";
import { SessionSpaceNormalizer } from "./sessionSpaceNormalizer";
import { defaultClusterCapacity } from "./types/persisted-types";
import { FinalSpace } from "./finalSpace";
import { LocalCompressedId } from "./test/id-compressor/testCommon";

/**
 * See {@link IIdCompressor} and {@link IIdCompressorCore}
 */
export class IdCompressor implements IIdCompressor, IIdCompressorCore {
	/**
	 * Max allowed initial cluster size.
	 */
	public static maxClusterSize = 2 ** 20;

	// ----- Local state -----
	public readonly localSessionId: SessionId;
	private readonly localSession: Session;
	private readonly normalizer = new SessionSpaceNormalizer();
	private generatedIdCount = 0;
	// -----------------------

	// ----- Final state -----
	private nextRangeBaseGenCount: number = 1;
	private newClusterCapacity: number;
	private readonly sessions = new Sessions();
	private readonly finalSpace = new FinalSpace();
	// -----------------------

	private constructor(localSessionId: SessionId, private readonly logger?: ITelemetryLogger) {
		this.localSessionId = localSessionId;
		this.localSession = this.sessions.getOrCreate(localSessionId);
		this.newClusterCapacity = defaultClusterCapacity;
	}

	public static create(logger?: ITelemetryLogger): IdCompressor {
		return new IdCompressor(createSessionId(), logger);
	}

	/**
	 * The size of each newly created ID cluster.
	 */
	public get clusterCapacity(): number {
		return this.newClusterCapacity;
	}

	/**
	 * Must only be set with a value upon which consensus has been reached. Value must be greater than zero and less than
	 * `IdCompressor.maxClusterSize`.
	 */
	public set clusterCapacity(value: number) {
		assert(value > 0, "Clusters must have a positive capacity.");
		assert(value <= IdCompressor.maxClusterSize, "Clusters must not exceed max cluster size.");
		this.newClusterCapacity = value;
	}

	public generateCompressedId(): SessionSpaceCompressedId {
		this.generatedIdCount++;
		const tailCluster = this.localSession.getTailCluster();
		if (tailCluster === undefined) {
			return this.generateNextLocalId();
		}
		const clusterOffset = this.generatedIdCount - localIdToGenCount(tailCluster.baseLocalId);
		return tailCluster.capacity > clusterOffset
			? // Space in the cluster: eager final
			  (((tailCluster.baseFinalId as number) + clusterOffset) as SessionSpaceCompressedId)
			: // No space in the cluster, return next local
			  this.generateNextLocalId();
	}

	private generateNextLocalId(): LocalCompressedId {
		const newLocal = -this.generatedIdCount as LocalCompressedId;
		this.normalizer.addLocalRange(newLocal, 1);
		return newLocal;
	}

	public takeNextCreationRange(): IdCreationRange {
		const count = this.generatedIdCount - (this.nextRangeBaseGenCount - 1);
		if (count === 0) {
			return {
				sessionId: this.localSessionId,
			};
		}
		const range: IdCreationRange = {
			sessionId: this.localSessionId,
			ids: {
				firstGenCount: this.nextRangeBaseGenCount,
				count,
			},
		};
		this.nextRangeBaseGenCount = this.generatedIdCount + 1;
		return range;
	}

	public finalizeCreationRange(range: IdCreationRange): void {
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
