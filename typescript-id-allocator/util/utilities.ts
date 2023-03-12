// COPIED FROM FLUID FRAMEWORK

import { StableId, UuidString } from "../types";
import { v4, NIL } from "uuid";

const hexadecimalCharCodes = Array.from("09afAF").map((c) => c.charCodeAt(0)) as [
	zero: number,
	nine: number,
	a: number,
	f: number,
	A: number,
	F: number,
];

function isHexadecimalCharacter(charCode: number): boolean {
	return (
		(charCode >= hexadecimalCharCodes[0] && charCode <= hexadecimalCharCodes[1]) ||
		(charCode >= hexadecimalCharCodes[2] && charCode <= hexadecimalCharCodes[3]) ||
		(charCode >= hexadecimalCharCodes[4] && charCode <= hexadecimalCharCodes[5])
	);
}

/** The null (lowest/all-zeros) UUID */
export const nilUuid = assertIsUuidString(NIL);

/**
 * Asserts that the given string is a UUID
 */
export function assertIsUuidString(uuidString: string): UuidString {
	assert(isUuidString(uuidString), 0x4a2 /* Expected an UuidString */);
	return uuidString;
}

/**
 * Returns true iff the given string is a valid UUID-like string of hexadecimal characters
 * 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx'
 */
export function isUuidString(str: string): str is UuidString {
	for (let i = 0; i < str.length; i++) {
		switch (i) {
			case 8:
			case 13:
			case 18:
			case 23:
				if (str.charAt(i) !== "-") {
					return false;
				}
				break;

			default:
				if (!isHexadecimalCharacter(str.charCodeAt(i))) {
					return false;
				}
				break;
		}
	}

	return true;
}

/**
 * Generate a random stable ID
 */
export function generateStableId(): StableId {
	return assertIsStableId(v4());
}

/**
 * Asserts that the given string is a stable ID.
 */
export function assertIsStableId(stableId: string): StableId {
	assert(isStableId(stableId), 0x4a3 /* Expected a StableId */);
	return stableId;
}

/**
 * Returns true iff the given string is a valid Version 4, variant 2 UUID
 * 'xxxxxxxx-xxxx-4xxx-vxxx-xxxxxxxxxxxx'
 */
export function isStableId(str: string): str is StableId {
	if (str.length !== 36) {
		return false;
	}

	for (let i = 0; i < str.length; i++) {
		switch (i) {
			case 8:
			case 13:
			case 18:
			case 23:
				if (str.charAt(i) !== "-") {
					return false;
				}
				break;

			case 14:
				if (str.charAt(i) !== "4") {
					return false;
				}
				break;

			case 19: {
				const char = str.charAt(i);
				if (char !== "8" && char !== "9" && char !== "a" && char !== "b") {
					return false;
				}
				break;
			}

			default:
				if (!isHexadecimalCharacter(str.charCodeAt(i))) {
					return false;
				}
				break;
		}
	}

	return true;
}

/**
 * A browser friendly assert library.
 * Use this instead of the 'assert' package, which has a big impact on bundle sizes.
 * @param condition - The condition that should be true, if the condition is false an error will be thrown.
 * Only use this API when `false` indicates a logic error in the problem and thus a bug that should be fixed.
 * @param message - The message to include in the error when the condition does not hold.
 * A number should not be specified manually: use a string.
 * Before a release, policy-check should be run, which will convert any asserts still using strings to
 * use numbered error codes instead.
 */
export function assert(condition: boolean, message: string | number): asserts condition {
	if (!condition) {
		throw new Error(
			typeof message === "number" ? `0x${message.toString(16).padStart(3, "0")}` : message,
		);
	}
}

export function fail(message: string): never {
	throw new Error(message);
}
