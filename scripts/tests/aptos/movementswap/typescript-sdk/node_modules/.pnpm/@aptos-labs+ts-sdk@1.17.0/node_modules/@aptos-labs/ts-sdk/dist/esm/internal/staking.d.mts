import { AptosConfig } from '../api/aptosConfig.mjs';
import { AccountAddressInput } from '../core/accountAddress.mjs';
import { OrderByArg } from '../types/index.mjs';
import { GetNumberOfDelegatorsResponse, GetDelegatedStakingActivitiesResponse } from '../types/indexer.mjs';
import '../utils/apiEndpoints.mjs';
import '../utils/const.mjs';
import '../types/generated/operations.mjs';
import '../types/generated/types.mjs';
import '../bcs/serializer.mjs';
import '../core/hex.mjs';
import '../core/common.mjs';
import '../bcs/deserializer.mjs';
import '../transactions/instances/transactionArgument.mjs';

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/staking}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * faucet namespace and without having a dependency cycle error.
 */

declare function getNumberOfDelegators(args: {
    aptosConfig: AptosConfig;
    poolAddress: AccountAddressInput;
}): Promise<number>;
declare function getNumberOfDelegatorsForAllPools(args: {
    aptosConfig: AptosConfig;
    options?: OrderByArg<GetNumberOfDelegatorsResponse[0]>;
}): Promise<GetNumberOfDelegatorsResponse>;
declare function getDelegatedStakingActivities(args: {
    aptosConfig: AptosConfig;
    delegatorAddress: AccountAddressInput;
    poolAddress: AccountAddressInput;
}): Promise<GetDelegatedStakingActivitiesResponse>;

export { getDelegatedStakingActivities, getNumberOfDelegators, getNumberOfDelegatorsForAllPools };
