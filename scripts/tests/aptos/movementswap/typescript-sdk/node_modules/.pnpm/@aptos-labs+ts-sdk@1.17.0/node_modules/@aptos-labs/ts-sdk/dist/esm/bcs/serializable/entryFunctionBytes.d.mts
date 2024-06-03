import { Serializable, Serializer } from '../serializer.mjs';
import { Deserializer } from '../deserializer.mjs';
import { FixedBytes } from './fixedBytes.mjs';
import { EntryFunctionArgument } from '../../transactions/instances/transactionArgument.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';
import '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';

/**
 * This class exists solely to represent a sequence of fixed bytes as a serialized entry function, because
 * serializing an entry function appends a prefix that's *only* used for entry function arguments.
 *
 * NOTE: Attempting to use this class for a serialized script function will result in erroneous
 * and unexpected behavior.
 *
 * If you wish to convert this class back to a TransactionArgument, you must know the type
 * of the argument beforehand, and use the appropriate class to deserialize the bytes within
 * an instance of this class.
 */
declare class EntryFunctionBytes extends Serializable implements EntryFunctionArgument {
    readonly value: FixedBytes;
    private constructor();
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    /**
     * The only way to create an instance of this class is to use this static method.
     *
     * This function should only be used when deserializing a sequence of EntryFunctionPayload arguments.
     * @param deserializer the deserializer instance with the buffered bytes
     * @param length the length of the bytes to deserialize
     * @returns an instance of this class, which will now only be usable as an EntryFunctionArgument
     */
    static deserialize(deserializer: Deserializer, length: number): EntryFunctionBytes;
}

export { EntryFunctionBytes };
