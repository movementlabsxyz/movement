import { Deserializer } from '../../bcs/deserializer.mjs';
import { Serializable, Serializer } from '../../bcs/serializer.mjs';
import { ChainId } from './chainId.mjs';
import { AccountAddress } from '../../core/accountAddress.mjs';
import { TransactionPayload } from './transactionPayload.mjs';
import '../../types/index.mjs';
import '../../utils/apiEndpoints.mjs';
import '../../types/indexer.mjs';
import '../../types/generated/operations.mjs';
import '../../types/generated/types.mjs';
import '../../core/hex.mjs';
import '../../core/common.mjs';
import './transactionArgument.mjs';
import './identifier.mjs';
import './moduleId.mjs';
import '../typeTag/index.mjs';

/**
 * Representation of a Raw Transaction that can serialized and deserialized
 */
declare class RawTransaction extends Serializable {
    readonly sender: AccountAddress;
    readonly sequence_number: bigint;
    readonly payload: TransactionPayload;
    readonly max_gas_amount: bigint;
    readonly gas_unit_price: bigint;
    readonly expiration_timestamp_secs: bigint;
    readonly chain_id: ChainId;
    /**
     * RawTransactions contain the metadata and payloads that can be submitted to Aptos chain for execution.
     * RawTransactions must be signed before Aptos chain can execute them.
     *
     * @param sender The sender Account Address
     * @param sequence_number Sequence number of this transaction. This must match the sequence number stored in
     *   the sender's account at the time the transaction executes.
     * @param payload Instructions for the Aptos Blockchain, including publishing a module,
     *   execute an entry function or execute a script payload.
     * @param max_gas_amount Maximum total gas to spend for this transaction. The account must have more
     *   than this gas or the transaction will be discarded during validation.
     * @param gas_unit_price Price to be paid per gas unit.
     * @param expiration_timestamp_secs The blockchain timestamp at which the blockchain would discard this transaction.
     * @param chain_id The chain ID of the blockchain that this transaction is intended to be run on.
     */
    constructor(sender: AccountAddress, sequence_number: bigint, payload: TransactionPayload, max_gas_amount: bigint, gas_unit_price: bigint, expiration_timestamp_secs: bigint, chain_id: ChainId);
    serialize(serializer: Serializer): void;
    static deserialize(deserializer: Deserializer): RawTransaction;
}
/**
 * Representation of a Raw Transaction With Data that can serialized and deserialized
 */
declare abstract class RawTransactionWithData extends Serializable {
    /**
     * Serialize a Raw Transaction With Data
     */
    abstract serialize(serializer: Serializer): void;
    /**
     * Deserialize a Raw Transaction With Data
     */
    static deserialize(deserializer: Deserializer): RawTransactionWithData;
}
/**
 * Representation of a Multi Agent Transaction that can serialized and deserialized
 */
declare class MultiAgentRawTransaction extends RawTransactionWithData {
    /**
     * The raw transaction
     */
    readonly raw_txn: RawTransaction;
    /**
     * The secondary signers on this transaction
     */
    readonly secondary_signer_addresses: Array<AccountAddress>;
    constructor(raw_txn: RawTransaction, secondary_signer_addresses: Array<AccountAddress>);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): MultiAgentRawTransaction;
}
/**
 * Representation of a Fee Payer Transaction that can serialized and deserialized
 */
declare class FeePayerRawTransaction extends RawTransactionWithData {
    /**
     * The raw transaction
     */
    readonly raw_txn: RawTransaction;
    /**
     * The secondary signers on this transaction - optional and can be empty
     */
    readonly secondary_signer_addresses: Array<AccountAddress>;
    /**
     * The fee payer account address
     */
    readonly fee_payer_address: AccountAddress;
    constructor(raw_txn: RawTransaction, secondary_signer_addresses: Array<AccountAddress>, fee_payer_address: AccountAddress);
    serialize(serializer: Serializer): void;
    static load(deserializer: Deserializer): FeePayerRawTransaction;
}

export { FeePayerRawTransaction, MultiAgentRawTransaction, RawTransaction, RawTransactionWithData };
