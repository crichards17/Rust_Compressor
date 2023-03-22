/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

import { strict as assert } from "assert";
import { TestOnly } from "wasm-id-allocator";
import { IdCompressor } from "../../src/IdCompressor";
import {
	CompressedId,
	FinalCompressedId,
	LocalCompressedId,
	OpSpaceCompressedId,
	StableId,
} from "../../src/types";

/**
 * Check if the given value is defined using mocha's `expect`. Return the defined value;
 */
export function expectDefined<T>(value: T | undefined): T {
	assert.notStrictEqual(value, undefined);
	return value as T;
}

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

export function convertToGenCount(id: LocalCompressedId): number {
	assert(id < 0);
	return -id;
}

export function convertToUnackedLocalId(genCount: number): LocalCompressedId & OpSpaceCompressedId {
	assert(genCount > 0);
	return -genCount as LocalCompressedId & OpSpaceCompressedId;
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
 * Only for use in tests. Always returns false if underlying WASM is built in release.
 */
export function compressorEquals(
	a: ReadonlyIdCompressor,
	b: ReadonlyIdCompressor,
	compareLocalState: boolean /* TODO add local state comparison */,
): boolean {
	return TestOnly.compressor_equals((a as any).wasmCompressor, (b as any).wasmCompressor);
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
