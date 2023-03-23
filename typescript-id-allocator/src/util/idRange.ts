import { assert } from "../copied-utils";
import type { IdCreationRange } from "../types/persisted-types";

export function getIds(range: IdCreationRange):
	| {
			firstGenCount: number;
			lastGenCount: number;
			overrides?: IdCreationRange.Overrides;
	  }
	| undefined {
	const { ids } = range;
	if (ids === undefined) {
		return undefined;
	}

	let first = ids.firstGenCount;
	let last = ids.lastGenCount;

	const overrides = ids as Partial<IdCreationRange.HasOverrides>;
	if (overrides.overrides !== undefined) {
		first ??= overrides.overrides[0][0];
		last ??= overrides.overrides[overrides.overrides.length - 1][0];
	}

	assert(first !== undefined && last !== undefined, 0x49b /* malformed IdCreationRange */);

	return {
		firstGenCount: first,
		lastGenCount: last,
		overrides: overrides.overrides,
	};
}
