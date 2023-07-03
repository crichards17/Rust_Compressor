import BTree from "sorted-btree";
import { FinalCompressedId, LocalCompressedId } from "./test/id-compressor/testCommon";
import { SessionId } from "./types";
import {
	compareStrings,
	genCountToLocalId,
	numericUuidFromStableId,
	offsetNumericUuid,
	subtractNumericUuids,
} from "./utilities";
import { NumericUuid, StableId } from "./types/identifiers";

/**
 * The local/UUID space within an individual session.
 * Contains a collection of all sessions that make up a distributed document's IDs.
 */
export class Sessions {
	// TODO: add map cache to accelerate session lookup
	private readonly sessionMap = new BTree<SessionId, Session>(undefined, compareStrings);
	private readonly sessionList: Session[] = [];

	public getOrCreate(sessionId: SessionId): Session {
		const existing = this.sessionMap.get(sessionId);
		if (existing !== undefined) {
			return existing;
		}
		const session = new Session(sessionId);
		this.sessionList.push(session);
		this.sessionMap.set(sessionId, session);
		return session;
	}

	public get(sessionId: SessionId): Session | undefined {
		return this.sessionMap.get(sessionId);
	}

	public getContainingCluster(
		query: StableId,
	): [Session, IdCluster, LocalCompressedId] | undefined {
		const possibleMatch = this.sessionMap.getPairOrNextLower(query as SessionId);
		if (possibleMatch === undefined) {
			return undefined;
		}
		const numericStable = numericUuidFromStableId(query);
		const [_, session] = possibleMatch;
		const maxNumericStable = session.getMaxAllocatedNumericStable();
		if (numericStable > maxNumericStable) {
			return undefined;
		}
		const alignedLocal = genCountToLocalId(
			Number(subtractNumericUuids(numericStable, session.sessionUuid)) + 1,
		);
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

	public getTailCluster(): IdCluster | undefined {
		return this.clusterChain.length === 0
			? undefined
			: this.clusterChain[this.clusterChain.length];
	}

	public getMaxAllocatedNumericStable(): NumericUuid {
		return this.clusterChain.length === 0
			? this.sessionUuid
			: offsetNumericUuid(
					this.sessionUuid,
					this.clusterChain[this.clusterChain.length - 1].count - 1,
			  );
	}
}

/**
 * A cluster of final (sequenced via consensus), sequentially allocated compressed IDs.
 * A final ID in a cluster decompresses to a sequentially allocated UUID that is the result of adding its offset within
 * the cluster to base UUID for the session that created it.
 */
export interface IdCluster {
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
