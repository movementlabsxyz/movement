import { AptosConfig } from '../api/aptosConfig.mjs';
import { AccountAddressInput } from '../core/accountAddress.mjs';
import { WaitForTransactionOptions, UserTransactionResponse } from '../types/index.mjs';
import '../utils/apiEndpoints.mjs';
import '../utils/const.mjs';
import '../types/indexer.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';
import '../bcs/serializer.mjs';
import '../core/hex.mjs';
import '../core/common.mjs';
import '../bcs/deserializer.mjs';
import '../transactions/instances/transactionArgument.mjs';

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/faucet}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * faucet namespace and without having a dependency cycle error.
 */

declare function fundAccount(args: {
    aptosConfig: AptosConfig;
    accountAddress: AccountAddressInput;
    amount: number;
    options?: WaitForTransactionOptions;
}): Promise<UserTransactionResponse>;

export { fundAccount };
