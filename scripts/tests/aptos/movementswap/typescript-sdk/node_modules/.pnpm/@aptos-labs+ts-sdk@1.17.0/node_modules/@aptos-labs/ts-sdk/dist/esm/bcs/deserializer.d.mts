import { Uint8, Uint16, Uint32, Uint64, Uint128, Uint256 } from '../types/index.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

/**
 * This interface exists to define Deserializable<T> inputs for functions that
 * deserialize a byte buffer into a type T.
 * It is not intended to be implemented or extended, because Typescript has no support
 * for static methods in interfaces.
 */
interface Deserializable<T> {
    deserialize(deserializer: Deserializer): T;
}
declare class Deserializer {
    private buffer;
    private offset;
    constructor(data: Uint8Array);
    private read;
    /**
     * Deserializes a string. UTF8 string is supported. Reads the string's bytes length "l" first,
     * and then reads "l" bytes of content. Decodes the byte array into a string.
     *
     * BCS layout for "string": string_length | string_content
     * where string_length is a u32 integer encoded as a uleb128 integer, equal to the number of bytes in string_content.
     *
     * @example
     * ```ts
     * const deserializer = new Deserializer(new Uint8Array([8, 49, 50, 51, 52, 97, 98, 99, 100]));
     * assert(deserializer.deserializeStr() === "1234abcd");
     * ```
     */
    deserializeStr(): string;
    /**
     * Deserializes an array of bytes.
     *
     * BCS layout for "bytes": bytes_length | bytes
     * where bytes_length is a u32 integer encoded as a uleb128 integer, equal to the length of the bytes array.
     */
    deserializeBytes(): Uint8Array;
    /**
     * Deserializes an array of bytes. The number of bytes to read is already known.
     *
     */
    deserializeFixedBytes(len: number): Uint8Array;
    /**
     * Deserializes a boolean value.
     *
     * BCS layout for "boolean": One byte. "0x01" for true and "0x00" for false.
     */
    deserializeBool(): boolean;
    /**
     * Deserializes a uint8 number.
     *
     * BCS layout for "uint8": One byte. Binary format in little-endian representation.
     */
    deserializeU8(): Uint8;
    /**
     * Deserializes a uint16 number.
     *
     * BCS layout for "uint16": Two bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const deserializer = new Deserializer(new Uint8Array([0x34, 0x12]));
     * assert(deserializer.deserializeU16() === 4660);
     * ```
     */
    deserializeU16(): Uint16;
    /**
     * Deserializes a uint32 number.
     *
     * BCS layout for "uint32": Four bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const deserializer = new Deserializer(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
     * assert(deserializer.deserializeU32() === 305419896);
     * ```
     */
    deserializeU32(): Uint32;
    /**
     * Deserializes a uint64 number.
     *
     * BCS layout for "uint64": Eight bytes. Binary format in little-endian representation.
     * @example
     * ```ts
     * const deserializer = new Deserializer(new Uint8Array([0x00, 0xEF, 0xCD, 0xAB, 0x78, 0x56, 0x34, 0x12]));
     * assert(deserializer.deserializeU64() === 1311768467750121216);
     * ```
     */
    deserializeU64(): Uint64;
    /**
     * Deserializes a uint128 number.
     *
     * BCS layout for "uint128": Sixteen bytes. Binary format in little-endian representation.
     */
    deserializeU128(): Uint128;
    /**
     * Deserializes a uint256 number.
     *
     * BCS layout for "uint256": Thirty-two bytes. Binary format in little-endian representation.
     */
    deserializeU256(): Uint256;
    /**
     * Deserializes a uleb128 encoded uint32 number.
     *
     * BCS use uleb128 encoding in two cases: (1) lengths of variable-length sequences and (2) tags of enum values
     */
    deserializeUleb128AsU32(): Uint32;
    /**
     * Helper function that primarily exists to support alternative syntax for deserialization.
     * That is, if we have a `const deserializer: new Deserializer(...)`, instead of having to use
     * `MyClass.deserialize(deserializer)`, we can call `deserializer.deserialize(MyClass)`.
     *
     * @example const deserializer = new Deserializer(new Uint8Array([1, 2, 3]));
     * const value = deserializer.deserialize(MyClass); // where MyClass has a `deserialize` function
     * // value is now an instance of MyClass
     * // equivalent to `const value = MyClass.deserialize(deserializer)`
     * @param cls The BCS-deserializable class to deserialize the buffered bytes into.
     *
     * @returns the deserialized value of class type T
     */
    deserialize<T>(cls: Deserializable<T>): T;
    /**
     * Deserializes an array of BCS Deserializable values given an existing Deserializer
     * instance with a loaded byte buffer.
     *
     * @param cls The BCS-deserializable class to deserialize the buffered bytes into.
     * @example
     * // serialize a vector of addresses
     * const addresses = new Array<AccountAddress>(
     *   AccountAddress.from("0x1"),
     *   AccountAddress.from("0x2"),
     *   AccountAddress.from("0xa"),
     *   AccountAddress.from("0xb"),
     * );
     * const serializer = new Serializer();
     * serializer.serializeVector(addresses);
     * const serializedBytes = serializer.toUint8Array();
     *
     * // deserialize the bytes into an array of addresses
     * const deserializer = new Deserializer(serializedBytes);
     * const deserializedAddresses = deserializer.deserializeVector(AccountAddress);
     * // deserializedAddresses is now an array of AccountAddress instances
     * @returns an array of deserialized values of type T
     */
    deserializeVector<T>(cls: Deserializable<T>): Array<T>;
}

export { type Deserializable, Deserializer };
