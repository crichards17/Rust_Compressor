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
} from "./idCompressorTestUtilities";
import { defaultClusterCapacity, IdCompressor } from "../../src/IdCompressor";
import { isFinalId, isLocalId } from "./testCommon";
import { createSessionId, fail } from "../../src/util/utilities";
import { assert } from "console";

describe("IdCompressor Perf", () => {
	afterEach(() => {
		CompressorFactory.disposeAllCompressors();
	});

	const type = BenchmarkType.Measurement;
	const localClient = Client.Client1;
	const remoteClient = Client.Client2;
	let perfCompressor: IdCompressor;

	function setupCompressors(clusterSize: number, allowLocal: boolean): IdCompressorTestNetwork {
		const perfNetwork = new IdCompressorTestNetwork(clusterSize);
		const maxClusterSize = 25;
		const generator = take(1000, makeOpGenerator({ validateInterval: 2000, maxClusterSize }));
		if (perfNetwork.initialClusterSize > maxClusterSize) {
			perfNetwork.enqueueCapacityChange(maxClusterSize);
		}
		performFuzzActions(
			generator,
			perfNetwork,
			Math.E,
			allowLocal ? undefined : localClient,
			!allowLocal,
		);
		perfCompressor = perfNetwork.getCompressorUnsafeNoProxy(localClient);
		return perfNetwork;
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

	function benchmarkWithFlag(creator: (flag: boolean) => void) {
		for (const flag of [true, false]) {
			creator(flag);
		}
	}

	benchmark({
		type,
		title: `allocate local ID`,
		before: () => {
			setupCompressors(defaultClusterCapacity, true);
		},
		benchmarkFn: () => {
			perfCompressor!.generateCompressedId();
		},
	});

	benchmark({
		type,
		title: "take an ID creation range",
		before: () => {
			setupCompressors(defaultClusterCapacity, true);
		},
		benchmarkFn: () => {
			perfCompressor!.generateCompressedId();
			perfCompressor!.takeNextCreationRange();
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

	let final!: FinalCompressedId;
	benchmark({
		type,
		title: `normalize a final ID from the local session to session space`,
		before: () => {
			const network = setupCompressors(defaultClusterCapacity, true);
			network.allocateAndSendIds(localClient, 1);
			network.deliverOperations(localClient);
			const log = network.getSequencedIdLog(localClient);
			const sessionId = log[log.length - 1].id;
			//assert(isLocalId(sessionId));
			const opSpaceId = perfCompressor.normalizeToOpSpace(sessionId);
			final = isFinalId(opSpaceId) ? opSpaceId : fail("not a final ID");
			perfCompressor = network.getCompressorUnsafeNoProxy(localClient);
		},
		benchmarkFn: () => {
			perfCompressor!.normalizeToSessionSpace(final, perfCompressor.localSessionId);
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

	benchmarkWithFlag((local) => {
		let idToDecompress!: CompressedId;
		benchmark({
			type,
			title: `decompress final ID into stable IDs (${local ? "local" : "remote"})`,
			before: () => {
				idToDecompress = setupCompressorWithId(local);
			},
			benchmarkFn: () => {
				perfCompressor!.decompress(idToDecompress as SessionSpaceCompressedId);
			},
		});
	});

	benchmarkWithFlag((local) => {
		let stableToCompress!: StableId;
		benchmark({
			type,
			title: `recompress a stable ID to a ${local ? "local" : "final"} ID`,
			before: () => {
				const idAdded = setupCompressorWithId(local);
				stableToCompress = perfCompressor.decompress(idAdded as SessionSpaceCompressedId);
			},
			benchmarkFn: () => {
				perfCompressor!.recompress(stableToCompress);
			},
		});
	});

	benchmark({
		type,
		title: `serialize an IdCompressor`,
		before: () => {
			setupCompressors(defaultClusterCapacity, false);
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
			serialized = perfCompressor.serialize(false);
		},
		benchmarkFn: () => {
			const compressor = IdCompressor.deserialize(serialized, overrideRemoteSessionId);
			compressor.dispose();
		},
	});
});
