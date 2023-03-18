/*!
 * Copyright (c) Microsoft Corporation and contributors. All rights reserved.
 * Licensed under the MIT License.
 */

import { strict as assert } from "assert";
import { CompressedId, FinalCompressedId, LocalCompressedId } from "../../types";

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

/**
 * Remove `readonly` from all fields.
 */
export type Mutable<T> = { -readonly [P in keyof T]: T[P] };
