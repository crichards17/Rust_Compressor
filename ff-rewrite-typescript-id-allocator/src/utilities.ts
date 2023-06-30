/* eslint-disable no-bitwise */
import { v4, NIL } from "uuid";
import { SessionId, StableId, UuidString } from "./types";
import { assert } from "./copied-utils/assert";
import { NumericUuid } from "./types/identifiers";

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
function assertIsUuidString(uuidString: string): UuidString {
	assert(isUuidString(uuidString), 0x4a2 /* Expected an UuidString */);
	return uuidString;
}

/**
 * Returns true iff the given string is a valid UUID-like string of hexadecimal characters
 * 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx'
 */
function isUuidString(str: string): str is UuidString {
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
 * Generate a random session ID
 */
export function createSessionId(): SessionId {
	return assertIsStableId(v4()) as SessionId;
}

/**
 * Asserts that the given string is a stable ID.
 */
function assertIsStableId(stableId: string): StableId {
	assert(isStableId(stableId), 0x4a3 /* Expected a StableId */);
	return stableId;
}

/**
 * Asserts that the given string is a stable ID.
 */
export function assertIsSessionId(stableId: string): SessionId {
	assert(isStableId(stableId), 0x4a3 /* Expected a StableId */);
	return stableId as SessionId;
}

/**
 * Returns true iff the given string is a valid Version 4, variant 2 UUID
 * 'xxxxxxxx-xxxx-4xxx-vxxx-xxxxxxxxxxxx'
 */
function isStableId(str: string): str is StableId {
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

export function isNaN(num: any): boolean {
	return Object.is(num, Number.NaN);
}

export function uuidStringFromBytes(uuidBytes: Uint8Array | undefined): string | undefined {
	if (uuidBytes === undefined) {
		return undefined;
	}
	let uuidString = "";
	for (let i = 0; i < 36; i++) {
		uuidString += String.fromCharCode(uuidBytes[i]);
	}
	return uuidString;
}

// xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
const versionMask = 0x4n << (19n * 4n); // Version 4
const variantMask = 0x8n << (15n * 4n); // Variant RFC4122 (1 0 x x)
const upperMask = 0xffffffffffffn << (20n * 4n);
// Upper mask when version/variant bits are removed
const strippedUpperMask = upperMask >> 6n;
const middieBittiesMask = 0xfffn << (16n * 4n);
// Middie mask when version/variant bits are removed
const strippedMiddieBittiesMask = middieBittiesMask >> 2n;
// Note: leading character should be 3 to mask at 0011
// The more-significant half of the N nibble is used to denote the variant (10xx)
const lowerMask = 0x3fffffffffffffffn;
// Used to help with stringifying bigints which would otherwise drop trailing zeros
const precisionMask = 0x1n << 128n;

export function numericUuidFromStableId(stableId: StableId): NumericUuid {
	const uuidU128 = BigInt(`0x${stableId.replace(/-/g, "")}`);
	const upperMasked = uuidU128 & upperMask;
	const middieBittiesMasked = uuidU128 & middieBittiesMask;
	const lowerMasked = uuidU128 & lowerMask;

	const upperMaskedPlaced = upperMasked >> 6n;
	const middieBittiesMaskedPlaced = middieBittiesMasked >> 2n;

	const id = upperMaskedPlaced | middieBittiesMaskedPlaced | lowerMasked;
	return id as NumericUuid;
}

export function stableIdFromNumericUuid(numericUuid: NumericUuid): StableId {
	// bitwise reverse transform
	const upperMasked = (numericUuid & strippedUpperMask) << 6n;
	const middieBittiesMasked = (numericUuid & strippedMiddieBittiesMask) << 2n;
	const lowerMasked = numericUuid & lowerMask;
	const uuidU128 =
		precisionMask | upperMasked | versionMask | middieBittiesMasked | variantMask | lowerMasked;
	const uuidString = uuidU128.toString(16).substring(1);
	return `${uuidString.substring(0, 8)}-${uuidString.substring(8, 12)}-${uuidString.substring(
		12,
		16,
	)}-${uuidString.substring(16, 20)}-${uuidString.substring(20, 32)}` as StableId;
}

export function offsetNumericUuid(numericUuid: NumericUuid, offset: number): NumericUuid {
	return ((numericUuid as bigint) + BigInt(offset)) as NumericUuid;
}

export function subtractNumericUuids(a: NumericUuid, b: NumericUuid): NumericUuid {
	return (a - b) as NumericUuid;
}

export function addNumericUuids(a: NumericUuid, b: NumericUuid): NumericUuid {
	// eslint-disable-next-line @typescript-eslint/restrict-plus-operands
	return (a + b) as NumericUuid;
}
