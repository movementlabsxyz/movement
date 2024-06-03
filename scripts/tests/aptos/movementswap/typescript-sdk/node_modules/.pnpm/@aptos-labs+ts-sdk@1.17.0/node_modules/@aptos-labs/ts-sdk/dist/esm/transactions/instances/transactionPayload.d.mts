import { Deserializer } from '../../bcs/deserializer.mjs';
import { Serializable, Serializer } from '../../bcs/serializer.mjs';
import { AccountAddress } from '../../core/accountAddress.mjs';
import { MoveModuleId } from '../../types/index.mjs';
import { TransactionArgument, EntryFunctionArgument, ScriptFunctionArgument } from './transactionArgument.mjs';
import { Identifier } from './identifier.mjs';
import { ModuleId } from './moduleId.mjs';
import { TypeTag } from '../typeTag/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';

/**
 * Deserialize a Script Transaction Argument
 */
declare function deserializeFromScriptArgument(deserializer: Deserializer): TransactionArgument;
/**
 * Representation of the supported Transaction Payload
 * that can serialized and deserialized
 */
declare abstract class TransactionPayload extends Serializable {
    /**
     * Serialize a Transaction Payload
     */
    abstract serialize(serializer: Serializer): void;
    /**
     * Deserialize a Transaction Payload
     */
    static deserialize(deserializer: Deserializer): TransactionPayload;
}
/**
 * Representation of a Transaction Payload Script that can serialized and deserialized
 */
declare class TransactionPayloadScript extends TransactionPayload {
    readonly script: Script;
    constructor(script: Script);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): TransactionPayloadScript;
}
/**
 * Representation of a Transaction Payload Entry Function that can serialized and deserialized
 */
declare class TransactionPayloadEntryFunction extends TransactionPayload {
    readonly entryFunction: EntryFunction;
    constructor(entryFunction: EntryFunction);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): TransactionPayloadEntryFunction;
}
/**
 * Representation of a Transaction Payload Multi-sig that can serialized and deserialized
 */
declare class TransactionPayloadMultiSig extends TransactionPayload {
    readonly multiSig: MultiSig;
    constructor(multiSig: MultiSig);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): TransactionPayloadMultiSig;
}
/**
 * Representation of a EntryFunction that can serialized and deserialized
 */
declare class EntryFunction {
    readonly module_name: ModuleId;
    readonly function_name: Identifier;
    readonly type_args: Array<TypeTag>;
    readonly args: Array<EntryFunctionArgument>;
    /**
     * Contains the payload to run a function within a module.
     * @param module_name Fully qualified module name in format "account_address::module_name" e.g. "0x1::coin"
     * @param function_name The function name. e.g "transfer"
     * @param type_args Type arguments that move function requires.
     *
     * @example
     * A coin transfer function has one type argument "CoinType".
     * ```
     * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
     * ```
     * @param args arguments to the move function.
     *
     * @example
     * A coin transfer function has three arguments "from", "to" and "amount".
     * ```
     * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
     * ```
     */
    constructor(module_name: ModuleId, function_name: Identifier, type_args: Array<TypeTag>, args: Array<EntryFunctionArgument>);
    /**
     * A helper function to build a EntryFunction payload from raw primitive values
     *
     * @param module_id Fully qualified module name in format "AccountAddress::module_id" e.g. "0x1::coin"
     * @param function_name Function name
     * @param type_args Type arguments that move function requires.
     *
     * @example
     * A coin transfer function has one type argument "CoinType".
     * ```
     * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
     * ```
     * @param args Arguments to the move function.
     *
     * @example
     * A coin transfer function has three arguments "from", "to" and "amount".
     * ```
     * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
     * ```
     * @returns EntryFunction
     */
    static build(module_id: MoveModuleId, function_name: string, type_args: Array<TypeTag>, args: Array<EntryFunctionArgument>): EntryFunction;
    serialize(serializer: Serializer): void;
    /**
     * Deserializes an entry function payload with the arguments represented as EntryFunctionBytes instances.
     * @see EntryFunctionBytes
     *
     * NOTE: When you deserialize an EntryFunction payload with this method, the entry function
     * arguments are populated into the deserialized instance as type-agnostic, raw fixed bytes
     * in the form of the EntryFunctionBytes class.
     *
     * In order to correctly deserialize these arguments as their actual type representations, you
     * must know the types of the arguments beforehand and deserialize them yourself individually.
     *
     * One way you could achieve this is by using the ABIs for an entry function and deserializing each
     * argument as its given, corresponding type.
     *
     * @param deserializer
     * @returns A deserialized EntryFunction payload for a transaction.
     *
     */
    static deserialize(deserializer: Deserializer): EntryFunction;
}
/**
 * Representation of a Script that can serialized and deserialized
 */
declare class Script {
    /**
     * The move module bytecode
     */
    readonly bytecode: Uint8Array;
    /**
     * The type arguments that the bytecode function requires.
     */
    readonly type_args: Array<TypeTag>;
    /**
     * The arguments that the bytecode function requires.
     */
    readonly args: Array<ScriptFunctionArgument>;
    /**
     * Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.
     *
     * @param bytecode The move module bytecode
     * @param type_args The type arguments that the bytecode function requires.
     *
     * @example
     * A coin transfer function has one type argument "CoinType".
     * ```
     * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
     * ```
     * @param args The arguments that the bytecode function requires.
     *
     * @example
     * A coin transfer function has three arguments "from", "to" and "amount".
     * ```
     * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
     * ```
     */
    constructor(bytecode: Uint8Array, type_args: Array<TypeTag>, args: Array<ScriptFunctionArgument>);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): Script;
}
/**
 * Representation of a MultiSig that can serialized and deserialized
 */
declare class MultiSig {
    readonly multisig_address: AccountAddress;
    readonly transaction_payload?: MultiSigTransactionPayload;
    /**
     * Contains the payload to run a multi-sig account transaction.
     *
     * @param multisig_address The multi-sig account address the transaction will be executed as.
     *
     * @param transaction_payload The payload of the multi-sig transaction. This is optional when executing a multi-sig
     *  transaction whose payload is already stored on chain.
     */
    constructor(multisig_address: AccountAddress, transaction_payload?: MultiSigTransactionPayload);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): MultiSig;
}
/**
 * Representation of a MultiSig Transaction Payload from `multisig_account.move`
 * that can be serialized and deserialized

 * This class exists right now to represent an extensible transaction payload class for
 * transactions used in `multisig_account.move`. Eventually, this class will be able to
 * support script payloads when the `multisig_account.move` module supports them.
 */
declare class MultiSigTransactionPayload extends Serializable {
    readonly transaction_payload: EntryFunction;
    /**
     * Contains the payload to run a multi-sig account transaction.
     *
     * @param transaction_payload The payload of the multi-sig transaction.
     * This can only be EntryFunction for now but,
     * Script might be supported in the future.
     */
    constructor(transaction_payload: EntryFunction);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): MultiSigTransactionPayload;
}

export { EntryFunction, MultiSig, MultiSigTransactionPayload, Script, TransactionPayload, TransactionPayloadEntryFunction, TransactionPayloadMultiSig, TransactionPayloadScript, deserializeFromScriptArgument };
