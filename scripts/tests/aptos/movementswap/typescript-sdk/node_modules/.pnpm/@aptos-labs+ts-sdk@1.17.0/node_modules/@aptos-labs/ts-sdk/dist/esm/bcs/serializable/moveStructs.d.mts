import { U8, U16, U32, U64, U128, U256, Bool } from './movePrimitives.mjs';
import { Serializable, Serializer } from '../serializer.mjs';
import { Deserializer, Deserializable } from '../deserializer.mjs';
import { HexInput, AnyNumber } from '../../types/index.mjs';
import { EntryFunctionArgument, TransactionArgument } from '../../transactions/instances/transactionArgument.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';

/**
 * This class is the Aptos Typescript SDK representation of a Move `vector<T>`,
 * where `T` represents either a primitive type (`bool`, `u8`, `u64`, ...)
 * or a BCS-serializable struct itself.
 *
 * It is a BCS-serializable, array-like type that contains an array of values of type `T`,
 * where `T` is a class that implements `Serializable`.
 *
 * The purpose of this class is to facilitate easy construction of BCS-serializable
 * Move `vector<T>` types.
 *
 * @example
 * // in Move: `vector<u8> [1, 2, 3, 4];`
 * const vecOfU8s = new MoveVector<U8>([new U8(1), new U8(2), new U8(3), new U8(4)]);
 * // in Move: `std::bcs::to_bytes(vector<u8> [1, 2, 3, 4]);`
 * const bcsBytes = vecOfU8s.toUint8Array();
 *
 * // vector<vector<u8>> [ vector<u8> [1], vector<u8> [1, 2, 3, 4], vector<u8> [5, 6, 7, 8] ];
 * const vecOfVecs = new MoveVector<MoveVector<U8>>([
 *   new MoveVector<U8>([new U8(1)]),
 *   MoveVector.U8([1, 2, 3, 4]),
 *   MoveVector.U8([5, 6, 7, 8]),
 * ]);
 *
 * // vector<Option<u8>> [ std::option::some<u8>(1), std::option::some<u8>(2) ];
 * const vecOfOptionU8s = new MoveVector<MoveOption<U8>>([
 *    MoveOption.U8(1),
 *    MoveOption.U8(2),
 * ]);
 *
 * // vector<MoveString> [ std::string::utf8(b"hello"), std::string::utf8(b"world") ];
 * const vecOfStrings = new MoveVector([new MoveString("hello"), new MoveString("world")]);
 * const vecOfStrings2 = MoveVector.MoveString(["hello", "world"]);
 *
 * @params
 * values: an Array<T> of values where T is a class that implements Serializable
 * @returns a `MoveVector<T>` with the values `values`
 */
declare class MoveVector<T extends Serializable & EntryFunctionArgument> extends Serializable implements TransactionArgument {
    values: Array<T>;
    constructor(values: Array<T>);
    serializeForEntryFunction(serializer: Serializer): void;
    /**
     * NOTE: This function will only work when the inner values in the `MoveVector` are `U8`s.
     * @param serializer
     */
    serializeForScriptFunction(serializer: Serializer): void;
    /**
     * Factory method to generate a MoveVector of U8s from an array of numbers.
     *
     * @example
     * const v = MoveVector.U8([1, 2, 3, 4]);
     * @params values: an array of `numbers` to convert to U8s
     * @returns a `MoveVector<U8>`
     */
    static U8(values: Array<number> | HexInput): MoveVector<U8>;
    /**
     * Factory method to generate a MoveVector of U16s from an array of numbers.
     *
     * @example
     * const v = MoveVector.U16([1, 2, 3, 4]);
     * @params values: an array of `numbers` to convert to U16s
     * @returns a `MoveVector<U16>`
     */
    static U16(values: Array<number>): MoveVector<U16>;
    /**
     * Factory method to generate a MoveVector of U32s from an array of numbers.
     *
     * @example
     * const v = MoveVector.U32([1, 2, 3, 4]);
     * @params values: an array of `numbers` to convert to U32s
     * @returns a `MoveVector<U32>`
     */
    static U32(values: Array<number>): MoveVector<U32>;
    /**
     * Factory method to generate a MoveVector of U64s from an array of numbers or bigints.
     *
     * @example
     * const v = MoveVector.U64([1, 2, 3, 4]);
     * @params values: an array of numbers of type `number | bigint` to convert to U64s
     * @returns a `MoveVector<U64>`
     */
    static U64(values: Array<AnyNumber>): MoveVector<U64>;
    /**
     * Factory method to generate a MoveVector of U128s from an array of numbers or bigints.
     *
     * @example
     * const v = MoveVector.U128([1, 2, 3, 4]);
     * @params values: an array of numbers of type `number | bigint` to convert to U128s
     * @returns a `MoveVector<U128>`
     */
    static U128(values: Array<AnyNumber>): MoveVector<U128>;
    /**
     * Factory method to generate a MoveVector of U256s from an array of numbers or bigints.
     *
     * @example
     * const v = MoveVector.U256([1, 2, 3, 4]);
     * @params values: an array of numbers of type `number | bigint` to convert to U256s
     * @returns a `MoveVector<U256>`
     */
    static U256(values: Array<AnyNumber>): MoveVector<U256>;
    /**
     * Factory method to generate a MoveVector of Bools from an array of booleans.
     *
     * @example
     * const v = MoveVector.Bool([true, false, true, false]);
     * @params values: an array of `bools` to convert to Bools
     * @returns a `MoveVector<Bool>`
     */
    static Bool(values: Array<boolean>): MoveVector<Bool>;
    /**
     * Factory method to generate a MoveVector of MoveStrings from an array of strings.
     *
     * @example
     * const v = MoveVector.MoveString(["hello", "world"]);
     * @params values: an array of `strings` to convert to MoveStrings
     * @returns a `MoveVector<MoveString>`
     */
    static MoveString(values: Array<string>): MoveVector<MoveString>;
    serialize(serializer: Serializer): void;
    /**
     * Deserialize a MoveVector of type T, specifically where T is a Serializable and Deserializable type.
     *
     * NOTE: This only works with a depth of one. Generics will not work.
     *
     * NOTE: This will not work with types that aren't of the Serializable class.
     *
     * If you're looking for a more flexible deserialization function, you can use the deserializeVector function
     * in the Deserializer class.
     *
     * @example
     * const vec = MoveVector.deserialize(deserializer, U64);
     * @params deserializer: the Deserializer instance to use, with bytes loaded into it already.
     * cls: the class to typecast the input values to, must be a Serializable and Deserializable type.
     * @returns a MoveVector of the corresponding class T
     * *
     */
    static deserialize<T extends Serializable & EntryFunctionArgument>(deserializer: Deserializer, cls: Deserializable<T>): MoveVector<T>;
}
declare class MoveString extends Serializable implements TransactionArgument {
    value: string;
    constructor(value: string);
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): MoveString;
}
declare class MoveOption<T extends Serializable & EntryFunctionArgument> extends Serializable implements EntryFunctionArgument {
    private vec;
    readonly value?: T;
    constructor(value?: T | null);
    serializeForEntryFunction(serializer: Serializer): void;
    /**
     * Retrieves the inner value of the MoveOption.
     *
     * This method is inspired by Rust's `Option<T>.unwrap()`.
     * In Rust, attempting to unwrap a `None` value results in a panic.
     *
     * Similarly, this method will throw an error if the value is not present.
     *
     * @example
     * const option = new MoveOption<Bool>(new Bool(true));
     * const value = option.unwrap();  // Returns the Bool instance
     *
     * @throws {Error} Throws an error if the MoveOption does not contain a value.
     *
     * @returns {T} The contained value if present.
     */
    unwrap(): T;
    isSome(): boolean;
    serialize(serializer: Serializer): void;
    /**
     * Factory method to generate a MoveOption<U8> from a `number` or `undefined`.
     *
     * @example
     * MoveOption.U8(1).isSome() === true;
     * MoveOption.U8().isSome() === false;
     * MoveOption.U8(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U8> with an inner value `value`
     */
    static U8(value?: number | null): MoveOption<U8>;
    /**
     * Factory method to generate a MoveOption<U16> from a `number` or `undefined`.
     *
     * @example
     * MoveOption.U16(1).isSome() === true;
     * MoveOption.U16().isSome() === false;
     * MoveOption.U16(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U16> with an inner value `value`
     */
    static U16(value?: number | null): MoveOption<U16>;
    /**
     * Factory method to generate a MoveOption<U32> from a `number` or `undefined`.
     *
     * @example
     * MoveOption.U32(1).isSome() === true;
     * MoveOption.U32().isSome() === false;
     * MoveOption.U32(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U32> with an inner value `value`
     */
    static U32(value?: number | null): MoveOption<U32>;
    /**
     * Factory method to generate a MoveOption<U64> from a `number` or a `bigint` or `undefined`.
     *
     * @example
     * MoveOption.U64(1).isSome() === true;
     * MoveOption.U64().isSome() === false;
     * MoveOption.U64(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U64> with an inner value `value`
     */
    static U64(value?: AnyNumber | null): MoveOption<U64>;
    /**
     * Factory method to generate a MoveOption<U128> from a `number` or a `bigint` or `undefined`.
     *
     * @example
     * MoveOption.U128(1).isSome() === true;
     * MoveOption.U128().isSome() === false;
     * MoveOption.U128(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U128> with an inner value `value`
     */
    static U128(value?: AnyNumber | null): MoveOption<U128>;
    /**
     * Factory method to generate a MoveOption<U256> from a `number` or a `bigint` or `undefined`.
     *
     * @example
     * MoveOption.U256(1).isSome() === true;
     * MoveOption.U256().isSome() === false;
     * MoveOption.U256(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<U256> with an inner value `value`
     */
    static U256(value?: AnyNumber | null): MoveOption<U256>;
    /**
     * Factory method to generate a MoveOption<Bool> from a `boolean` or `undefined`.
     *
     * @example
     * MoveOption.Bool(true).isSome() === true;
     * MoveOption.Bool().isSome() === false;
     * MoveOption.Bool(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<Bool> with an inner value `value`
     */
    static Bool(value?: boolean | null): MoveOption<Bool>;
    /**
     * Factory method to generate a MoveOption<MoveString> from a `string` or `undefined`.
     *
     * @example
     * MoveOption.MoveString("hello").isSome() === true;
     * MoveOption.MoveString("").isSome() === true;
     * MoveOption.MoveString().isSome() === false;
     * MoveOption.MoveString(undefined).isSome() === false;
     * @params value: the value used to fill the MoveOption. If `value` is undefined
     * the resulting MoveOption's .isSome() method will return false.
     * @returns a MoveOption<MoveString> with an inner value `value`
     */
    static MoveString(value?: string | null): MoveOption<MoveString>;
    static deserialize<U extends Serializable & EntryFunctionArgument>(deserializer: Deserializer, cls: Deserializable<U>): MoveOption<U>;
}

export { MoveOption, MoveString, MoveVector };
