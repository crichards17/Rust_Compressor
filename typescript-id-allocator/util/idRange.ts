// COPIED FROM FLUID FRAMEWORK

import type { IdCreationRange, UnackedLocalId } from "../types/persisted-types";
import { assert } from "./utilities";

export function getIds(
	range: IdCreationRange,
):
	| { first: UnackedLocalId; last: UnackedLocalId; overrides?: IdCreationRange.Overrides }
	| undefined {
	const { ids } = range;
	if (ids === undefined) {
		return undefined;
	}

	let first = ids.first;
	let last = ids.last;

	const overrides = ids as Partial<IdCreationRange.HasOverrides>;
	if (overrides.overrides !== undefined) {
		first ??= overrides.overrides[0][0];
		last ??= overrides.overrides[overrides.overrides.length - 1][0];
	}

	assert(first !== undefined && last !== undefined, 0x49b /* malformed IdCreationRange */);

	return {
		first,
		last,
		overrides: overrides.overrides,
	};
}
