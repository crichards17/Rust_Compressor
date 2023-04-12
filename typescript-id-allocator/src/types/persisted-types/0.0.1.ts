import type { SessionId } from "../identifiers";

/**
 * The version of IdCompressor that is currently persisted.
 */
export const currentWrittenVersion = "0.0.1";

/**
 * The minimal required contents of a serialized IdCompressor.
 */
export interface VersionedSerializedIdCompressor {
	readonly _versionedSerializedIdCompressor: "8c73c57c-1cf4-4278-8915-6444cb4f6af5";
	readonly version: string;
}

/**
 * The serialized contents of an IdCompressor, suitable for persistence in a summary.
 */
export interface SerializedIdCompressor extends VersionedSerializedIdCompressor {
	readonly bytes: Uint8Array;
}

/**
 * The serialized contents of an IdCompressor, suitable for persistence in a summary.
 */
export interface SerializedIdCompressorWithNoSession extends SerializedIdCompressor {
	readonly _noLocalState: "3aa2e1e8-cc28-4ea7-bc1a-a11dc3f26dfb";
}

/**
 * The serialized contents of an IdCompressor, suitable for persistence in a summary.
 */
export interface SerializedIdCompressorWithOngoingSession extends SerializedIdCompressor {
	readonly _hasLocalState: "1281acae-6d14-47e7-bc92-71c8ee0819cb";
}

/**
 * Data describing a range of session-local IDs (from a remote or local session).
 *
 * A range is composed of local IDs that were generated.
 */
export interface IdCreationRange {
	readonly sessionId: SessionId;
	readonly ids?: {
		readonly firstGenCount: number;
		readonly count: number;
	};
}
