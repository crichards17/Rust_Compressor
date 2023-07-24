import { assert } from "./copied-utils";
import { IdCluster, Session } from "./sessions";
import { FinalCompressedId } from "./test/id-compressor/testCommon";

export class FinalSpace {
	private readonly clusters: IdCluster[] = [];

	public getClusterCount(): number {
		return this.clusters.length;
	}

	public getTailCluster(): IdCluster | undefined {
		return this.getClusterCount() === 0 ? undefined : this.clusters[this.clusters.length - 1];
	}

	public addCluster(newCluster: IdCluster) {
		const tailCluster = this.getTailCluster();
		assert(
			tailCluster === undefined || newCluster.baseFinalId > tailCluster.baseFinalId,
			"Cluster insert to final_space is out of order.",
		);
		this.clusters.push(newCluster);
	}

	public getContainingCluster(finalId: FinalCompressedId): IdCluster | undefined {
		return Session.getContainingCluster(finalId, this.clusters);
	}

	public getFinalIdLimit(): FinalCompressedId {
		if (this.clusters.length === 0) {
			return 0 as FinalCompressedId;
		}
		const lastCluster = this.clusters[this.clusters.length];
		return ((lastCluster.baseFinalId as number) + lastCluster.count) as FinalCompressedId;
	}
}
