// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/digitalAsset}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * digitalAsset namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptosConfig";
import { Bool, MoveString, MoveVector, U64 } from "../bcs";
import { AccountAddress, AccountAddressInput } from "../core";
import { Account } from "../account";
import { EntryFunctionABI, InputGenerateTransactionOptions } from "../transactions/types";
import {
  AnyNumber,
  GetCollectionDataResponse,
  GetCurrentTokenOwnershipResponse,
  GetOwnedTokensResponse,
  GetTokenActivityResponse,
  GetTokenDataResponse,
  MoveAbility,
  MoveStructId,
  OrderByArg,
  PaginationArgs,
  TokenStandardArg,
} from "../types";
import {
  GetCollectionDataQuery,
  GetCurrentTokenOwnershipQuery,
  GetTokenActivityQuery,
  GetTokenDataQuery,
} from "../types/generated/operations";
import {
  GetCollectionData,
  GetCurrentTokenOwnership,
  GetTokenActivity,
  GetTokenData,
} from "../types/generated/queries";
import { queryIndexer } from "./general";
import { generateTransaction } from "./transactionSubmission";
import { MAX_U64_BIG_INT } from "../bcs/consts";
import { CurrentTokenOwnershipsV2BoolExp, TokenActivitiesV2BoolExp } from "../types/generated/types";
import {
  checkOrConvertArgument,
  objectStructTag,
  parseTypeTag,
  stringStructTag,
  TypeTagAddress,
  TypeTagBool,
  TypeTagGeneric,
  TypeTagStruct,
  TypeTagU64,
  TypeTagVector,
} from "../transactions";
import { SimpleTransaction } from "../transactions/instances/simpleTransaction";

// A property type map for the user input and what Move expects
const PropertyTypeMap = {
  BOOLEAN: "bool",
  U8: "u8",
  U16: "u16",
  U32: "u32",
  U64: "u64",
  U128: "u128",
  U256: "u256",
  ADDRESS: "address",
  STRING: "0x1::string::String",
  ARRAY: "vector<u8>",
};

export type PropertyType = keyof typeof PropertyTypeMap;

// Accepted property value types for user input
// To pass in an Array, use Uint8Array type
// for example `new MoveVector([new MoveString("hello"), new MoveString("world")]).bcsToBytes()`
export type PropertyValue = boolean | number | bigint | string | AccountAddress | Uint8Array;

// The default digital asset type to use if non provided
const defaultDigitalAssetType = "0x4::token::Token";

// FETCH QUERIES

export async function getDigitalAssetData(args: {
  aptosConfig: AptosConfig;
  digitalAssetAddress: AccountAddressInput;
}): Promise<GetTokenDataResponse> {
  const { aptosConfig, digitalAssetAddress } = args;

  const whereCondition: { token_data_id: { _eq: string } } = {
    token_data_id: { _eq: AccountAddress.from(digitalAssetAddress).toStringLong() },
  };

  const graphqlQuery = {
    query: GetTokenData,
    variables: {
      where_condition: whereCondition,
    },
  };

  const data = await queryIndexer<GetTokenDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getDigitalAssetData",
  });

  return data.current_token_datas_v2[0];
}

export async function getCurrentDigitalAssetOwnership(args: {
  aptosConfig: AptosConfig;
  digitalAssetAddress: AccountAddressInput;
}): Promise<GetCurrentTokenOwnershipResponse> {
  const { aptosConfig, digitalAssetAddress } = args;

  const whereCondition: CurrentTokenOwnershipsV2BoolExp = {
    token_data_id: { _eq: AccountAddress.from(digitalAssetAddress).toStringLong() },
    amount: { _gt: 0 },
  };

  const graphqlQuery = {
    query: GetCurrentTokenOwnership,
    variables: {
      where_condition: whereCondition,
    },
  };

  const data = await queryIndexer<GetCurrentTokenOwnershipQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getCurrentDigitalAssetOwnership",
  });

  return data.current_token_ownerships_v2[0];
}

export async function getOwnedDigitalAssets(args: {
  aptosConfig: AptosConfig;
  ownerAddress: AccountAddressInput;
  options?: PaginationArgs & OrderByArg<GetTokenActivityResponse[0]>;
}): Promise<GetOwnedTokensResponse> {
  const { aptosConfig, ownerAddress, options } = args;

  const whereCondition: CurrentTokenOwnershipsV2BoolExp = {
    owner_address: { _eq: AccountAddress.from(ownerAddress).toStringLong() },
    amount: { _gt: 0 },
  };

  const graphqlQuery = {
    query: GetCurrentTokenOwnership,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetCurrentTokenOwnershipQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getOwnedDigitalAssets",
  });

  return data.current_token_ownerships_v2;
}

export async function getDigitalAssetActivity(args: {
  aptosConfig: AptosConfig;
  digitalAssetAddress: AccountAddressInput;
  options?: PaginationArgs & OrderByArg<GetTokenActivityResponse[0]>;
}): Promise<GetTokenActivityResponse> {
  const { aptosConfig, digitalAssetAddress, options } = args;

  const whereCondition: TokenActivitiesV2BoolExp = {
    token_data_id: { _eq: AccountAddress.from(digitalAssetAddress).toStringLong() },
  };

  const graphqlQuery = {
    query: GetTokenActivity,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetTokenActivityQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getDigitalAssetActivity",
  });

  return data.token_activities_v2;
}

export interface CreateCollectionOptions {
  maxSupply?: AnyNumber;
  mutableDescription?: boolean;
  mutableRoyalty?: boolean;
  mutableURI?: boolean;
  mutableTokenDescription?: boolean;
  mutableTokenName?: boolean;
  mutableTokenProperties?: boolean;
  mutableTokenURI?: boolean;
  tokensBurnableByCreator?: boolean;
  tokensFreezableByCreator?: boolean;
  royaltyNumerator?: number;
  royaltyDenominator?: number;
}

const createCollectionAbi: EntryFunctionABI = {
  typeParameters: [],
  parameters: [
    new TypeTagStruct(stringStructTag()),
    new TypeTagU64(),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagBool(),
    new TypeTagU64(),
    new TypeTagU64(),
  ],
};

export async function createCollectionTransaction(
  args: {
    aptosConfig: AptosConfig;
    creator: Account;
    description: string;
    name: string;
    uri: string;
    options?: InputGenerateTransactionOptions;
  } & CreateCollectionOptions,
): Promise<SimpleTransaction> {
  const { aptosConfig, options, creator } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::create_collection",
      functionArguments: [
        // Do not change the order
        new MoveString(args.description),
        new U64(args.maxSupply ?? MAX_U64_BIG_INT),
        new MoveString(args.name),
        new MoveString(args.uri),
        new Bool(args.mutableDescription ?? true),
        new Bool(args.mutableRoyalty ?? true),
        new Bool(args.mutableURI ?? true),
        new Bool(args.mutableTokenDescription ?? true),
        new Bool(args.mutableTokenName ?? true),
        new Bool(args.mutableTokenProperties ?? true),
        new Bool(args.mutableTokenURI ?? true),
        new Bool(args.tokensBurnableByCreator ?? true),
        new Bool(args.tokensFreezableByCreator ?? true),
        new U64(args.royaltyNumerator ?? 0),
        new U64(args.royaltyDenominator ?? 1),
      ],
      abi: createCollectionAbi,
    },
    options,
  });
}

export async function getCollectionData(args: {
  aptosConfig: AptosConfig;
  creatorAddress: AccountAddressInput;
  collectionName: string;
  options?: TokenStandardArg;
}): Promise<GetCollectionDataResponse> {
  const { aptosConfig, creatorAddress, collectionName, options } = args;
  const address = AccountAddress.from(creatorAddress);

  const whereCondition: any = {
    collection_name: { _eq: collectionName },
    creator_address: { _eq: address.toStringLong() },
  };

  if (options?.tokenStandard) {
    whereCondition.token_standard = { _eq: options?.tokenStandard ?? "v2" };
  }

  const graphqlQuery = {
    query: GetCollectionData,
    variables: {
      where_condition: whereCondition,
    },
  };
  const data = await queryIndexer<GetCollectionDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getCollectionData",
  });

  return data.current_collections_v2[0];
}

export async function getCollectionDataByCollectionId(args: {
  aptosConfig: AptosConfig;
  collectionId: AccountAddressInput;
}): Promise<GetCollectionDataResponse> {
  const { aptosConfig, collectionId } = args;
  const address = AccountAddress.from(collectionId);

  const whereCondition: any = {
    collection_id: { _eq: address.toStringLong() },
  };

  const graphqlQuery = {
    query: GetCollectionData,
    variables: {
      where_condition: whereCondition,
    },
  };
  const data = await queryIndexer<GetCollectionDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getCollectionData",
  });

  return data.current_collections_v2[0];
}

export async function getCollectionId(args: {
  aptosConfig: AptosConfig;
  creatorAddress: AccountAddressInput;
  collectionName: string;
  options?: TokenStandardArg;
}): Promise<string> {
  return (await getCollectionData(args)).collection_id;
}

// TRANSACTIONS

const mintDigitalAssetAbi: EntryFunctionABI = {
  typeParameters: [],
  parameters: [
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagVector(new TypeTagStruct(stringStructTag())),
    new TypeTagVector(new TypeTagStruct(stringStructTag())),
    new TypeTagVector(TypeTagVector.u8()),
  ],
};

export async function mintDigitalAssetTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  collection: string;
  description: string;
  name: string;
  uri: string;
  propertyKeys?: Array<string>;
  propertyTypes?: Array<PropertyType>;
  propertyValues?: Array<PropertyValue>;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    options,
    creator,
    collection,
    description,
    name,
    uri,
    propertyKeys,
    propertyTypes,
    propertyValues,
  } = args;
  const convertedPropertyType = propertyTypes?.map((type) => PropertyTypeMap[type]);
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::mint",
      functionArguments: [
        new MoveString(collection),
        new MoveString(description),
        new MoveString(name),
        new MoveString(uri),
        MoveVector.MoveString(propertyKeys ?? []),
        MoveVector.MoveString(convertedPropertyType ?? []),
        getPropertyValueRaw(propertyValues ?? [], convertedPropertyType ?? []),
      ],
      abi: mintDigitalAssetAbi,
    },
    options,
  });
}

const transferDigitalAssetAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))), new TypeTagAddress()],
};

export async function transferDigitalAssetTransaction(args: {
  aptosConfig: AptosConfig;
  sender: Account;
  digitalAssetAddress: AccountAddressInput;
  recipient: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, sender, digitalAssetAddress, recipient, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: sender.accountAddress,
    data: {
      function: "0x1::object::transfer",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress), AccountAddress.from(recipient)],
      abi: transferDigitalAssetAbi,
    },
    options,
  });
}

const mintSoulBoundAbi: EntryFunctionABI = {
  typeParameters: [],
  parameters: [
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    new TypeTagVector(new TypeTagStruct(stringStructTag())),
    new TypeTagVector(new TypeTagStruct(stringStructTag())),
    new TypeTagVector(TypeTagVector.u8()),
    new TypeTagAddress(),
  ],
};

export async function mintSoulBoundTransaction(args: {
  aptosConfig: AptosConfig;
  account: Account;
  collection: string;
  description: string;
  name: string;
  uri: string;
  recipient: AccountAddressInput;
  propertyKeys?: Array<string>;
  propertyTypes?: Array<PropertyType>;
  propertyValues?: Array<PropertyValue>;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    account,
    collection,
    description,
    name,
    uri,
    recipient,
    propertyKeys,
    propertyTypes,
    propertyValues,
    options,
  } = args;
  if (propertyKeys?.length !== propertyValues?.length) {
    throw new Error("Property keys and property values counts do not match");
  }
  if (propertyTypes?.length !== propertyValues?.length) {
    throw new Error("Property types and property values counts do not match");
  }
  const convertedPropertyType = propertyTypes?.map((type) => PropertyTypeMap[type]);
  return generateTransaction({
    aptosConfig,
    sender: account.accountAddress,
    data: {
      function: "0x4::aptos_token::mint_soul_bound",
      functionArguments: [
        collection,
        description,
        name,
        uri,
        MoveVector.MoveString(propertyKeys ?? []),
        MoveVector.MoveString(convertedPropertyType ?? []),
        getPropertyValueRaw(propertyValues ?? [], convertedPropertyType ?? []),
        recipient,
      ],
      abi: mintSoulBoundAbi,
    },
    options,
  });
}

const burnDigitalAssetAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0)))],
};

export async function burnDigitalAssetTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::burn",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress)],
      abi: burnDigitalAssetAbi,
    },
    options,
  });
}

const freezeDigitalAssetAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0)))],
};

export async function freezeDigitalAssetTransferTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::freeze_transfer",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [digitalAssetAddress],
      abi: freezeDigitalAssetAbi,
    },
    options,
  });
}

const unfreezeDigitalAssetAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0)))],
};

export async function unfreezeDigitalAssetTransferTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::unfreeze_transfer",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [digitalAssetAddress],
      abi: unfreezeDigitalAssetAbi,
    },
    options,
  });
}

const setDigitalAssetDescriptionAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))), new TypeTagStruct(stringStructTag())],
};

export async function setDigitalAssetDescriptionTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  description: string;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, description, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::set_description",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(description)],
      abi: setDigitalAssetDescriptionAbi,
    },
    options,
  });
}

const setDigitalAssetNameAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))), new TypeTagStruct(stringStructTag())],
};

export async function setDigitalAssetNameTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  name: string;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, name, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::set_name",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(name)],
      abi: setDigitalAssetNameAbi,
    },
    options,
  });
}

const setDigitalAssetURIAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))), new TypeTagStruct(stringStructTag())],
};

export async function setDigitalAssetURITransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  uri: string;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, uri, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::set_uri",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(uri)],
      abi: setDigitalAssetURIAbi,
    },
    options,
  });
}

const addDigitalAssetPropertyAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [
    new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    TypeTagVector.u8(),
  ],
};

export async function addDigitalAssetPropertyTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  propertyKey: string;
  propertyType: PropertyType;
  propertyValue: PropertyValue;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    creator,
    propertyKey,
    propertyType,
    propertyValue,
    digitalAssetAddress,
    digitalAssetType,
    options,
  } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::add_property",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [
        AccountAddress.from(digitalAssetAddress),
        new MoveString(propertyKey),
        new MoveString(PropertyTypeMap[propertyType]),
        MoveVector.U8(getSinglePropertyValueRaw(propertyValue, PropertyTypeMap[propertyType])),
      ],
      abi: addDigitalAssetPropertyAbi,
    },
    options,
  });
}

const removeDigitalAssetPropertyAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))), new TypeTagStruct(stringStructTag())],
};

export async function removeDigitalAssetPropertyTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  propertyKey: string;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, creator, propertyKey, digitalAssetAddress, digitalAssetType, options } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::remove_property",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(propertyKey)],
      abi: removeDigitalAssetPropertyAbi,
    },
    options,
  });
}

const updateDigitalAssetPropertyAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }],
  parameters: [
    new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))),
    new TypeTagStruct(stringStructTag()),
    new TypeTagStruct(stringStructTag()),
    TypeTagVector.u8(),
  ],
};

export async function updateDigitalAssetPropertyTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  propertyKey: string;
  propertyType: PropertyType;
  propertyValue: PropertyValue;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    creator,
    propertyKey,
    propertyType,
    propertyValue,
    digitalAssetAddress,
    digitalAssetType,
    options,
  } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::update_property",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType],
      functionArguments: [
        AccountAddress.from(digitalAssetAddress),
        new MoveString(propertyKey),
        new MoveString(PropertyTypeMap[propertyType]),
        getSinglePropertyValueRaw(propertyValue, PropertyTypeMap[propertyType]),
      ],
      abi: updateDigitalAssetPropertyAbi,
    },
    options,
  });
}

const addDigitalAssetTypedPropertyAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }, { constraints: [] }],
  parameters: [
    new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))),
    new TypeTagStruct(stringStructTag()),
    new TypeTagGeneric(1),
  ],
};

export async function addDigitalAssetTypedPropertyTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  propertyKey: string;
  propertyType: PropertyType;
  propertyValue: PropertyValue;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    creator,
    propertyKey,
    propertyType,
    propertyValue,
    digitalAssetAddress,
    digitalAssetType,
    options,
  } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::add_typed_property",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType, PropertyTypeMap[propertyType]],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(propertyKey), propertyValue],
      abi: addDigitalAssetTypedPropertyAbi,
    },
    options,
  });
}

const updateDigitalAssetTypedPropertyAbi: EntryFunctionABI = {
  typeParameters: [{ constraints: [MoveAbility.KEY] }, { constraints: [] }],
  parameters: [
    new TypeTagStruct(objectStructTag(new TypeTagGeneric(0))),
    new TypeTagStruct(stringStructTag()),
    new TypeTagGeneric(1),
  ],
};

export async function updateDigitalAssetTypedPropertyTransaction(args: {
  aptosConfig: AptosConfig;
  creator: Account;
  propertyKey: string;
  propertyType: PropertyType;
  propertyValue: PropertyValue;
  digitalAssetAddress: AccountAddressInput;
  digitalAssetType?: MoveStructId;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const {
    aptosConfig,
    creator,
    propertyKey,
    propertyType,
    propertyValue,
    digitalAssetAddress,
    digitalAssetType,
    options,
  } = args;
  return generateTransaction({
    aptosConfig,
    sender: creator.accountAddress,
    data: {
      function: "0x4::aptos_token::update_typed_property",
      typeArguments: [digitalAssetType ?? defaultDigitalAssetType, PropertyTypeMap[propertyType]],
      functionArguments: [AccountAddress.from(digitalAssetAddress), new MoveString(propertyKey), propertyValue],
      abi: updateDigitalAssetTypedPropertyAbi,
    },
    options,
  });
}

function getPropertyValueRaw(propertyValues: Array<PropertyValue>, propertyTypes: Array<string>): Array<Uint8Array> {
  const results = new Array<Uint8Array>();
  propertyTypes.forEach((typ, index) => {
    results.push(getSinglePropertyValueRaw(propertyValues[index], typ));
  });

  return results;
}

function getSinglePropertyValueRaw(propertyValue: PropertyValue, propertyType: string): Uint8Array {
  const typeTag = parseTypeTag(propertyType);
  const res = checkOrConvertArgument(propertyValue, typeTag, 0, []);
  return res.bcsToBytes();
}
