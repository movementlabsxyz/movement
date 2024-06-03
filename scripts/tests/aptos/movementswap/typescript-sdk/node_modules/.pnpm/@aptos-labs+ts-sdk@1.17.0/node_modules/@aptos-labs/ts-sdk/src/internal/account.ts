// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/account}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * account namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptosConfig";
import { AptosApiError, getAptosFullNode, paginateWithCursor } from "../client";
import { AccountAddress, AccountAddressInput } from "../core/accountAddress";
import { Account } from "../account";
import { AnyPublicKey, Ed25519PublicKey, PrivateKey } from "../core/crypto";
import { queryIndexer } from "./general";
import {
  AccountData,
  GetAccountCoinsDataResponse,
  GetAccountCollectionsWithOwnedTokenResponse,
  GetAccountOwnedObjectsResponse,
  GetAccountOwnedTokensFromCollectionResponse,
  GetAccountOwnedTokensQueryResponse,
  LedgerVersionArg,
  MoveModuleBytecode,
  MoveResource,
  MoveStructId,
  OrderByArg,
  PaginationArgs,
  TokenStandardArg,
  TransactionResponse,
  WhereArg,
} from "../types";
import {
  GetAccountCoinsCountQuery,
  GetAccountCoinsDataQuery,
  GetAccountCollectionsWithOwnedTokensQuery,
  GetAccountOwnedObjectsQuery,
  GetAccountOwnedTokensFromCollectionQuery,
  GetAccountOwnedTokensQuery,
  GetAccountTokensCountQuery,
  GetAccountTransactionsCountQuery,
} from "../types/generated/operations";
import {
  GetAccountCoinsCount,
  GetAccountCoinsData,
  GetAccountCollectionsWithOwnedTokens,
  GetAccountOwnedObjects,
  GetAccountOwnedTokens,
  GetAccountOwnedTokensFromCollection,
  GetAccountTokensCount,
  GetAccountTransactionsCount,
} from "../types/generated/queries";
import { memoizeAsync } from "../utils/memoize";
import { Secp256k1PrivateKey, AuthenticationKey, Ed25519PrivateKey } from "../core";
import { CurrentFungibleAssetBalancesBoolExp } from "../types/generated/types";
import { getTableItem } from "./table";

export async function getInfo(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
}): Promise<AccountData> {
  const { aptosConfig, accountAddress } = args;
  const { data } = await getAptosFullNode<{}, AccountData>({
    aptosConfig,
    originMethod: "getInfo",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}`,
  });
  return data;
}

export async function getModules(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: PaginationArgs & LedgerVersionArg;
}): Promise<MoveModuleBytecode[]> {
  const { aptosConfig, accountAddress, options } = args;
  return paginateWithCursor<{}, MoveModuleBytecode[]>({
    aptosConfig,
    originMethod: "getModules",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}/modules`,
    params: {
      ledger_version: options?.ledgerVersion,
      start: options?.offset,
      limit: options?.limit ?? 1000,
    },
  });
}

/**
 * Queries for a move module given account address and module name
 *
 * @param args.accountAddress Hex-encoded 32 byte Aptos account address
 * @param args.moduleName The name of the module
 * @param args.query.ledgerVersion Specifies ledger version of transactions. By default, latest version will be used
 * @returns The move module.
 */
export async function getModule(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  moduleName: string;
  options?: LedgerVersionArg;
}): Promise<MoveModuleBytecode> {
  // We don't memoize the account module by ledger version, as it's not a common use case, this would be handled
  // by the developer directly
  if (args.options?.ledgerVersion !== undefined) {
    return getModuleInner(args);
  }

  return memoizeAsync(
    async () => getModuleInner(args),
    `module-${args.accountAddress}-${args.moduleName}`,
    1000 * 60 * 5, // 5 minutes
  )();
}

async function getModuleInner(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  moduleName: string;
  options?: LedgerVersionArg;
}): Promise<MoveModuleBytecode> {
  const { aptosConfig, accountAddress, moduleName, options } = args;

  const { data } = await getAptosFullNode<{}, MoveModuleBytecode>({
    aptosConfig,
    originMethod: "getModule",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}/module/${moduleName}`,
    params: { ledger_version: options?.ledgerVersion },
  });
  return data;
}

export async function getTransactions(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: PaginationArgs;
}): Promise<TransactionResponse[]> {
  const { aptosConfig, accountAddress, options } = args;
  return paginateWithCursor<{}, TransactionResponse[]>({
    aptosConfig,
    originMethod: "getTransactions",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}/transactions`,
    params: { start: options?.offset, limit: options?.limit },
  });
}

export async function getResources(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: PaginationArgs & LedgerVersionArg;
}): Promise<MoveResource[]> {
  const { aptosConfig, accountAddress, options } = args;
  return paginateWithCursor<{}, MoveResource[]>({
    aptosConfig,
    originMethod: "getResources",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}/resources`,
    params: {
      ledger_version: options?.ledgerVersion,
      start: options?.offset,
      limit: options?.limit ?? 999,
    },
  });
}

export async function getResource<T extends {}>(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  resourceType: MoveStructId;
  options?: LedgerVersionArg;
}): Promise<T> {
  const { aptosConfig, accountAddress, resourceType, options } = args;
  const { data } = await getAptosFullNode<{}, MoveResource>({
    aptosConfig,
    originMethod: "getResource",
    path: `accounts/${AccountAddress.from(accountAddress).toString()}/resource/${resourceType}`,
    params: { ledger_version: options?.ledgerVersion },
  });
  return data.data as T;
}

export async function lookupOriginalAccountAddress(args: {
  aptosConfig: AptosConfig;
  authenticationKey: AccountAddressInput;
  options?: LedgerVersionArg;
}): Promise<AccountAddress> {
  const { aptosConfig, authenticationKey, options } = args;
  type OriginatingAddress = {
    address_map: { handle: string };
  };
  const resource = await getResource<OriginatingAddress>({
    aptosConfig,
    accountAddress: "0x1",
    resourceType: "0x1::account::OriginatingAddress",
    options,
  });

  const {
    address_map: { handle },
  } = resource;

  const authKeyAddress = AccountAddress.from(authenticationKey);

  // If the address is not found in the address map, which means its not rotated
  // then return the address as is
  try {
    const originalAddress = await getTableItem<string>({
      aptosConfig,
      handle,
      data: {
        key: authKeyAddress.toString(),
        key_type: "address",
        value_type: "address",
      },
      options,
    });

    return AccountAddress.from(originalAddress);
  } catch (err) {
    if (err instanceof AptosApiError && err.data.error_code === "table_item_not_found") {
      return authKeyAddress;
    }

    throw err;
  }
}

export async function getAccountTokensCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
}): Promise<number> {
  const { aptosConfig, accountAddress } = args;

  const address = AccountAddress.from(accountAddress).toStringLong();

  const whereCondition: { owner_address: { _eq: string }; amount: { _gt: number } } = {
    owner_address: { _eq: address },
    amount: { _gt: 0 },
  };

  const graphqlQuery = {
    query: GetAccountTokensCount,
    variables: { where_condition: whereCondition },
  };

  const data = await queryIndexer<GetAccountTokensCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountTokensCount",
  });

  // commonjs (aka cjs) doesnt handle Nullish Coalescing for some reason
  // might be because of how ts infer the graphql generated scheme type
  return data.current_token_ownerships_v2_aggregate.aggregate
    ? data.current_token_ownerships_v2_aggregate.aggregate.count
    : 0;
}

export async function getAccountOwnedTokens(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: TokenStandardArg & PaginationArgs & OrderByArg<GetAccountOwnedTokensQueryResponse[0]>;
}): Promise<GetAccountOwnedTokensQueryResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const whereCondition: { owner_address: { _eq: string }; amount: { _gt: number }; token_standard?: { _eq: string } } =
    {
      owner_address: { _eq: address },
      amount: { _gt: 0 },
    };

  if (options?.tokenStandard) {
    whereCondition.token_standard = { _eq: options?.tokenStandard };
  }

  const graphqlQuery = {
    query: GetAccountOwnedTokens,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountOwnedTokensQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountOwnedTokens",
  });

  return data.current_token_ownerships_v2;
}

export async function getAccountOwnedTokensFromCollectionAddress(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  collectionAddress: AccountAddressInput;
  options?: TokenStandardArg & PaginationArgs & OrderByArg<GetAccountOwnedTokensFromCollectionResponse[0]>;
}): Promise<GetAccountOwnedTokensFromCollectionResponse> {
  const { aptosConfig, accountAddress, collectionAddress, options } = args;
  const ownerAddress = AccountAddress.from(accountAddress).toStringLong();
  const collAddress = AccountAddress.from(collectionAddress).toStringLong();

  const whereCondition: {
    owner_address: { _eq: string };
    current_token_data: { collection_id: { _eq: string } };
    amount: { _gt: number };
    token_standard?: { _eq: string };
  } = {
    owner_address: { _eq: ownerAddress },
    current_token_data: { collection_id: { _eq: collAddress } },
    amount: { _gt: 0 },
  };

  if (options?.tokenStandard) {
    whereCondition.token_standard = { _eq: options?.tokenStandard };
  }

  const graphqlQuery = {
    query: GetAccountOwnedTokensFromCollection,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountOwnedTokensFromCollectionQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountOwnedTokensFromCollectionAddress",
  });

  return data.current_token_ownerships_v2;
}

export async function getAccountCollectionsWithOwnedTokens(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: TokenStandardArg & PaginationArgs & OrderByArg<GetAccountCollectionsWithOwnedTokenResponse[0]>;
}): Promise<GetAccountCollectionsWithOwnedTokenResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const whereCondition: {
    owner_address: { _eq: string };
    amount: { _gt: number };
    current_collection?: { token_standard: { _eq: string } };
  } = {
    owner_address: { _eq: address },
    amount: { _gt: 0 },
  };

  if (options?.tokenStandard) {
    whereCondition.current_collection = {
      token_standard: { _eq: options?.tokenStandard },
    };
  }

  const graphqlQuery = {
    query: GetAccountCollectionsWithOwnedTokens,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountCollectionsWithOwnedTokensQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountCollectionsWithOwnedTokens",
  });

  return data.current_collection_ownership_v2_view;
}

export async function getAccountTransactionsCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
}): Promise<number> {
  const { aptosConfig, accountAddress } = args;

  const address = AccountAddress.from(accountAddress).toStringLong();

  const graphqlQuery = {
    query: GetAccountTransactionsCount,
    variables: { address },
  };

  const data = await queryIndexer<GetAccountTransactionsCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountTransactionsCount",
  });

  // commonjs (aka cjs) doesnt handle Nullish Coalescing for some reason
  // might be because of how ts infer the graphql generated scheme type
  return data.account_transactions_aggregate.aggregate ? data.account_transactions_aggregate.aggregate.count : 0;
}

export async function getAccountCoinAmount(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  coinType: MoveStructId;
}): Promise<number> {
  const { aptosConfig, accountAddress, coinType } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const data = await getAccountCoinsData({
    aptosConfig,
    accountAddress: address,
    options: {
      where: { asset_type: { _eq: coinType } },
    },
  });

  // commonjs (aka cjs) doesnt handle Nullish Coalescing for some reason
  // might be because of how ts infer the graphql generated scheme type
  return data[0] ? data[0].amount : 0;
}

export async function getAccountCoinsData(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: PaginationArgs & OrderByArg<GetAccountCoinsDataResponse[0]> & WhereArg<CurrentFungibleAssetBalancesBoolExp>;
}): Promise<GetAccountCoinsDataResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const whereCondition: { owner_address: { _eq: string } } = {
    ...options?.where,
    owner_address: { _eq: address },
  };

  const graphqlQuery = {
    query: GetAccountCoinsData,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountCoinsDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountCoinsData",
  });

  return data.current_fungible_asset_balances;
}

export async function getAccountCoinsCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
}): Promise<number> {
  const { aptosConfig, accountAddress } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const graphqlQuery = {
    query: GetAccountCoinsCount,
    variables: { address },
  };

  const data = await queryIndexer<GetAccountCoinsCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountCoinsCount",
  });

  if (!data.current_fungible_asset_balances_aggregate.aggregate) {
    throw Error("Failed to get the count of account coins");
  }

  return data.current_fungible_asset_balances_aggregate.aggregate.count;
}

export async function getAccountOwnedObjects(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  options?: PaginationArgs & OrderByArg<GetAccountOwnedObjectsResponse[0]>;
}): Promise<GetAccountOwnedObjectsResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.from(accountAddress).toStringLong();

  const whereCondition: { owner_address: { _eq: string } } = {
    owner_address: { _eq: address },
  };
  const graphqlQuery = {
    query: GetAccountOwnedObjects,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };
  const data = await queryIndexer<GetAccountOwnedObjectsQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getAccountOwnedObjects",
  });

  return data.current_objects;
}

/**
 * NOTE: There is a potential issue once unified single signer scheme will be adopted
 * by the community.
 *
 * Becuase on could create 2 accounts with the same private key with this new authenticator type,
 * we’ll need to determine the order in which we lookup the accounts. First unified
 * scheme and then legacy scheme vs first legacy scheme and then unified scheme.
 *
 */
export async function deriveAccountFromPrivateKey(args: {
  aptosConfig: AptosConfig;
  privateKey: PrivateKey;
}): Promise<Account> {
  const { aptosConfig, privateKey } = args;
  const publicKey = new AnyPublicKey(privateKey.publicKey());

  if (privateKey instanceof Secp256k1PrivateKey) {
    // private key is secp256k1, therefore we know it for sure uses a single signer key
    const authKey = AuthenticationKey.fromPublicKey({ publicKey });
    const address = authKey.derivedAddress();
    return Account.fromPrivateKey({ privateKey, address });
  }

  if (privateKey instanceof Ed25519PrivateKey) {
    // lookup single sender ed25519
    const singleSenderTransactionAuthenticatorAuthKey = AuthenticationKey.fromPublicKey({
      publicKey,
    });
    const isSingleSenderTransactionAuthenticator = await isAccountExist({
      authKey: singleSenderTransactionAuthenticatorAuthKey,
      aptosConfig,
    });
    if (isSingleSenderTransactionAuthenticator) {
      const address = singleSenderTransactionAuthenticatorAuthKey.derivedAddress();
      return Account.fromPrivateKey({ privateKey, address, legacy: false });
    }
    // lookup legacy ed25519
    const legacyAuthKey = AuthenticationKey.fromPublicKey({
      publicKey: publicKey.publicKey as Ed25519PublicKey,
    });
    const isLegacyEd25519 = await isAccountExist({ authKey: legacyAuthKey, aptosConfig });
    if (isLegacyEd25519) {
      const address = legacyAuthKey.derivedAddress();
      return Account.fromPrivateKey({ privateKey, address, legacy: true });
    }
  }
  // if we are here, it means we couldn't find an address with an
  // auth key that matches the provided private key
  throw new Error(`Can't derive account from private key ${privateKey}`);
}

export async function isAccountExist(args: { aptosConfig: AptosConfig; authKey: AuthenticationKey }): Promise<boolean> {
  const { aptosConfig, authKey } = args;
  const accountAddress = await lookupOriginalAccountAddress({
    aptosConfig,
    authenticationKey: authKey.derivedAddress(),
  });

  try {
    await getInfo({
      aptosConfig,
      accountAddress,
    });
    return true;
  } catch (error: any) {
    // account not found
    if (error.status === 404) {
      return false;
    }
    throw new Error(`Error while looking for an account info ${accountAddress.toString()}`);
  }
}
