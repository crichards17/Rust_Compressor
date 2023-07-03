import { AppendOnlySortedMap } from "./appendOnlySortedMap";
import { LocalCompressedId } from "./test/id-compressor/testCommon";
import { compareFiniteNumbersReversed } from "./utilities";

export class SessionSpaceNormalizer {
	private readonly leadingLocals = new AppendOnlySortedMap<LocalCompressedId, number>(
		compareFiniteNumbersReversed,
	);

	public addLocalRange(baseLocal: LocalCompressedId, count: number): void {
		const last = this.leadingLocals.last();
		if (last !== undefined) {
			const [lastLocal, lastCount] = last;
			if (lastLocal - lastCount === baseLocal) {
				this.leadingLocals.replaceLast(lastLocal, lastCount + count);
				return;
			}
		}
		this.leadingLocals.append(baseLocal, count);
	}

	public contains(query: LocalCompressedId): boolean {
		const containingBlock = this.leadingLocals.getPairOrNextLower(query);
		if (containingBlock !== undefined) {
			const [startingLocal, count] = containingBlock;
			if (query >= startingLocal - (count - 1)) {
				return true;
			}
		}
		return false;
	}
}
