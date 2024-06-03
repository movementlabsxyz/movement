import { Serializable, Serializer } from '../bcs/serializer.mjs';
import { Deserializer } from '../bcs/deserializer.mjs';
import { ParsingResult } from './common.mjs';
import { TransactionArgument } from '../transactions/instances/transactionArgument.mjs';
import { HexInput } from '../types/index.mjs';
import './hex.mjs';
import '../utils/apiEndpoints.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';

/**
 * This enum is used to explain why an address was invalid.
 */
declare enum AddressInvalidReason {
    INCORRECT_NUMBER_OF_BYTES = "incorrect_number_of_bytes",
    INVALID_HEX_CHARS = "invalid_hex_chars",
    TOO_SHORT = "too_short",
    TOO_LONG = "too_long",
    LEADING_ZERO_X_REQUIRED = "leading_zero_x_required",
    LONG_FORM_REQUIRED_UNLESS_SPECIAL = "long_form_required_unless_special",
    INVALID_PADDING_ZEROES = "INVALID_PADDING_ZEROES"
}
type AccountAddressInput = HexInput | AccountAddress;
/**
 * NOTE: Only use this class for account addresses. For other hex data, e.g. transaction
 * hashes, use the Hex class.
 *
 * AccountAddress is used for working with account addresses. Account addresses, when
 * represented as a string, generally look like these examples:
 * - 0x1
 * - 0xaa86fe99004361f747f91342ca13c426ca0cccb0c1217677180c9493bad6ef0c
 *
 * Proper formatting and parsing of account addresses is defined by AIP-40.
 * To learn more about the standard, read the AIP here:
 * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
 *
 * The comments in this class make frequent reference to the LONG and SHORT formats,
 * as well as "special" addresses. To learn what these refer to see AIP-40.
 */
declare class AccountAddress extends Serializable implements TransactionArgument {
    /**
     * This is the internal representation of an account address.
     */
    readonly data: Uint8Array;
    /**
     * The number of bytes that make up an account address.
     */
    static readonly LENGTH: number;
    /**
     * The length of an address string in LONG form without a leading 0x.
     */
    static readonly LONG_STRING_LENGTH: number;
    static ZERO: AccountAddress;
    static ONE: AccountAddress;
    static TWO: AccountAddress;
    static THREE: AccountAddress;
    static FOUR: AccountAddress;
    /**
     * Creates an instance of AccountAddress from a Uint8Array.
     *
     * @param args.data A Uint8Array representing an account address.
     */
    constructor(input: Uint8Array);
    /**
     * Returns whether an address is special, where special is defined as 0x0 to 0xf
     * inclusive. In other words, the last byte of the address must be < 0b10000 (16)
     * and every other byte must be zero.
     *
     * For more information on how special addresses are defined see AIP-40:
     * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
     *
     * @returns true if the address is special, false if not.
     */
    isSpecial(): boolean;
    /**
     * Return the AccountAddress as a string as per AIP-40.
     * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
     *
     * In short, it means that special addresses are represented in SHORT form, meaning
     * 0x0 through to 0xf inclusive, and every other address is represented in LONG form,
     * meaning 0x + 64 hex characters.
     *
     * @returns AccountAddress as a string conforming to AIP-40.
     */
    toString(): `0x${string}`;
    /**
     * NOTE: Prefer to use `toString` where possible.
     *
     * Return the AccountAddress as a string as per AIP-40 but without the leading 0x.
     *
     * Learn more by reading the docstring of `toString`.
     *
     * @returns AccountAddress as a string conforming to AIP-40 but without the leading 0x.
     */
    toStringWithoutPrefix(): string;
    /**
     * NOTE: Prefer to use `toString` where possible.
     *
     * Whereas toString will format special addresses (as defined by isSpecial) using the
     * SHORT form (no leading 0s), this format the address in the LONG format
     * unconditionally.
     *
     * This means it will be 0x + 64 hex characters.
     *
     * @returns AccountAddress as a string in LONG form.
     */
    toStringLong(): `0x${string}`;
    /**
     * NOTE: Prefer to use `toString` where possible.
     *
     * Whereas toString will format special addresses (as defined by isSpecial) using the
     * SHORT form (no leading 0s), this function will include leading zeroes. The string
     * will not have a leading zero.
     *
     * This means it will be 64 hex characters without a leading 0x.
     *
     * @returns AccountAddress as a string in LONG form without a leading 0x.
     */
    toStringLongWithoutPrefix(): string;
    /**
     * Get the inner hex data. The inner data is already a Uint8Array so no conversion
     * is taking place here, it just returns the inner data.
     *
     * @returns Hex data as Uint8Array
     */
    toUint8Array(): Uint8Array;
    /**
     * Serialize the AccountAddress to a Serializer instance's data buffer.
     * @param serializer The serializer to serialize the AccountAddress to.
     * @returns void
     * @example
     * const serializer = new Serializer();
     * const address = AccountAddress.fromString("0x1");
     * address.serialize(serializer);
     * const bytes = serializer.toUint8Array();
     * // `bytes` is now the BCS-serialized address.
     */
    serialize(serializer: Serializer): void;
    serializeForEntryFunction(serializer: Serializer): void;
    serializeForScriptFunction(serializer: Serializer): void;
    /**
     * Deserialize an AccountAddress from the byte buffer in a Deserializer instance.
     * @param deserializer The deserializer to deserialize the AccountAddress from.
     * @returns An instance of AccountAddress.
     * @example
     * const bytes = hexToBytes("0x0102030405060708091011121314151617181920212223242526272829303132");
     * const deserializer = new Deserializer(bytes);
     * const address = AccountAddress.deserialize(deserializer);
     * // `address` is now an instance of AccountAddress.
     */
    static deserialize(deserializer: Deserializer): AccountAddress;
    /**
     * NOTE: This function has strict parsing behavior. For relaxed behavior, please use
     * the `fromString` function.
     *
     * Creates an instance of AccountAddress from a hex string.
     *
     * This function allows only the strictest formats defined by AIP-40. In short this
     * means only the following formats are accepted:
     *
     * - LONG
     * - SHORT for special addresses
     *
     * Where:
     * - LONG is defined as 0x + 64 hex characters.
     * - SHORT for special addresses is 0x0 to 0xf inclusive without padding zeroes.
     *
     * This means the following are not accepted:
     * - SHORT for non-special addresses.
     * - Any address without a leading 0x.
     *
     * Learn more about the different address formats by reading AIP-40:
     * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
     *
     * @param input A hex string representing an account address.
     *
     * @returns An instance of AccountAddress.
     */
    static fromStringStrict(input: string): AccountAddress;
    /**
     * NOTE: This function has relaxed parsing behavior. For strict behavior, please use
     * the `fromStringStrict` function. Where possible use `fromStringStrict` rather than this
     * function, `fromString` is only provided for backwards compatibility.
     *
     * Creates an instance of AccountAddress from a hex string.
     *
     * This function allows all formats defined by AIP-40. In short this means the
     * following formats are accepted:
     *
     * - LONG, with or without leading 0x
     * - SHORT, with or without leading 0x
     *
     * Where:
     * - LONG is 64 hex characters.
     * - SHORT is 1 to 63 hex characters inclusive.
     * - Padding zeroes are allowed, e.g. 0x0123 is valid.
     *
     * Learn more about the different address formats by reading AIP-40:
     * https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.
     *
     * @param input A hex string representing an account address.
     *
     * @returns An instance of AccountAddress.
     */
    static fromString(input: string): AccountAddress;
    /**
     * Convenience method for creating an AccountAddress from all known inputs.
     *
     * This handles, Uint8array, string, and AccountAddress itself
     * @param input
     */
    static from(input: AccountAddressInput): AccountAddress;
    /**
     * Convenience method for creating an AccountAddress from all known inputs.
     *
     * This handles, Uint8array, string, and AccountAddress itself
     * @param input
     */
    static fromStrict(input: AccountAddressInput): AccountAddress;
    /**
     * Check if the string is a valid AccountAddress.
     *
     * @param args.input A hex string representing an account address.
     * @param args.strict If true, use strict parsing behavior. If false, use relaxed parsing behavior.
     *
     * @returns valid = true if the string is valid, valid = false if not. If the string
     * is not valid, invalidReason will be set explaining why it is invalid.
     */
    static isValid(args: {
        input: AccountAddressInput;
        strict?: boolean;
    }): ParsingResult<AddressInvalidReason>;
    /**
     * Return whether AccountAddresses are equal. AccountAddresses are considered equal
     * if their underlying byte data is identical.
     *
     * @param other The AccountAddress to compare to.
     * @returns true if the AccountAddresses are equal, false if not.
     */
    equals(other: AccountAddress): boolean;
}

export { AccountAddress, type AccountAddressInput, AddressInvalidReason };
