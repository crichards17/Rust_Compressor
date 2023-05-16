/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

import { IdCompressor } from "../../../src/IdCompressor";
import { SessionSpaceCompressedId, StableId, OpSpaceCompressedId } from "../../../src/types";
import { uuidStringFromBytes } from "../../../src/utilities";

/**
 * An identifier (v4 UUID) that has been shortened by a distributed compression algorithm.
 * Lacks a space (session/op), meaning its scope is the same as the space-specific ID from which it was derived.
 * @alpha
 */
export type CompressedId =
	| SessionSpaceCompressedId
	| OpSpaceCompressedId
	| FinalCompressedId
	| LocalCompressedId;

/**
 * A compressed ID that is stable and unique within the scope of network of compressors (i.e. a document).
 * It can only be used/decompressed in the context of the originating document.
 * @alpha
 */
export type FinalCompressedId = number & {
	readonly FinalCompressedId: "5d83d1e2-98b7-4e4e-a889-54c855cfa73d";

	// Same brand as OpNormalizedCompressedId, as final IDs are always finally normalized
	readonly OpNormalized: "9209432d-a959-4df7-b2ad-767ead4dbcae";
};

/**
 * A compressed ID that is local to a session (can only be decompressed when paired with a SessionId).
 * Internally, it should not be persisted outside a scope annotated with the originating SessionId in order to be unambiguous.
 * If external persistence is needed (e.g. by a client), a StableId should be used instead.
 * @alpha
 */
export type LocalCompressedId = number & {
	readonly LocalCompressedId: "6fccb42f-e2a4-4243-bd29-f13d12b9c6d1";
} & SessionSpaceCompressedId; // Same brand as CompressedId, as local IDs are always locally normalized

/**
 * @returns true if the supplied ID is a final ID.
 */
export function isFinalId(id: CompressedId): id is FinalCompressedId {
	return id >= 0;
}

/**
 * @returns true if the supplied ID is a local ID.
 */
export function isLocalId(id: CompressedId): id is LocalCompressedId {
	return id < 0;
}

/**
 * Remove `readonly` from all fields.
 */
export type Mutable<T> = { -readonly [P in keyof T]: T[P] };

/**
 * Retrieve a value from a map with the given key, or create a new entry if the key is not in the map.
 * @param map - The map to query/update
 * @param key - The key to lookup in the map
 * @param defaultValue - a function which returns a default value. This is called and used to set an initial value for the given key in the map if none exists
 * @returns either the existing value for the given key, or the newly-created value (the result of `defaultValue`)
 */
export function getOrCreate<K, V>(map: Map<K, V>, key: K, defaultValue: (key: K) => V): V {
	let value = map.get(key);
	if (value === undefined) {
		value = defaultValue(key);
		map.set(key, value);
	}
	return value;
}

export function incrementStableId(stableId: StableId, offset: number): StableId {
	// eslint-disable-next-line @typescript-eslint/no-require-imports, @typescript-eslint/no-var-requires
	const increment = require("wasm-id-allocator").increment_uuid;
	if (increment !== undefined) {
		return uuidStringFromBytes(increment(stableId, offset)) as StableId;
	} else {
		throw new EvalError("increment_uuid is not available in release build.");
	}
}

/**
 * Only for use in tests.
 */
export function compressorEquals(
	a: ReadonlyIdCompressor,
	b: ReadonlyIdCompressor,
	compareLocalState: boolean,
): boolean {
	// eslint-disable-next-line @typescript-eslint/no-require-imports, @typescript-eslint/no-var-requires
	const equals = require("wasm-id-allocator").compressor_equals;
	if (equals !== undefined) {
		// eslint-disable-next-line @typescript-eslint/no-unsafe-return
		return equals((a as any).wasmCompressor, (b as any).wasmCompressor, compareLocalState);
	} else {
		throw new EvalError("compressor_equals is not available in release build.");
	}
}

/** An immutable view of an `IdCompressor` */
export interface ReadonlyIdCompressor
	extends Omit<
		IdCompressor,
		| "generateCompressedId"
		| "generateCompressedIdRange"
		| "takeNextCreationRange"
		| "finalizeCreationRange"
	> {
	readonly clusterCapacity: number;
}
