import { Serializable, Serializer } from '../serializer.mjs';
import { Deserializer } from '../deserializer.mjs';
import { HexInput } from '../../types/index.mjs';
import { TransactionArgument } from '../../transactions/instances/transactionArgument.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';

/**
 *  This class exists to represent a contiguous sequence of already serialized BCS-bytes.
 *
 *  It differs from most other Serializable classes in that its internal byte buffer is serialized to BCS
 *  bytes exactly as-is, without prepending the length of the bytes.
 *
 *  If you want to write your own serialization function and pass the bytes as a transaction argument,
 *  you should use this class.
 *
 *  This class is also more generally used to represent type-agnostic BCS bytes as a vector<u8>.
 *
 *  An example of this is the bytes resulting from entry function arguments that have been serialized
 *  for an entry function.
 *
 *  @example
 *  const yourCustomSerializedBytes = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
 *  const fixedBytes = new FixedBytes(yourCustomSerializedBytes);
 *  const payload = await generateTransactionPayload({
 *    function: "0xbeefcafe::your_module::your_function_that_requires_custom_serialization",
 *    functionArguments: [yourCustomBytes],
 *  });
 *
 *  For example, if you store each of the 32 bytes for an address as a U8 in a MoveVector<U8>, when you
 *  serialize that MoveVector<U8>, it will be serialized to 33 bytes. If you solely want to pass around
 *  the 32 bytes as a Serializable class that *does not* prepend the length to the BCS-serialized representation,
 *  use this class.
 *
 * @params value: HexInput representing a sequence of Uint8 bytes
 * @returns a Serializable FixedBytes instance, which when serialized, does not prepend the length of the bytes
 * @see EntryFunctionBytes
 */
declare class FixedBytes extends Serializable implements TransactionArgument {
    value: Uint8Array;
    constructor(value: HexInput);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer, length: number): FixedBytes;
}

export { FixedBytes };
