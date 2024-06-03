import { Serializer } from '../../bcs/serializer.mjs';
import { Hex } from '../../core/hex.mjs';
import '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/common.mjs';

interface TransactionArgument extends EntryFunctionArgument, ScriptFunctionArgument {
}
interface EntryFunctionArgument {
    /**
     * Serialize an argument to BCS-serialized bytes.
     */
    serialize(serializer: Serializer): void;
    /**
     * Serialize an argument as a type-agnostic, fixed byte sequence. The byte sequence contains
     * the number of the following bytes followed by the BCS-serialized bytes for a typed argument.
     */
    serializeForEntryFunction(serializer: Serializer): void;
    bcsToBytes(): Uint8Array;
    bcsToHex(): Hex;
}
interface ScriptFunctionArgument {
    /**
     * Serialize an argument to BCS-serialized bytes.
     */
    serialize(serializer: Serializer): void;
    /**
     * Serialize an argument to BCS-serialized bytes as a type aware byte sequence.
     * The byte sequence contains an enum variant index followed by the BCS-serialized
     * bytes for a typed argument.
     */
    serializeForScriptFunction(serializer: Serializer): void;
    bcsToBytes(): Uint8Array;
    bcsToHex(): Hex;
}

export type { EntryFunctionArgument, ScriptFunctionArgument, TransactionArgument };
