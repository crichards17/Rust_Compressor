/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

import { strict as assert } from "assert";
import { MockLogger } from "@fluidframework/telemetry-utils";
import { validateAssertionError } from "@fluidframework/test-runtime-utils";
import {
	performFuzzActions,
	sessionIds,
	IdCompressorTestNetwork,
	Client,
	DestinationClient,
	MetaClient,
	expectSerializes,
	roundtrip,
	makeOpGenerator,
	CompressorFactory,
} from "./idCompressorTestUtilities";
import { compressorEquals, isFinalId, isLocalId } from "./testCommon";
import { take } from "../copied-utils/stochastic";
import { OpSpaceCompressedId, SessionSpaceCompressedId, StableId } from "../../src/types";
import { createSessionId, fail } from "../../src/util/utilities";
import { IdCompressor } from "../../src/IdCompressor";
import { setFlagsFromString } from "v8";
import { runInNewContext } from "vm";

describe("IdCompressor", () => {
	it("Memory pressure measurement", () => {
		const numSessions = 5000;
		const capacity = 10;
		const compressor = IdCompressor.create();
		compressor.clusterCapacity = capacity;
		for (let i = 0; i < numSessions; i++) {
			if (Math.random() > 0.1) {
				for (let j = 0; j < capacity; j++) {
					compressor.generateCompressedId();
				}
				compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			}
			compressor.finalizeCreationRange({
				sessionId: createSessionId(),
				ids: {
					firstGenCount: 1,
					count: capacity,
				},
			});
		}
		setFlagsFromString("--expose_gc");
		const gc = runInNewContext("gc"); // nocommit
		gc();
		console.log(process.memoryUsage().heapUsed);
	});

	describe("Telemetry", () => {
		itCompressor("emits first cluster and new cluster telemetry events", () => {
			const mockLogger = new MockLogger();
			const compressor = CompressorFactory.createCompressor(Client.Client1, 5, mockLogger);
			const localId1 = compressor.generateCompressedId();
			assert(isLocalId(localId1));
			compressor.finalizeCreationRange(compressor.takeNextCreationRange());

			mockLogger.assertMatch([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 0,
					localIdCount: 1,
					rangeSize: 1,
					clusterCapacity: 5,
					clusterChange: "Creation",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
		});

		itCompressor("emits new cluster event on second cluster", () => {
			// Fill the first cluster
			const mockLogger = new MockLogger();
			const compressor = CompressorFactory.createCompressor(Client.Client1, 1, mockLogger);
			compressor.generateCompressedId();
			const range = compressor.takeNextCreationRange();
			compressor.finalizeCreationRange(range);

			// Create another cluster with a different client so that expansion doesn't happen
			const mockLogger2 = new MockLogger();
			const compressor2 = CompressorFactory.createCompressor(Client.Client2, 1, mockLogger2);
			compressor2.finalizeCreationRange(range);
			compressor2.generateCompressedId();
			const range2 = compressor2.takeNextCreationRange();
			compressor2.finalizeCreationRange(range2);
			compressor.finalizeCreationRange(range2);
			// Make sure we emitted the FirstCluster event
			mockLogger.assertMatchAny([
				{
					clusterChange: "Creation",
				},
			]);
			mockLogger.clear();

			// Fill the one remaining spot and make sure no clusters are created/expanded
			compressor.generateCompressedId();
			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					clusterChange: "None",
				},
			]);

			// Trigger a new cluster creation
			compressor.generateCompressedId();
			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					clusterChange: "Creation",
				},
			]);
		});

		itCompressor("correctly logs telemetry events for eager final id allocations", () => {
			const mockLogger = new MockLogger();
			const compressor = CompressorFactory.createCompressor(Client.Client1, 5, mockLogger);
			const localId1 = compressor.generateCompressedId();
			assert(isLocalId(localId1));

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 0,
					localIdCount: 1,
					clusterChange: "Creation",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
			mockLogger.clear();
			const finalId1 = compressor.generateCompressedId();
			const finalId2 = compressor.generateCompressedId();
			assert(isFinalId(finalId1));
			assert(isFinalId(finalId2));

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 2,
					localIdCount: 0,
					clusterChange: "None",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
		});

		itCompressor("correctly logs telemetry events for expansion case", () => {
			const mockLogger = new MockLogger();
			const compressor = CompressorFactory.createCompressor(Client.Client1, 5, mockLogger);
			const localId1 = compressor.generateCompressedId();
			assert(isLocalId(localId1));

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 0,
					localIdCount: 1,
					clusterChange: "Creation",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
			mockLogger.clear();

			for (let i = 0; i < 5; i++) {
				const id = compressor.generateCompressedId();
				assert(isFinalId(id));
			}

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatchAny([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 5,
					localIdCount: 0,
					clusterChange: "None",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
			mockLogger.clear();

			const expansionId1 = compressor.generateCompressedId();
			const expansionId2 = compressor.generateCompressedId();
			assert(isLocalId(expansionId1));
			assert(isLocalId(expansionId2));

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			mockLogger.assertMatch([
				{
					eventName: "RuntimeIdCompressor:IdCompressorFinalizeStatus",
					eagerFinalIdCount: 0,
					localIdCount: 2,
					clusterChange: "Expansion",
					sessionId: sessionIds.get(Client.Client1),
				},
			]);
		});

		itCompressor("emits telemetry when serialized", () => {
			const mockLogger = new MockLogger();
			const compressor = CompressorFactory.createCompressor(Client.Client1, 5, mockLogger);
			const localId1 = compressor.generateCompressedId();
			assert(isLocalId(localId1));

			compressor.finalizeCreationRange(compressor.takeNextCreationRange());
			compressor.serialize(false);

			mockLogger.assertMatchAny([
				{
					eventName: "RuntimeIdCompressor:SerializedIdCompressorSize",
					size: 80,
				},
			]);
		});
	});

	describeNetwork("Networked", (itNetwork) => {
		itNetwork(
			"upholds the invariant that IDs always decompress to the same UUID",
			2,
			(network) => {
				network.allocateAndSendIds(Client.Client1, 5);
				network.allocateAndSendIds(Client.Client2, 5);
				network.allocateAndSendIds(Client.Client3, 5);

				const preAckLocals = new Map<Client, [SessionSpaceCompressedId, string][]>();
				for (const [client, compressor] of network.getTargetCompressors(MetaClient.All)) {
					const locals: [SessionSpaceCompressedId, string][] = [];
					for (const idData of network.getIdLog(client)) {
						locals.push([idData.id, compressor.decompress(idData.id)]);
					}
					preAckLocals.set(client, locals);
				}

				// Ack all IDs
				network.deliverOperations(DestinationClient.All);

				for (const [client, compressor] of network.getTargetCompressors(MetaClient.All)) {
					const preAckLocalIds =
						preAckLocals.get(client) ?? fail("Expected preack locals for client");
					let i = 0;
					for (const idData of network.getIdLog(client)) {
						if (idData.originatingClient === client) {
							assert(!isFinalId(idData.id));
							const currentUuid = compressor.decompress(idData.id);
							assert.equal(currentUuid, preAckLocalIds[i % preAckLocalIds.length][1]);
							i++;
						}
					}
				}
			},
		);

		itNetwork("can normalize session space IDs to op space", 5, (network) => {
			const clusterCapacity = 5;
			const idCount = clusterCapacity * 2;
			for (let i = 0; i < idCount; i++) {
				network.allocateAndSendIds(Client.Client1, 1);
				network.allocateAndSendIds(Client.Client2, 1);
				network.allocateAndSendIds(Client.Client3, 1);
			}

			for (const [client, compressor] of network.getTargetCompressors(MetaClient.All)) {
				for (const idData of network.getIdLog(client)) {
					assert.equal(idData.originatingClient, client);
					assert(isLocalId(compressor.normalizeToOpSpace(idData.id)));
				}
			}

			network.deliverOperations(DestinationClient.All);

			for (const [client, compressor] of network.getTargetCompressors(MetaClient.All)) {
				for (const idData of network.getIdLog(client)) {
					assert(isFinalId(compressor.normalizeToOpSpace(idData.id)));
				}
			}
		});

		itNetwork(
			"can normalize local op space IDs from a local session to session space IDs",
			(network) => {
				const compressor = network.getCompressor(Client.Client1);
				network.allocateAndSendIds(Client.Client1, 1);
				network.deliverOperations(Client.Client1);
				const sessionSpaceIds = network.getIdLog(Client.Client1);
				const opSpaceId = compressor.normalizeToOpSpace(sessionSpaceIds[0].id);
				const sessionSpaceId = compressor.normalizeToSessionSpace(
					opSpaceId,
					compressor.localSessionId,
				);
				assert(isFinalId(opSpaceId));
				assert(isLocalId(sessionSpaceId));
			},
		);

		itNetwork(
			"can normalize local op space IDs from a remote session to session space IDs",
			(network) => {
				const compressor1 = network.getCompressor(Client.Client1);
				const compressor2 = network.getCompressor(Client.Client2);
				const opSpaceIds = network.allocateAndSendIds(Client.Client1, 1);
				// Mimic sending a reference to an ID that hasn't been acked yet, such as in a slow network
				const id = opSpaceIds[0];
				const getSessionNormalizedId = () =>
					compressor2.normalizeToSessionSpace(id, compressor1.localSessionId);
				assert.throws(getSessionNormalizedId, (e) =>
					validateAssertionError(
						e,
						"No IDs have ever been finalized by the supplied session.",
					),
				);
				network.deliverOperations(Client.Client2);
				assert(isFinalId(getSessionNormalizedId()));
			},
		);

		function expectSequencedLogsAlign(
			network: IdCompressorTestNetwork,
			client1: Client,
			client2: Client,
			numUnifications = 0,
		): void {
			network.deliverOperations(DestinationClient.All);
			assert(client1 !== client2, "Clients must not be the same");
			const log1 = network.getSequencedIdLog(client1);
			const log2 = network.getSequencedIdLog(client2);
			assert.equal(log1.length, log2.length);
			const compressor1 = network.getCompressor(client1);
			const compressor2 = network.getCompressor(client2);
			const ids = new Set<OpSpaceCompressedId>();
			const uuids = new Set<StableId>();
			for (let i = 0; i < log1.length; i++) {
				const data1 = log1[i];
				const id1 = compressor1.normalizeToOpSpace(data1.id);
				const id2 = compressor2.normalizeToOpSpace(log2[i].id);
				assert(isFinalId(id1));
				ids.add(id1);
				assert.equal(id1, id2);
				const uuidOrOverride1 = compressor1.decompress(
					compressor1.normalizeToSessionSpace(id1, compressor1.localSessionId),
				);
				uuids.add(uuidOrOverride1);
				assert.equal(
					uuidOrOverride1,
					compressor2.decompress(
						compressor2.normalizeToSessionSpace(id2, compressor2.localSessionId),
					),
				);
			}
			const expectedSize = log1.length - numUnifications;
			assert.equal(ids.size, expectedSize);
			assert.equal(uuids.size, expectedSize);
		}

		itNetwork("produces ID spaces correctly", (network) => {
			// This test asserts that IDs returned from IdCompressor APIs are correctly encoded as either local or final.
			// This is a glass box test in that it assumes the negative/positive encoding of CompressedIds (negative = local, positive = final).
			const compressor1 = network.getCompressor(Client.Client1);

			// Client 1 makes three IDs
			network.allocateAndSendIds(Client.Client1, 3);
			network.getIdLog(Client.Client1).forEach(({ id }) => assert(isLocalId(id)));

			// Client 1's IDs have not been acked so have no op space equivalent
			network
				.getIdLog(Client.Client1)
				.forEach((idData) => assert(isLocalId(compressor1.normalizeToOpSpace(idData.id))));

			// Client 1's IDs are acked
			network.deliverOperations(Client.Client1);
			network.getIdLog(Client.Client1).forEach(({ id }) => assert(isLocalId(id)));

			// Client 2 makes three IDs
			network.allocateAndSendIds(Client.Client2, 3);

			network.getIdLog(Client.Client2).forEach(({ id }) => assert(isLocalId(id)));

			// Client 1 receives Client 2's IDs
			network.deliverOperations(Client.Client1);

			network
				.getIdLog(Client.Client1)
				.slice(-3)
				.forEach(({ id }) => assert(isFinalId(id)));

			// All IDs have been acked or are from another client, and therefore have a final form in op space
			network
				.getIdLog(Client.Client1)
				.forEach(({ id }) => assert(isFinalId(compressor1.normalizeToOpSpace(id))));

			// Compression should preserve ID space correctness
			network.getIdLog(Client.Client1).forEach((idData) => {
				const roundtripped = compressor1.recompress(compressor1.decompress(idData.id));
				assert.equal(Math.sign(roundtripped), Math.sign(idData.id));
			});

			network.getIdLog(Client.Client1).forEach((idData) => {
				const opNormalized = compressor1.normalizeToOpSpace(idData.id);
				assert.equal(
					Math.sign(compressor1.normalizeToSessionSpace(opNormalized, idData.sessionId)),
					Math.sign(idData.id),
				);
			});
		});

		itNetwork("produces consistent IDs with large fuzz input", (network) => {
			const generator = take(5000, makeOpGenerator({}));
			performFuzzActions(generator, network, 1984, undefined, true, (n) =>
				n.assertNetworkState(),
			);
			network.deliverOperations(DestinationClient.All);
		});

		itNetwork("can set the cluster size via API", 2, (network) => {
			const compressor = network.getCompressor(Client.Client1);
			const compressor2 = network.getCompressor(Client.Client2);
			const initialClusterCapacity = compressor.clusterCapacity;
			network.allocateAndSendIds(Client.Client1, 1);
			network.allocateAndSendIds(Client.Client2, 1);
			network.enqueueCapacityChange(5);
			network.allocateAndSendIds(Client.Client1, 3);
			const opSpaceIds = network.allocateAndSendIds(Client.Client2, 3);
			network.deliverOperations(DestinationClient.All);
			// Glass box test, as it knows the order of final IDs
			assert.equal(
				compressor.normalizeToSessionSpace(opSpaceIds[2], compressor2.localSessionId),
				(initialClusterCapacity + 1) * 2 + compressor.clusterCapacity + 1,
			);
		});

		itNetwork("does not decompress ids for empty parts of clusters", 2, (network) => {
			// This is a glass box test in that it creates a final ID outside of the ID compressor
			network.allocateAndSendIds(Client.Client1, 1);
			network.deliverOperations(DestinationClient.All);
			const id = network.getSequencedIdLog(Client.Client2)[0].id;
			assert(isFinalId(id));
			// eslint-disable-next-line @typescript-eslint/restrict-plus-operands
			const emptyId = (id + 1) as SessionSpaceCompressedId;
			assert.throws(
				() => network.getCompressor(Client.Client2).decompress(emptyId),
				(e) =>
					validateAssertionError(
						e,
						"Compressed ID was not generated by this compressor.",
					),
			);
		});

		describe("Finalizing", () => {
			itNetwork("can finalize IDs from multiple clients", (network) => {
				network.allocateAndSendIds(Client.Client1, 3);
				network.allocateAndSendIds(
					Client.Client2,
					network.getCompressor(Client.Client2).clusterCapacity * 2,
				);
				network.allocateAndSendIds(Client.Client3, 5);
				expectSequencedLogsAlign(network, Client.Client1, Client.Client2);
			});

			itNetwork("can finalize a range when the current cluster is full", 5, (network) => {
				const clusterCapacity = network.getCompressor(Client.Client1).clusterCapacity;
				network.allocateAndSendIds(Client.Client1, clusterCapacity);
				network.allocateAndSendIds(Client.Client2, clusterCapacity);
				network.allocateAndSendIds(Client.Client1, clusterCapacity);
				expectSequencedLogsAlign(network, Client.Client1, Client.Client2);
			});

			itNetwork("can finalize a range that spans multiple clusters", 5, (network) => {
				const clusterCapacity = network.getCompressor(Client.Client1).clusterCapacity;
				network.allocateAndSendIds(Client.Client1, 1);
				network.allocateAndSendIds(Client.Client2, 1);
				network.allocateAndSendIds(Client.Client1, clusterCapacity * 3);
				expectSequencedLogsAlign(network, Client.Client1, Client.Client2);
			});
		});

		describe("Serialization", () => {
			itNetwork(
				"prevents attempts to resume a session from a serialized compressor with no session",
				(network) => {
					const compressor = network.getCompressor(Client.Client1);
					network.allocateAndSendIds(Client.Client2, 1);
					network.allocateAndSendIds(Client.Client3, 1);
					network.deliverOperations(Client.Client1);
					const serializedWithoutLocalState = compressor.serialize(false);
					assert.throws(
						() =>
							CompressorFactory.deserialize(
								serializedWithoutLocalState,
								sessionIds.get(Client.Client2),
							),
						(e) => validateAssertionError(e, "Cannot resume existing session."),
					);
				},
			);

			itNetwork("round-trips local state", 3, (network) => {
				network.allocateAndSendIds(Client.Client1, 2);
				network.allocateAndSendIds(Client.Client2, 3);
				network.allocateAndSendIds(Client.Client1, 5);
				network.allocateAndSendIds(Client.Client1, 5);
				network.allocateAndSendIds(Client.Client3, 3);
				network.allocateAndSendIds(Client.Client2, 3);
				network.deliverOperations(Client.Client1);
				// Some un-acked locals at the end
				network.allocateAndSendIds(Client.Client1, 4);
				expectSerializes(network.getCompressor(Client.Client1));
			});

			itNetwork("can serialize a partially empty cluster", 5, (network) => {
				network.allocateAndSendIds(Client.Client1, 2);
				network.deliverOperations(DestinationClient.All);
				expectSerializes(network.getCompressor(Client.Client1));
				expectSerializes(network.getCompressor(Client.Client3));
			});

			itNetwork("can serialize a full cluster", 2, (network) => {
				network.allocateAndSendIds(Client.Client1, 2);
				network.deliverOperations(DestinationClient.All);
				expectSerializes(network.getCompressor(Client.Client1));
				expectSerializes(network.getCompressor(Client.Client3));
			});

			itNetwork("can serialize full clusters from different clients", 2, (network) => {
				network.allocateAndSendIds(Client.Client1, 2);
				network.allocateAndSendIds(Client.Client2, 2);
				network.deliverOperations(DestinationClient.All);
				expectSerializes(network.getCompressor(Client.Client1));
				expectSerializes(network.getCompressor(Client.Client3));
			});

			itNetwork("can serialize clusters of different sizes and clients", 3, (network) => {
				network.allocateAndSendIds(Client.Client1, 2);
				network.allocateAndSendIds(Client.Client2, 3);
				network.allocateAndSendIds(Client.Client1, 5);
				network.allocateAndSendIds(Client.Client1, 5);
				network.allocateAndSendIds(Client.Client2, 3);
				network.deliverOperations(DestinationClient.All);
				expectSerializes(network.getCompressor(Client.Client1));
				expectSerializes(network.getCompressor(Client.Client3));
			});

			// TODO: test in Rust
			// itNetwork(
			// 	"packs IDs into a single cluster when a single client generates non-overridden ids",
			// 	3,
			// 	(network) => {
			// 		network.allocateAndSendIds(Client.Client1, 20);
			// 		network.deliverOperations(DestinationClient.All);
			// 		const [serialized1WithNoSession, serialized1WithSession] = expectSerializes(
			// 			network.getCompressor(Client.Client1),
			// 		);
			// 		assert.equal(serialized1WithNoSession.clusters.length, 1);
			// 		assert.equal(serialized1WithSession.clusters.length, 1);
			// 		const [serialized3WithNoSession, serialized3WithSession] = expectSerializes(
			// 			network.getCompressor(Client.Client3),
			// 		);
			// 		assert.equal(serialized3WithNoSession.clusters.length, 1);
			// 		assert.equal(serialized3WithSession.clusters.length, 1);
			// 	},
			// );

			itNetwork(
				"can resume a session and interact with multiple other clients",
				3,
				(network) => {
					const clusterSize = network.getCompressor(Client.Client1).clusterCapacity;
					network.allocateAndSendIds(Client.Client1, clusterSize);
					network.allocateAndSendIds(Client.Client2, clusterSize);
					network.allocateAndSendIds(Client.Client3, clusterSize);
					network.allocateAndSendIds(Client.Client1, clusterSize);
					network.allocateAndSendIds(Client.Client2, clusterSize);
					network.allocateAndSendIds(Client.Client3, clusterSize);
					network.deliverOperations(DestinationClient.All);
					network.goOfflineThenResume(Client.Client1);
					network.allocateAndSendIds(Client.Client1, 2);
					network.allocateAndSendIds(Client.Client2, 2);
					network.allocateAndSendIds(Client.Client3, 2);
					expectSequencedLogsAlign(network, Client.Client1, Client.Client2);
				},
			);

			itNetwork("can serialize after a large fuzz input", 3, (network) => {
				const generator = take(5000, makeOpGenerator({}));
				performFuzzActions(generator, network, Math.PI, undefined, true, (n) => {
					// Periodically check that everyone in the network has the same serialized state
					n.deliverOperations(DestinationClient.All);
					const compressors = n.getTargetCompressors(DestinationClient.All);
					let deserializedPrev = roundtrip(compressors[0][1], false)[1];
					for (let i = 1; i < compressors.length; i++) {
						const deserializedCur = roundtrip(compressors[i][1], false)[1];
						assert(compressorEquals(deserializedPrev, deserializedCur, false));
						deserializedPrev = deserializedCur;
					}
				});
				expectSerializes(network.getCompressor(Client.Client1));
				expectSerializes(network.getCompressor(Client.Client2));
				expectSerializes(network.getCompressor(Client.Client3));
			});
		});
	});
});

type NetworkTestFunction = (
	title: string,
	test: (network: IdCompressorTestNetwork) => void,
) => void;

type NetworkTestFunctionWithCapacity = (
	title: string,
	initialClusterCapacity: number,
	test: (network: IdCompressorTestNetwork) => void,
) => void;

function createNetworkTestFunction(
	validateAfter: boolean,
): NetworkTestFunction & NetworkTestFunctionWithCapacity {
	return (
		title: string,
		testOrCapacity: ((network: IdCompressorTestNetwork) => void) | number,
		test?: (network: IdCompressorTestNetwork) => void,
	) => {
		it(title, () => {
			let network: IdCompressorTestNetwork | undefined = undefined;
			try {
				const hasCapacity = typeof testOrCapacity === "number";
				const capacity = hasCapacity ? testOrCapacity : undefined;
				network = new IdCompressorTestNetwork(capacity);
				(hasCapacity ? test ?? fail("test must be defined") : testOrCapacity)(network);
				if (validateAfter) {
					network.deliverOperations(DestinationClient.All);
					network.assertNetworkState();
				}
			} finally {
				network?.dispose();
			}
		}).timeout(10000);
	};
}

function describeNetwork(
	title: string,
	its: (itFunc: NetworkTestFunction & NetworkTestFunctionWithCapacity) => void,
) {
	describe(title, () => {
		its(createNetworkTestFunction(false));
	});

	describe(`${title} (with validation)`, () => {
		its(createNetworkTestFunction(true));
	});
}

function itCompressor(title: string, testFn: () => void): void {
	it(title, () => {
		assert.equal(CompressorFactory.compressorCount, 0, "Compressor leakage across tests.");
		try {
			testFn();
		} finally {
			CompressorFactory.disposeAllCompressors();
		}
	});
}
