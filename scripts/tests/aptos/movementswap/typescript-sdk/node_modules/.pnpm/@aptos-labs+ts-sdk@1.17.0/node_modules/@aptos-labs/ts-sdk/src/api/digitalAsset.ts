// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import {
  AnyNumber,
  GetCollectionDataResponse,
  GetCurrentTokenOwnershipResponse,
  GetOwnedTokensResponse,
  GetTokenActivityResponse,
  GetTokenDataResponse,
  MoveStructId,
  OrderByArg,
  PaginationArgs,
  TokenStandardArg,
} from "../types";
import { AccountAddress, AccountAddressInput } from "../core";
import { Account } from "../account";
import { InputGenerateTransactionOptions } from "../transactions/types";
import {
  addDigitalAssetPropertyTransaction,
  addDigitalAssetTypedPropertyTransaction,
  burnDigitalAssetTransaction,
  CreateCollectionOptions,
  createCollectionTransaction,
  freezeDigitalAssetTransferTransaction,
  getCollectionData,
  getCollectionDataByCollectionId,
  getCollectionId,
  getCurrentDigitalAssetOwnership,
  getDigitalAssetActivity,
  getDigitalAssetData,
  getOwnedDigitalAssets,
  mintDigitalAssetTransaction,
  mintSoulBoundTransaction,
  PropertyType,
  PropertyValue,
  removeDigitalAssetPropertyTransaction,
  setDigitalAssetDescriptionTransaction,
  setDigitalAssetNameTransaction,
  setDigitalAssetURITransaction,
  transferDigitalAssetTransaction,
  unfreezeDigitalAssetTransferTransaction,
  updateDigitalAssetPropertyTransaction,
  updateDigitalAssetTypedPropertyTransaction,
} from "../internal/digitalAsset";
import { ProcessorType } from "../utils/const";
import { AptosConfig } from "./aptosConfig";
import { waitForIndexerOnVersion } from "./utils";
import { SimpleTransaction } from "../transactions/instances/simpleTransaction";

/**
 * A class to query all `DigitalAsset` related queries on Aptos.
 */
export class DigitalAsset {
  constructor(readonly config: AptosConfig) {}

  /**
   * Queries data of a specific collection by the collection creator address and the collection name.
   *
   * If, for some reason, a creator account has 2 collections with the same name in v1 and v2,
   * can pass an optional `tokenStandard` parameter to query a specific standard
   *
   * @example
   * const collection = await aptos.getCollectionData({creatorAddress:"0x123",collectionName:"myCollection"})
   *
   * @param args.creatorAddress the address of the collection's creator
   * @param args.collectionName the name of the collection
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   * @param args.options.tokenStandard the token standard to query
   * @returns GetCollectionDataResponse response type
   */
  async getCollectionData(args: {
    creatorAddress: AccountAddressInput;
    collectionName: string;
    minimumLedgerVersion?: AnyNumber;
    options?: TokenStandardArg;
  }): Promise<GetCollectionDataResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getCollectionData({ aptosConfig: this.config, ...args });
  }

  /**
   * Queries data of a specific collection by the collection ID.
   *
   * @example
   * const collection = await aptos.getCollectionDataByCollectionId({collectionId:"0x123"})
   *
   * @param args.collectionId the ID of the collection, it's the same thing as the address of the collection object
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   * @returns GetCollectionDataResponse response type
   */
  async getCollectionDataByCollectionId(args: {
    collectionId: AccountAddressInput;
    minimumLedgerVersion?: AnyNumber;
  }): Promise<GetCollectionDataResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getCollectionDataByCollectionId({ aptosConfig: this.config, ...args });
  }

  /**
   * Queries a collection's ID.
   *
   * This is the same as the collection's object address in V2, but V1 does
   * not use objects, and does not have an address
   *
   * @example
   * const collection = await aptos.getCollectionId({creatorAddress:"0x123",collectionName:"myCollection"})
   *
   * @param args.creatorAddress the address of the collection's creator
   * @param args.collectionName the name of the collection
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   * @param args.options.tokenStandard the token standard to query
   * @returns the collection id
   */
  async getCollectionId(args: {
    creatorAddress: AccountAddressInput;
    collectionName: string;
    minimumLedgerVersion?: AnyNumber;
    options?: TokenStandardArg;
  }): Promise<string> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getCollectionId({ aptosConfig: this.config, ...args });
  }

  /**
   * Gets digital asset data given the address of a digital asset.
   *
   * @example
   * const digitalAsset = await aptos.getDigitalAssetData({digitalAssetAddress:"0x123"})
   *
   * @param args.tokenAddress The address of the digital asset
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   * @returns GetTokenDataResponse containing relevant data to the digital asset.
   */
  async getDigitalAssetData(args: {
    digitalAssetAddress: AccountAddressInput;
    minimumLedgerVersion?: AnyNumber;
  }): Promise<GetTokenDataResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getDigitalAssetData({ aptosConfig: this.config, ...args });
  }

  /**
   * Gets digital asset ownership data given the address of a digital asset.
   *
   * @example
   * const digitalAssetOwner = await aptos.getCurrentDigitalAssetOwnership({digitalAssetAddress:"0x123"})
   *
   * @param args.tokenAddress The address of the digital asset
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   *
   * @returns GetCurrentTokenOwnershipResponse containing relevant ownership data of the digital asset.
   */
  async getCurrentDigitalAssetOwnership(args: {
    digitalAssetAddress: AccountAddressInput;
    minimumLedgerVersion?: AnyNumber;
  }): Promise<GetCurrentTokenOwnershipResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getCurrentDigitalAssetOwnership({ aptosConfig: this.config, ...args });
  }

  /**
   * Gets the digital assets that the given address owns.
   *
   * @example
   * const digitalAssets = await aptos.getOwnedDigitalAssets({ownerAddress:"0x123"})
   *
   * @param args.ownerAddress The address of the owner
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   *
   * @returns GetOwnedTokensResponse containing ownership data of the digital assets belonging to the ownerAddresss.
   */
  async getOwnedDigitalAssets(args: {
    ownerAddress: AccountAddressInput;
    minimumLedgerVersion?: AnyNumber;
    options?: PaginationArgs & OrderByArg<GetOwnedTokensResponse[0]>;
  }): Promise<GetOwnedTokensResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getOwnedDigitalAssets({ aptosConfig: this.config, ...args });
  }

  /**
   * Gets the activity data given the address of a digital asset.
   *
   * @example
   * const digitalAssetActivity = await aptos.getDigitalAssetActivity({digitalAssetAddress:"0x123"})
   *
   * @param args.tokenAddress The address of the digital asset
   * @param args.minimumLedgerVersion Optional ledger version to sync up to, before querying
   *
   * @returns GetTokenActivityResponse containing relevant activity data to the digital asset.
   */
  async getDigitalAssetActivity(args: {
    digitalAssetAddress: AccountAddressInput;
    minimumLedgerVersion?: AnyNumber;
    options?: PaginationArgs & OrderByArg<GetTokenActivityResponse[0]>;
  }): Promise<GetTokenActivityResponse> {
    await waitForIndexerOnVersion({
      config: this.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: ProcessorType.TOKEN_V2_PROCESSOR,
    });
    return getDigitalAssetActivity({ aptosConfig: this.config, ...args });
  }

  /**
   * Creates a new collection within the specified account
   *
   * @example
   * const transaction = await aptos.createCollectionTransaction({
   *  creator: alice,
   *  description: "collectionDescription",
   *  name: "collectionName",
   *  uri: "collection-uri.com",
   * });
   *
   * @param args.creator the account of the collection's creator
   * @param args.description the description of the collection
   * @param args.name the name of the collection
   * @param args.uri the URI to additional info about the collection
   *
   * The parameters below are optional.
   * @param args.maxSupply controls the max supply of the digital assets - defaults MAX_U64_BIG_INT
   * @param args.mutableDescription controls mutability of the collection's description - defaults true
   * @param args.mutableRoyalty controls mutability of the collection's description - defaults true
   * @param args.mutableUri controls mutability of the collection's URI - defaults true
   * @param args.mutableTokenDescription controls mutability of the digital asset's description - defaults true
   * @param args.mutableTokenName controls mutability of the digital asset's name - defaults true
   * @param args.mutableTokenProperties controls mutability of digital asset's properties - defaults true
   * @param args.mutableTokenUri controls mutability of the digital asset's URI - defaults true
   * @param args.tokensBurnableByCreator controls whether digital assets can be burnable by the creator - defaults true
   * @param args.tokensFreezableByCreator controls whether digital assets can be frozen by the creator - defaults true
   * @param args.royaltyNumerator the numerator of the royalty to be paid to the creator when
   * a digital asset is transferred - defaults 0
   * @param args.royaltyDenominator the denominator of the royalty to be paid to the creator
   * when a digital asset is transferred - defaults 1
   *
   * @returns A SimpleTransaction that when submitted will create the collection.
   */
  async createCollectionTransaction(
    args: {
      creator: Account;
      description: string;
      name: string;
      uri: string;
      options?: InputGenerateTransactionOptions;
    } & CreateCollectionOptions,
  ): Promise<SimpleTransaction> {
    return createCollectionTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Create a transaction to mint a digital asset into the creators account within an existing collection.
   *
   * @example
   * const transaction = await aptos.mintDigitalAssetTransaction({
   *  creator: alice,
   *  description: "collectionDescription",
   *  collection: "collectionName",
   *  name: "digitalAssetName",
   *  uri: "digital-asset-uri.com",
   * });
   *
   * @param args.creator the creator of the collection
   * @param args.collection the name of the collection the digital asset belongs to
   * @param args.description the description of the digital asset
   * @param args.name the name of the digital asset
   * @param args.uri the URI to additional info about the digital asset
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async mintDigitalAssetTransaction(args: {
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
    return mintDigitalAssetTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Transfer a digital asset (non fungible digital asset) ownership.
   *
   * We can transfer a digital asset only when the digital asset is not frozen
   * (i.e. owner transfer is not disabled such as for soul bound digital assets)
   *
   * @example
   * const transaction = await aptos.transferDigitalAssetTransaction({
   *  sender: alice,
   *  digitalAssetAddress: "0x123",
   *  recipient: "0x456",
   * });
   *
   * @param args.sender The sender account of the current digital asset owner
   * @param args.digitalAssetAddress The digital asset address
   * @param args.recipient The recipient account address
   * @param args.digitalAssetType optional. The digital asset type, default to "0x4::token::Token"
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async transferDigitalAssetTransaction(args: {
    sender: Account;
    digitalAssetAddress: AccountAddressInput;
    recipient: AccountAddress;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }): Promise<SimpleTransaction> {
    return transferDigitalAssetTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Mint a soul bound digital asset into a recipient's account
   *
   * @example
   * const transaction = await aptos.mintSoulBoundTransaction({
   *  account: alice,
   *  collection: "collectionName",
   *  description: "collectionDescription",
   *  name: "digitalAssetName",
   *  uri: "digital-asset-uri.com",
   *  recipient: "0x123"
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.collection The collection name that the digital asset belongs to
   * @param args.description The digital asset description
   * @param args.name The digital asset name
   * @param args.uri The digital asset URL
   * @param args.recipient The account address where the digital asset will be created
   * @param args.propertyKeys The property keys for storing on-chain properties
   * @param args.propertyTypes The type of property values
   * @param args.propertyValues The property values to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async mintSoulBoundTransaction(args: {
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
    return mintSoulBoundTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Burn a digital asset by its creator
   *
   * @example
   * const transaction = await aptos.burnDigitalAssetTransaction({
   *  creator: alice,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async burnDigitalAssetTransaction(args: {
    creator: Account;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return burnDigitalAssetTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Freeze digital asset transfer ability
   *
   * @example
   * const transaction = await aptos.freezeDigitalAssetTransaferTransaction({
   *  creator: alice,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async freezeDigitalAssetTransaferTransaction(args: {
    creator: Account;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return freezeDigitalAssetTransferTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Unfreeze digital asset transfer ability
   *
   * @example
   * const transaction = await aptos.unfreezeDigitalAssetTransaferTransaction({
   *  creator: alice,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async unfreezeDigitalAssetTransaferTransaction(args: {
    creator: Account;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return unfreezeDigitalAssetTransferTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Set the digital asset description
   *
   * @example
   * const transaction = await aptos.setDigitalAssetDescriptionTransaction({
   *  creator: alice,
   *  description: "digitalAssetDescription",
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.description The digital asset description
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async setDigitalAssetDescriptionTransaction(args: {
    creator: Account;
    description: string;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return setDigitalAssetDescriptionTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Set the digital asset name
   *
   * @example
   * const transaction = await aptos.setDigitalAssetNameTransaction({
   *  creator: alice,
   *  name: "digitalAssetName",
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.name The digital asset name
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async setDigitalAssetNameTransaction(args: {
    creator: Account;
    name: string;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return setDigitalAssetNameTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Set the digital asset name
   *
   * @example
   * const transaction = await aptos.setDigitalAssetURITransaction({
   *  creator: alice,
   *  uri: "digital-asset-uri.com",
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.creator The creator account
   * @param args.uri The digital asset uri
   * @param args.digitalAssetAddress The digital asset address
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async setDigitalAssetURITransaction(args: {
    creator: Account;
    uri: string;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return setDigitalAssetURITransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Add a digital asset property
   *
   * @example
   * const transaction = await aptos.addDigitalAssetPropertyTransaction({
   *  creator: alice,
   *  propertyKey: "newKey",
   *  propertyType: "BOOLEAN",
   *  propertyValue: true,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.digitalAssetAddress The digital asset address
   * @param args.propertyKey The property key for storing on-chain properties
   * @param args.propertyType The type of property value
   * @param args.propertyValue The property value to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async addDigitalAssetPropertyTransaction(args: {
    creator: Account;
    propertyKey: string;
    propertyType: PropertyType;
    propertyValue: PropertyValue;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return addDigitalAssetPropertyTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Remove a digital asset property
   *
   * @example
   * const transaction = await aptos.removeDigitalAssetPropertyTransaction({
   *  creator: alice,
   *  propertyKey: "newKey",
   *  propertyType: "BOOLEAN",
   *  propertyValue: true,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.digitalAssetAddress The digital asset address
   * @param args.propertyKey The property key for storing on-chain properties
   * @param args.propertyType The type of property value
   * @param args.propertyValue The property value to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async removeDigitalAssetPropertyTransaction(args: {
    creator: Account;
    propertyKey: string;
    propertyType: PropertyType;
    propertyValue: PropertyValue;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return removeDigitalAssetPropertyTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Update a digital asset property
   *
   * @example
   * const transaction = await aptos.updateDigitalAssetPropertyTransaction({
   *  creator: alice,
   *  propertyKey: "newKey",
   *  propertyType: "BOOLEAN",
   *  propertyValue: false,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.digitalAssetAddress The digital asset address
   * @param args.propertyKey The property key for storing on-chain properties
   * @param args.propertyType The type of property value
   * @param args.propertyValue The property value to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async updateDigitalAssetPropertyTransaction(args: {
    creator: Account;
    propertyKey: string;
    propertyType: PropertyType;
    propertyValue: PropertyValue;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return updateDigitalAssetPropertyTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Add a typed digital asset property
   *
   * @example
   * const transaction = await aptos.addDigitalAssetTypedPropertyTransaction({
   *  creator: alice,
   *  propertyKey: "typedKey",
   *  propertyType: "STRING",
   *  propertyValue: "hello",
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.digitalAssetAddress The digital asset address
   * @param args.propertyKey The property key for storing on-chain properties
   * @param args.propertyType The type of property value
   * @param args.propertyValue The property value to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async addDigitalAssetTypedPropertyTransaction(args: {
    creator: Account;
    propertyKey: string;
    propertyType: PropertyType;
    propertyValue: PropertyValue;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return addDigitalAssetTypedPropertyTransaction({ aptosConfig: this.config, ...args });
  }

  /**
   * Update a typed digital asset property
   *
   * @example
   * const transaction = await aptos.updateDigitalAssetTypedPropertyTransaction({
   *  creator: alice,
   *  propertyKey: "typedKey",
   *  propertyType: "U8",
   *  propertyValue: 2,
   *  digitalAssetAddress: "0x123",
   * });
   *
   * @param args.account The account that mints the digital asset
   * @param args.digitalAssetAddress The digital asset address
   * @param args.propertyKey The property key for storing on-chain properties
   * @param args.propertyType The type of property value
   * @param args.propertyValue The property value to be stored on-chain
   *
   * @returns A SimpleTransaction that can be simulated or submitted to chain
   */
  async updateDigitalAssetTypedPropertyTransaction(args: {
    creator: Account;
    propertyKey: string;
    propertyType: PropertyType;
    propertyValue: PropertyValue;
    digitalAssetAddress: AccountAddressInput;
    digitalAssetType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }) {
    return updateDigitalAssetTypedPropertyTransaction({ aptosConfig: this.config, ...args });
  }
}
