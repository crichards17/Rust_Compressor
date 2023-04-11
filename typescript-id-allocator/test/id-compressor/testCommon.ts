/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

import { TestOnly } from "wasm-id-allocator";
import { IdCompressor } from "../../src/IdCompressor";
import { CompressedId, FinalCompressedId, LocalCompressedId, StableId } from "../../src/types";

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
	return TestOnly.increment_uuid(stableId, offset) as StableId;
}

/**
 * Only for use in tests.
 */
export function compressorEquals(
	a: ReadonlyIdCompressor,
	b: ReadonlyIdCompressor,
	compareLocalState: boolean,
): boolean {
	return TestOnly.compressor_equals(
		(a as any).wasmCompressor,
		(b as any).wasmCompressor,
		compareLocalState,
	);
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
