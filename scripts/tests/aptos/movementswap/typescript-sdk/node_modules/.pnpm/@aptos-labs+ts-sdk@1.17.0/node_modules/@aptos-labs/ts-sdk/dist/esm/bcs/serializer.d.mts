import { Hex } from '../core/hex.mjs';
import { Uint8, Uint16, Uint32, AnyNumber } from '../types/index.mjs';
import '../core/common.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

declare abstract class Serializable {
    abstract serialize(serializer: Serializer): void;
    /**
     * Serializes a `Serializable` value to its BCS representation.
     * This function is the Typescript SDK equivalent of `bcs::to_bytes` in Move.
     * @returns the BCS representation of the Serializable instance as a byte buffer
     */
    bcsToBytes(): Uint8Array;
    /**
     * Helper function to get a value's BCS-serialized bytes as a Hex instance.
     * @returns a Hex instance with the BCS-serialized bytes loaded into its underlying Uint8Array
     */
    bcsToHex(): Hex;
}
declare class Serializer {
    private buffer;
    private offset;
    constructor(length?: number);
    private ensureBufferWillHandleSize;
    protected appendToBuffer(values: Uint8Array): void;
    private serializeWithFunction;
    /**
     * Serializes a string. UTF8 string is supported.
     *
     * The number of bytes in the string content is serialized first, as a uleb128-encoded u32 integer.
     * Then the string content is serialized as UTF8 encoded bytes.
     *
     * BCS layout for "string": string_length | string_content
     * where string_length is a u32 integer encoded as a uleb128 integer, equal to the number of bytes in string_content.
     *
     * @example
     * ```ts
     * const serializer = new Serializer();
     * serializer.serializeStr("1234abcd");
     * assert(serializer.toUint8Array() === new Uint8Array([8, 49, 50, 51, 52, 97, 98, 99, 100]));
     * ```
     */
    serializeStr(value: string): void;
    /**
     * Serializes an array of bytes.
     *
     * BCS layout for "bytes": bytes_length | bytes
     * where bytes_length is a u32 integer encoded as a uleb128 integer, equal to the length of the bytes array.
     */
    serializeBytes(value: Uint8Array): void;
    /**
     * Serializes an array of bytes with known length. Therefore, length doesn't need to be
     * serialized to help deserialization.
     *
     * When deserializing, the number of bytes to deserialize needs to be passed in.
     */
    serializeFixedBytes(value: Uint8Array): void;
    /**
     * Serializes a boolean value.
     *
     * BCS layout for "boolean": One byte. "0x01" for true and "0x00" for false.
     */
    serializeBool(value: boolean): void;
    /**
     * Serializes a uint8 number.
     *
     * BCS layout for "uint8": One byte. Binary format in little-endian representation.
     */
    serializeU8(value: Uint8): void;
    /**
     * Serializes a uint16 number.
     *
     * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const serializer = new Serializer();
     * serializer.serializeU16(4660);
     * assert(serializer.toUint8Array() === new Uint8Array([0x34, 0x12]));
     * ```
     */
    serializeU16(value: Uint16): void;
    /**
     * Serializes a uint32 number.
     *
     * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const serializer = new Serializer();
     * serializer.serializeU32(305419896);
     * assert(serializer.toUint8Array() === new Uint8Array([0x78, 0x56, 0x34, 0x12]));
     * ```
     */
    serializeU32(value: Uint32): void;
    /**
     * Serializes a uint64 number.
     *
     * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const serializer = new Serializer();
     * serializer.serializeU64(1311768467750121216);
     * assert(serializer.toUint8Array() === new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
     * ```
     */
    serializeU64(value: AnyNumber): void;
    /**
     * Serializes a uint128 number.
     *
     * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
     */
    serializeU128(value: AnyNumber): void;
    /**
     * Serializes a uint256 number.
     *
     * BCS layout for "uint256": Sixteen bytes. Binary format in little-endian representation.
     */
    serializeU256(value: AnyNumber): void;
    /**
     * Serializes a uint32 number with uleb128.
     *
     * BCS uses uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
     */
    serializeU32AsUleb128(val: Uint32): void;
    /**
     * Returns the buffered bytes
     */
    toUint8Array(): Uint8Array;
    /**
     * Serializes a `Serializable` value, facilitating composable serialization.
     *
     * @param value The Serializable value to serialize
     *
     * @example
     * // Define the MoveStruct class that implements the Serializable interface
     * class MoveStruct extends Serializable {
     *     constructor(
     *         public creatorAddress: AccountAddress, // where AccountAddress extends Serializable
     *         public collectionName: string,
     *         public tokenName: string
     *     ) {}
     *
     *     serialize(serializer: Serializer): void {
     *         serializer.serialize(this.creatorAddress);  // Composable serialization of another Serializable object
     *         serializer.serializeStr(this.collectionName);
     *         serializer.serializeStr(this.tokenName);
     *     }
     * }
     *
     * // Construct a MoveStruct
     * const moveStruct = new MoveStruct(new AccountAddress(...), "MyCollection", "TokenA");
     *
     * // Serialize a string, a u64 number, and a MoveStruct instance.
     * const serializer = new Serializer();
     * serializer.serializeStr("ExampleString");
     * serializer.serializeU64(12345678);
     * serializer.serialize(moveStruct);
     *
     * // Get the bytes from the Serializer instance
     * const serializedBytes = serializer.toUint8Array();
     *
     * @returns the serializer instance
     */
    serialize<T extends Serializable>(value: T): void;
    /**
     * Serializes an array of BCS Serializable values to a serializer instance.
     * Note that this does not return anything. The bytes are added to the serializer instance's byte buffer.
     *
     * @param values The array of BCS Serializable values
     * @example
     * const addresses = new Array<AccountAddress>(
     *   AccountAddress.from("0x1"),
     *   AccountAddress.from("0x2"),
     *   AccountAddress.from("0xa"),
     *   AccountAddress.from("0xb"),
     * );
     * const serializer = new Serializer();
     * serializer.serializeVector(addresses);
     * const serializedBytes = serializer.toUint8Array();
     * // serializedBytes is now the BCS-serialized bytes
     * // The equivalent value in Move would be:
     * // `bcs::to_bytes(&vector<address> [@0x1, @0x2, @0xa, @0xb])`;
     */
    serializeVector<T extends Serializable>(values: Array<T>): void;
}
declare function ensureBoolean(value: unknown): asserts value is boolean;
declare const outOfRangeErrorMessage: (value: AnyNumber, min: AnyNumber, max: AnyNumber) => string;
declare function validateNumberInRange<T extends AnyNumber>(value: T, minValue: T, maxValue: T): void;

export { Serializable, Serializer, ensureBoolean, outOfRangeErrorMessage, validateNumberInRange };
