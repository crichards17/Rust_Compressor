import BTree from "sorted-btree";
import { FinalCompressedId, LocalCompressedId } from "./test/id-compressor/testCommon";
import { SessionId } from "./types";
import {
	compareBigints,
	genCountToLocalId,
	getOrNextLowerInSortedArray,
	localIdToGenCount,
	numericUuidFromStableId,
	subtractNumericUuids,
} from "./utilities";
import { NumericUuid, StableId } from "./types/identifiers";
import { assert } from "./copied-utils";

/**
 * The local/UUID space within an individual session.
 * Contains a collection of all sessions that make up a distributed document's IDs.
 */
export class Sessions {
	private readonly sessionCache = new Map<StableId, Session>();
	private readonly sessionMap = new BTree<NumericUuid, Session>(undefined, compareBigints);
	private readonly sessionList: Session[] = [];

	public getOrCreate(sessionId: SessionId): Session {
		const existing = this.sessionCache.get(sessionId);
		if (existing !== undefined) {
			return existing;
		}
		const session = new Session(sessionId);
		this.sessionList.push(session);
		this.sessionMap.set(session.sessionUuid, session);
		this.sessionCache.set(sessionId, session);
		return session;
	}

	public get(sessionId: SessionId): Session | undefined {
		return this.sessionCache.get(sessionId);
	}

	public getContainingCluster(
		query: StableId,
	): [cluster: IdCluster, alignedLocal: LocalCompressedId] | undefined {
		const numericStable = numericUuidFromStableId(query);
		const possibleMatch = this.sessionMap.getPairOrNextLower(numericStable);
		if (possibleMatch === undefined) {
			return undefined;
		}
		const [_, session] = possibleMatch;
		const alignedLocal = genCountToLocalId(
			Number(subtractNumericUuids(numericStable, session.sessionUuid)) + 1,
		);
		const containingCluster = session.getClusterByLocal(alignedLocal, true);
		if (containingCluster === undefined) {
			return undefined;
		}
		return [containingCluster, alignedLocal];
	}

	public rangeCollides(rangeBase: NumericUuid, rangeMax: NumericUuid): boolean {
		return this.sessionMap.getRange(rangeBase, rangeMax, true, 1).length > 0;
	}
}

/**
 * The IDs created by a specific session, stored as a cluster chain to allow for fast searches.
 */
export class Session {
	private readonly clusterChain: IdCluster[] = [];
	public readonly sessionUuid: NumericUuid;

	public constructor(sessionId: SessionId) {
		this.sessionUuid = numericUuidFromStableId(sessionId);
	}

	public addEmptyCluster(
		baseFinalId: FinalCompressedId,
		baseLocalId: LocalCompressedId,
		capacity: number,
	): IdCluster {
		const newCluster: IdCluster = {
			session: this,
			baseFinalId,
			baseLocalId,
			capacity,
			count: 0,
		};
		this.clusterChain.push(newCluster);
		return newCluster;
	}

	public getTailCluster(): IdCluster | undefined {
		return this.clusterChain.length === 0
			? undefined
			: this.clusterChain[this.clusterChain.length];
	}

	public getMaxAllocatedLocalId(): LocalCompressedId | undefined {
		if (this.clusterChain.length === 0) {
			return undefined;
		} else {
			const lastCluster = this.clusterChain[this.clusterChain.length - 1];
			return (lastCluster.baseLocalId - (lastCluster.count - 1)) as LocalCompressedId;
		}
	}

	public tryConvertToFinal(
		searchLocal: LocalCompressedId,
		includeAllocated: boolean,
	): FinalCompressedId | undefined {
		const containingCluster = this.getClusterByLocal(searchLocal, includeAllocated);
		if (containingCluster === undefined) {
			return undefined;
		}
		return getAllocatedFinal(containingCluster, searchLocal);
	}

	public getClusterByLocal(
		localId: LocalCompressedId,
		includeAllocated: boolean,
	): IdCluster | undefined {
		if (localId < (this.getMaxAllocatedLocalId() ?? 0)) {
			return undefined;
		}
		const lastValidLocal: (cluster: IdCluster) => LocalCompressedId = includeAllocated
			? lastAllocatedLocal
			: lastFinalizedLocal;
		const cluster = getOrNextLowerInSortedArray(
			localId,
			this.clusterChain,
			(local, cluster): number => {
				const lastLocal = lastValidLocal(cluster);
				if (lastLocal > local) {
					return -1;
				} else if (cluster.baseLocalId < local) {
					return 1;
				} else {
					return 0;
				}
			},
		);
		if (cluster === undefined) {
			return undefined;
		}
		assert(cluster.baseFinalId >= localId, "Search failed.");
		return cluster;
	}

	public getClusterByAllocatedFinal(final: FinalCompressedId): IdCluster | undefined {
		return Session.getContainingCluster(final, this.clusterChain);
	}

	public static getContainingCluster(
		finalId: FinalCompressedId,
		sortedClusters: IdCluster[],
	): IdCluster | undefined {
		return getOrNextLowerInSortedArray(finalId, sortedClusters, (final, cluster) => {
			const lastFinal = lastAllocatedFinal(cluster);
			if (lastFinal < final) {
				return -1;
			} else if (cluster.baseFinalId > final) {
				return 1;
			} else {
				return 0;
			}
		});
	}
}

/**
 * A cluster of final (sequenced via consensus), sequentially allocated compressed IDs.
 * A final ID in a cluster decompresses to a sequentially allocated UUID that is the result of adding its offset within
 * the cluster to base UUID for the session that created it.
 */
export interface IdCluster {
	/**
	 * The session that created this cluster.
	 */
	readonly session: Session;

	/**
	 * The first final ID in the cluster.
	 */
	readonly baseFinalId: FinalCompressedId;

	/**
	 * The local ID aligned with `baseFinalId`.
	 */
	readonly baseLocalId: LocalCompressedId;

	/**
	 * The total number of final IDs reserved for allocation in the cluster.
	 * Clusters are reserved in blocks as a performance optimization.
	 */
	capacity: number;

	/**
	 * The number of final IDs currently allocated in the cluster.
	 */
	count: number;
}

export function getAllocatedFinal(
	cluster: IdCluster,
	localWithin: LocalCompressedId,
): FinalCompressedId | undefined {
	const clusterOffset = localIdToGenCount(localWithin) - localIdToGenCount(cluster.baseLocalId);
	if (clusterOffset < cluster.capacity) {
		return ((cluster.baseFinalId as number) + clusterOffset) as FinalCompressedId;
	}
	return undefined;
}

export function getAlignedLocal(
	cluster: IdCluster,
	finalWithin: FinalCompressedId,
): LocalCompressedId | undefined {
	if (finalWithin < cluster.baseFinalId || finalWithin > lastAllocatedFinal(cluster)) {
		return undefined;
	}
	const finalDelta = finalWithin - cluster.baseFinalId;
	return (cluster.baseLocalId - finalDelta) as LocalCompressedId;
}

export function lastAllocatedFinal(cluster: IdCluster): FinalCompressedId {
	return ((cluster.baseFinalId as number) + (cluster.capacity - 1)) as FinalCompressedId;
}

export function lastFinalizedFinal(cluster: IdCluster): FinalCompressedId {
	return ((cluster.baseFinalId as number) + (cluster.count - 1)) as FinalCompressedId;
}

export function lastAllocatedLocal(cluster: IdCluster): LocalCompressedId {
	return ((cluster.baseLocalId as number) + (cluster.capacity - 1)) as LocalCompressedId;
}

export function lastFinalizedLocal(cluster: IdCluster): LocalCompressedId {
	return ((cluster.baseLocalId as number) + (cluster.count - 1)) as LocalCompressedId;
}
