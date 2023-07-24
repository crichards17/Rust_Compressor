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
import {
	createSessionId,
	genCountToLocalId,
	localIdToGenCount,
	numericUuidFromStableId,
	offsetNumericUuid,
	stableIdFromNumericUuid,
	subtractNumericUuids,
} from "./utilities";
import { assert, fail } from "./copied-utils";
import {
	getAlignedLocal,
	getAllocatedFinal,
	lastFinalizedLocal,
	Session,
	Sessions,
} from "./sessions";
import { SessionSpaceNormalizer } from "./sessionSpaceNormalizer";
import { defaultClusterCapacity } from "./types/persisted-types";
import { FinalSpace } from "./finalSpace";
import { isFinalId, LocalCompressedId } from "./test/id-compressor/testCommon";

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
		assert(this.logger !== undefined, "use logger");
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
		const compressor = new IdCompressor(localSessionId, logger);
		return compressor;
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
		if (isFinalId(id)) {
			return id;
		} else {
			const local = id as unknown as LocalCompressedId;
			if (!this.normalizer.contains(local)) {
				throw new Error("Invalid ID to normalize.");
			}
			const finalForm = this.localSession.tryConvertToFinal(local, true);
			return finalForm === undefined
				? (local as unknown as OpSpaceCompressedId)
				: (finalForm as OpSpaceCompressedId);
		}
	}

	public normalizeToSessionSpace(
		id: OpSpaceCompressedId,
		originSessionId: SessionId,
	): SessionSpaceCompressedId {
		if (isFinalId(id)) {
			const containingCluster = this.localSession.getClusterByAllocatedFinal(id);
			if (containingCluster === undefined) {
				// Does not exist in local cluster chain
				if (id > this.finalSpace.getFinalIdLimit()) {
					throw new Error("Unknown op space ID.");
				}
				return id as unknown as SessionSpaceCompressedId;
			} else {
				const alignedLocal = getAlignedLocal(containingCluster, id);
				if (alignedLocal === undefined) {
					throw new Error("Unknown op space ID.");
				}
				if (this.normalizer.contains(alignedLocal)) {
					return alignedLocal;
				} else if (localIdToGenCount(alignedLocal) <= this.generatedIdCount) {
					return id as unknown as SessionSpaceCompressedId;
				} else {
					throw new Error("Unknown op space ID.");
				}
			}
		} else {
			const localToNormalize = id as unknown as LocalCompressedId;
			if (originSessionId === this.localSessionId) {
				if (this.normalizer.contains(localToNormalize)) {
					return localToNormalize;
				} else if (localIdToGenCount(localToNormalize) <= this.generatedIdCount) {
					// Id is an eager final
					const correspondingFinal = this.localSession.tryConvertToFinal(
						localToNormalize,
						true,
					);
					if (correspondingFinal === undefined) {
						throw new Error("Unknown op space ID.");
					}
					return correspondingFinal as unknown as SessionSpaceCompressedId;
				} else {
					throw new Error("Unknown op space ID.");
				}
			} else {
				// LocalId from a remote session
				const remoteSession = this.sessions.get(originSessionId);
				const correspondingFinal = remoteSession?.tryConvertToFinal(
					localToNormalize,
					false,
				);
				if (correspondingFinal === undefined) {
					throw new Error("Unknown op space ID.");
				}
				return correspondingFinal as unknown as SessionSpaceCompressedId;
			}
		}
	}

	public decompress(id: SessionSpaceCompressedId): StableId {
		return (
			this.tryDecompress(id) ?? fail("Compressed ID was not generated by this compressor.")
		);
	}

	public tryDecompress(id: SessionSpaceCompressedId): StableId | undefined {
		if (isFinalId(id)) {
			const containingCluster = this.finalSpace.getContainingCluster(id);
			if (containingCluster === undefined) {
				return undefined;
			}
			const alignedLocal = getAlignedLocal(containingCluster, id);
			if (alignedLocal === undefined) {
				return undefined;
			}
			const alignedGenCount = localIdToGenCount(alignedLocal);
			if (alignedLocal < lastFinalizedLocal(containingCluster)) {
				// must be an id generated (allocated or finalized) by the local session, or a finalized id from a remote session
				if (containingCluster.session === this.localSession) {
					if (this.normalizer.contains(alignedLocal)) {
						// the supplied ID was final, but was have been minted as local. the supplier should not have the ID in final form.
						return undefined;
					}
					if (alignedGenCount > this.generatedIdCount) {
						// the supplied ID was never generated
						return undefined;
					}
				} else {
					return undefined;
				}
			}

			return stableIdFromNumericUuid(
				offsetNumericUuid(containingCluster.session.sessionUuid, alignedGenCount - 1),
			);
		} else {
			const localToDecompress = id as unknown as LocalCompressedId;
			if (!this.normalizer.contains(localToDecompress)) {
				return undefined;
			}
			return stableIdFromNumericUuid(
				offsetNumericUuid(
					this.localSession.sessionUuid,
					localIdToGenCount(localToDecompress) - 1,
				),
			);
		}
	}

	public recompress(uncompressed: StableId): SessionSpaceCompressedId {
		return this.tryRecompress(uncompressed) ?? fail("Could not recompress.");
	}

	public tryRecompress(uncompressed: StableId): SessionSpaceCompressedId | undefined {
		const match = this.sessions.getContainingCluster(uncompressed);
		if (match === undefined) {
			const numericUncompressed = numericUuidFromStableId(uncompressed);
			const offset = subtractNumericUuids(numericUncompressed, this.localSession.sessionUuid);
			if (offset < Number.MAX_SAFE_INTEGER) {
				const genCountEquivalent = Number(offset) + 1;
				const localEquivalent = genCountToLocalId(genCountEquivalent);
				if (this.normalizer.contains(localEquivalent)) {
					return localEquivalent;
				}
			}
			return undefined;
		} else {
			const [containingCluster, alignedLocal] = match;
			if (containingCluster.session === this.localSession) {
				// Local session
				if (this.normalizer.contains(alignedLocal)) {
					return alignedLocal;
				} else if (localIdToGenCount(alignedLocal) < this.generatedIdCount) {
					// Id is an eager final
					return getAllocatedFinal(containingCluster, alignedLocal) as
						| SessionSpaceCompressedId
						| undefined;
				} else {
					return undefined;
				}
			} else {
				// Not the local session
				return localIdToGenCount(alignedLocal) < lastFinalizedLocal(containingCluster)
					? (getAllocatedFinal(containingCluster, alignedLocal) as
							| SessionSpaceCompressedId
							| undefined)
					: undefined;
			}
		}
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
