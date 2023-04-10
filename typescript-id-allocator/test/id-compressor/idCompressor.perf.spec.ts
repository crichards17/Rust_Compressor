/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

/* eslint-disable @typescript-eslint/no-non-null-assertion */

import {
	IdCreationRange,
	SerializedIdCompressorWithNoSession,
	CompressedId,
	FinalCompressedId,
	LocalCompressedId,
	OpSpaceCompressedId,
	SessionSpaceCompressedId,
	SessionId,
	StableId,
} from "../../src/types";
import { benchmark, BenchmarkType } from "@fluid-tools/benchmark";
import { take } from "../copied-utils/stochastic";
import {
	Client,
	CompressorFactory,
	IdCompressorTestNetwork,
	makeOpGenerator,
	performFuzzActions,
	sessionIds,
	TestIdData,
} from "./idCompressorTestUtilities";
import { defaultClusterCapacity, IdCompressor } from "../../src/IdCompressor";
import { isFinalId, isLocalId } from "./testCommon";
import { createSessionId, fail } from "../../src/util/utilities";

describe("IdCompressor Perf", () => {
	afterEach(() => {
		CompressorFactory.disposeAllCompressors();
	});

	const type = BenchmarkType.Measurement;
	const localClient = Client.Client1;
	const remoteClient = Client.Client2;
	let perfCompressor: IdCompressor | undefined;
	let perfNetwork: IdCompressorTestNetwork;
	let compressor: IdCompressor;

	function setupCompressors(clusterSize: number, allowLocal: boolean): IdCompressorTestNetwork {
		perfNetwork = new IdCompressorTestNetwork(clusterSize);
		[compressor] = createPerfCompressor(perfNetwork, allowLocal, localClient);
		perfCompressor = undefined;
		return perfNetwork;
	}

	function createPerfCompressor(
		network: IdCompressorTestNetwork,
		allowLocal: boolean,
		client: Client,
	): [IdCompressor, readonly TestIdData[]] {
		const maxClusterSize = 25;
		const generator = take(1000, makeOpGenerator({ validateInterval: 2000, maxClusterSize }));
		if (network.initialClusterSize > maxClusterSize) {
			network.enqueueCapacityChange(maxClusterSize);
		}
		performFuzzActions(
			generator,
			network,
			Math.E,
			allowLocal ? undefined : client,
			!allowLocal,
		);
		return [network.getCompressorUnsafe(client), network.getIdLog(client)];
	}

	function setupCompressorWithId(local: boolean): CompressedId {
		const clusterCapacity = defaultClusterCapacity;
		const network = setupCompressors(clusterCapacity, true);
		network.allocateAndSendIds(localClient, clusterCapacity);
		network.allocateAndSendIds(localClient, 1);
		if (!local) {
			network.deliverOperations(localClient);
		}

		const ids = network.getIdLog(localClient);
		const lastId = ids[ids.length - 1].id;
		return lastId;
	}

	function benchmarkWithIdTypes(creator: (local: boolean) => void) {
		for (const local of [true, false]) {
			creator(local);
		}
	}

	benchmark({
		type,
		title: `allocate local ID`,
		before: () => {
			setupCompressors(defaultClusterCapacity, true);
			perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.generateCompressedId();
		},
	});

	for (const clusterSize of [1, 10, 500, 1000]) {
		const numIds = 7;
		const session1 = "8150a099-5302-4672-b5f3-7a4492b59418" as SessionId;
		const session2 = "f2ded886-92da-4248-967b-eb96ee04cf51" as SessionId;
		let session: SessionId = session1;
		let lastFinalizedGenCount1 = 0;
		let lastFinalizedGenCount2 = 0;
		benchmark({
			type,
			title: `finalize a range of IDs (cluster size =${clusterSize})`,
			before: () => {
				setupCompressors(clusterSize, false);
				perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
			},
			benchmarkFn: () => {
				// Create a range with as minimal overhead as possible, as we'd like for this code to not exist
				// in the timing loop at all (but benchmark forces us to do so)
				const isFirstClient = session === session1;
				const firstGenCount =
					(isFirstClient ? lastFinalizedGenCount1 : lastFinalizedGenCount2) + 1;
				const lastGenCount = firstGenCount + numIds;
				const range: IdCreationRange = {
					sessionId: session,
					ids: {
						firstGenCount,
						lastGenCount,
					},
				};

				perfCompressor!.finalizeCreationRange(range);

				if (isFirstClient) {
					lastFinalizedGenCount1 = lastGenCount;
				} else {
					lastFinalizedGenCount2 = lastGenCount;
				}
				// Alternate clients to sidestep optimization that packs them all into last cluster
				session = isFirstClient ? session1 : session2;
			},
		});
	}

	benchmark({
		type,
		title: "takes a ID creation range",
		before: () => {
			setupCompressors(defaultClusterCapacity, true);
			perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.generateCompressedId();
			perfCompressor!.takeNextCreationRange();
		},
	});

	benchmarkWithIdTypes((local) => {
		let idToDecompress!: CompressedId;
		benchmark({
			type,
			title: `decompress final ID into stable IDs (${local ? "local" : "remote"})`,
			before: () => {
				idToDecompress = setupCompressorWithId(local);
				perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
			},
			benchmarkFn: () => {
				perfCompressor!.decompress(idToDecompress as SessionSpaceCompressedId);
			},
		});
	});

	benchmarkWithIdTypes((local) => {
		let stableToCompress!: StableId;
		benchmark({
			type,
			title: `compress a stable ID to a ${local ? "local" : "final"} ID`,
			before: () => {
				const idAdded = setupCompressorWithId(local);
				stableToCompress = compressor.decompress(idAdded as SessionSpaceCompressedId);
				perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
			},
			benchmarkFn: () => {
				perfCompressor!.recompress(stableToCompress);
			},
		});
	});

	let final!: FinalCompressedId;
	benchmark({
		type,
		title: `normalize a final ID from the local session to session space`,
		before: () => {
			const network = setupCompressors(defaultClusterCapacity, true);
			network.allocateAndSendIds(localClient, 1);
			network.deliverOperations(localClient);
			const log = network.getSequencedIdLog(localClient);
			const id = compressor.normalizeToOpSpace(log[log.length - 1].id);
			final = isFinalId(id) ? id : fail("not a final ID");
			perfCompressor = network.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.normalizeToSessionSpace(final, compressor.localSessionId);
		},
	});

	function getLastLocalId(client: Client, network: IdCompressorTestNetwork): LocalCompressedId {
		const log = network.getIdLog(client);
		for (let i = log.length - 1; i > 0; i--) {
			const cur = log[i].id;
			if (isLocalId(cur)) {
				return cur;
			}
		}
		fail("no local ID found in log");
	}

	let localId!: LocalCompressedId;
	benchmark({
		type,
		title: `normalize a local ID from the local session to session space`,
		before: () => {
			const network = setupCompressors(defaultClusterCapacity, true);
			network.deliverOperations(localClient);
			localId = getLastLocalId(localClient, network);
			perfCompressor = network.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.normalizeToOpSpace(localId);
		},
	});

	const remoteSessionId = sessionIds.get(remoteClient);
	let opSpaceId!: OpSpaceCompressedId;
	benchmark({
		type,
		title: `normalize a local ID from a remote session to session space`,
		before: () => {
			const network = setupCompressors(defaultClusterCapacity, true);
			network.deliverOperations(localClient);
			opSpaceId = getLastLocalId(remoteClient, network) as OpSpaceCompressedId;
			perfCompressor = network.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.normalizeToSessionSpace(opSpaceId, remoteSessionId);
		},
	});

	benchmark({
		type,
		title: `serialize an IdCompressor`,
		before: () => {
			setupCompressors(defaultClusterCapacity, false);
			perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.serialize(false);
		},
	});

	let serialized!: SerializedIdCompressorWithNoSession;
	const overrideRemoteSessionId = createSessionId();
	benchmark({
		type,
		title: `deserialize an IdCompressor`,
		before: () => {
			setupCompressors(defaultClusterCapacity, false);
			serialized = compressor.serialize(false);
		},
		benchmarkFn: () => {
			const compressor = IdCompressor.deserialize(serialized, overrideRemoteSessionId);
			compressor.dispose();
		},
	});
});
