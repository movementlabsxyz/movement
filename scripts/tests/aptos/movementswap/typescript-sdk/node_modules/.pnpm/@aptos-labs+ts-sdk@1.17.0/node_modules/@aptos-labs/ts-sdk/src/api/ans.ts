// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { Account } from "../account";
import { AccountAddress, AccountAddressInput } from "../core";
import {
  RegisterNameParameters,
  getExpiration,
  getOwnerAddress,
  registerName,
  getPrimaryName,
  setPrimaryName,
  getTargetAddress,
  setTargetAddress,
  renewDomain,
  getName,
  getAccountDomains,
  GetAccountDomainsArgs,
  GetAccountSubdomainsArgs,
  getAccountSubdomains,
  getAccountNames,
  GetAccountNamesArgs,
  getDomainSubdomains,
  GetDomainSubdomainsArgs,
} from "../internal/ans";
import { GetANSNameResponse } from "../types";
import { InputGenerateTransactionOptions } from "../transactions/types";
import { AptosConfig } from "./aptosConfig";
import { SimpleTransaction } from "../transactions/instances/simpleTransaction";

/**
 * A class to handle all `ANS` operations
 */
export class ANS {
  constructor(readonly config: AptosConfig) {}

  /**
   * Retrieve the owner address of a domain name or subdomain name from the contract.
   *
   * @example
   * // Will return the owner address of "test.aptos.apt" or undefined
   * const owner = await aptos.getOwnerAddress({name: "test.aptos"})
   * // owner = 0x123...
   *
   * @param args.name - A string of the name to retrieve
   *
   * @returns AccountAddress if the name is owned, undefined otherwise
   */
  async getOwnerAddress(args: { name: string }): Promise<AccountAddress | undefined> {
    return getOwnerAddress({ aptosConfig: this.config, ...args });
  }

  /**
   * Retrieve the expiration time of a domain name or subdomain name from the contract.
   *
   * @example
   * // Will return the expiration of "test.aptos.apt" or undefined
   * const exp = await aptos.getExpiration({name: "test.aptos"})
   * // new Date(exp) would give you the date in question: 2021-01-01T00:00:00.000Z
   *
   * @param args.name - A string of the name to retrieve
   *
   * @returns number as a unix timestamp in milliseconds.
   */
  async getExpiration(args: { name: string }): Promise<number | undefined> {
    return getExpiration({ aptosConfig: this.config, ...args });
  }

  /**
   * Retrieve the target address of a domain or subdomain name. This is the
   * address the name points to for use on chain. Note, the target address can
   * point to addresses that are not the owner of the name
   *
   * @example
   * const targetAddr = await aptos.getTargetAddress({name: "test.aptos"})
   * // targetAddr = 0x123...
   *
   * @param args.name - A string of the name: primary, primary.apt, secondary.primary, secondary.primary.apt, etc.
   *
   * @returns AccountAddress if the name has a target, undefined otherwise
   */
  async getTargetAddress(args: { name: string }): Promise<AccountAddress | undefined> {
    return getTargetAddress({ aptosConfig: this.config, ...args });
  }

  /**
   * Sets the target address of a domain or subdomain name. This is the
   * address the name points to for use on chain. Note, the target address can
   * point to addresses that are not the owner of the name
   *
   * @example
   * await aptos.setTargetAddress({sender: alice, name: "test.aptos", address: bob.accountAddress})
   * const address = await aptos.getTargetAddress({name: "test.aptos"})
   * // address = bob.accountAddress
   *
   * @param args.name - A string of the name: test.aptos.apt, test.apt, test, test.aptos, etc.
   * @param args.address - A AccountAddressInput of the address to set the domain or subdomain to
   *
   * @returns SimpleTransaction
   */
  async setTargetAddress(args: {
    sender: Account;
    name: string;
    address: AccountAddressInput;
    options?: InputGenerateTransactionOptions;
  }): Promise<SimpleTransaction> {
    return setTargetAddress({ aptosConfig: this.config, ...args });
  }

  /**
   * Retrieve the primary name for an account. An account can have
   * multiple names that target it, but only a single name that is primary. An
   * account also may not have a primary name.
   *
   * @example
   * const name = await aptos.getPrimaryName({address: alice.accountAddress})
   * // name = test.aptos
   *
   * @param args.address - A AccountAddressInput (address) of the account
   *
   * @returns a string if the account has a primary name, undefined otherwise
   */
  async getPrimaryName(args: { address: AccountAddressInput }): Promise<string | undefined> {
    return getPrimaryName({ aptosConfig: this.config, ...args });
  }

  /**
   * Sets the primary name for the sender. An account can have
   * multiple names that target it, but only a single name that is primary. An
   * account also may not have a primary name.
   *
   * @example
   * await aptos.setPrimaryName({sender: alice, name: "test.aptos"})
   * const primaryName = await aptos.getPrimaryName({address: alice.accountAddress})
   * // primaryName = test.aptos
   *
   * @param args.sender - The sender account
   * @param args.name - A string of the name: test, test.apt, test.aptos, test.aptos.apt, etc.
   *
   * @returns SimpleTransaction
   */
  async setPrimaryName(args: {
    sender: Account;
    name?: string;
    options?: InputGenerateTransactionOptions;
  }): Promise<SimpleTransaction> {
    return setPrimaryName({ aptosConfig: this.config, ...args });
  }

  /**
   * Registers a new name
   *
   * @example
   * // An example of registering a subdomain name assuming def.apt is already registered
   * // and belongs to the sender alice.
   *  const txn = aptos.registerName({
   *    sender: alice,
   *    name: "test.aptos.apt",
   *    expiration: {
   *      policy: "subdomain:independent",
   *      expirationDate: Date.now() + 30 * 24 * 60 * 60 * 1000,
   *    },
   *  });
   *
   * @param args.sender - The sender account
   * @param args.name - A string of the name to register. This can be inclusive or exclusive of the .apt suffix.
   * Examples include: "test", "test.apt", "test.aptos.apt", etc.
   * @param args.expiration  - An object with the expiration policy of the name.
   * @param args.expiration.policy - 'domain' | 'subdomain:follow-domain' | 'subdomain:independent'
   * - domain: Years is required and the name will expire after the given number of years.
   * - subdomain:follow-domain: The name will expire at the same time as the domain name.
   * - subdomain:independent: The name will expire at the given date.
   * @param args.expiration.expirationDate - An epoch number in milliseconds of
   * the date when the subdomain will expire. Only applicable when the policy is
   * set to 'subdomain:independent'.
   * @param args.transferable  - Determines if the subdomain being minted is soul-bound. Applicable only to subdomains.
   * @param args.targetAddress optional - The address the domain name will resolve to. If not provided,
   * the sender's address will be used.
   * @param args.toAddress optional - The address to send the domain name to. If not provided,
   * the transaction will be sent to the router.
   *
   * @returns SimpleTransaction
   */
  async registerName(args: Omit<RegisterNameParameters, "aptosConfig">): Promise<SimpleTransaction> {
    return registerName({ aptosConfig: this.config, ...args });
  }

  /**
   * Renews a domain name
   *
   * Note: If a domain name was minted with V1 of the contract, it will automatically be upgraded to V2 via this transaction.
   *
   * @example
   * await aptos.renewDomain({sender: alice, name: "test"})
   * // test.apt will be renewed for one year
   *
   * @param args.sender - The sender account
   * @param args.name - A string of the domain the subdomain will be under. The signer must be the domain owner.
   * Subdomains cannot be renewed.
   * @param args.years - The number of years to renew the name. Currently only one year is permitted.
   *
   * @returns SimpleTransaction
   */
  async renewDomain(args: {
    sender: Account;
    name: string;
    years?: 1;
    options?: InputGenerateTransactionOptions;
  }): Promise<SimpleTransaction> {
    return renewDomain({ aptosConfig: this.config, ...args });
  }

  /**
   * Fetches a single name from the indexer
   * @param args.name - A string of the name to retrieve, e.g. "test.aptos.apt"
   * or "test.apt" or "test". Can be inclusive or exclusive of the .apt suffix.
   * Can be a subdomain.
   *
   * @returns A promise of an ANSName or undefined
   */
  async getName(args: { name: string }): Promise<GetANSNameResponse[0] | undefined> {
    return getName({ aptosConfig: this.config, ...args });
  }

  /**
   * Fetches all  names for an account (both top level domains and subdomains)
   *
   * @param args
   * @param args.accountAddress - A AccountAddressInput of the address to retrieve names for.
   * @param args.options.offset - Optional, the offset to start from when fetching names
   * @param args.options.limit - Optional, A number of the names to fetch per request
   * @param args.options.orderBy - The order to sort the names by
   * @param args.options.where - Additional filters to apply to the query
   *
   * @returns a promise of an array of ANSName
   */
  async getAccountNames(args: GetAccountNamesArgs): Promise<GetANSNameResponse> {
    return getAccountNames({ aptosConfig: this.config, ...args });
  }

  /**
   * Fetches all top level domain names for an account
   *
   * @param args
   * @param args.accountAddress - A AccountAddressInput of the address to retrieve domain names for.
   * @param args.options.offset - Optional, the offset to start from when fetching names
   * @param args.options.limit - Optional, A number of the names to fetch per request
   * @param args.options.orderBy - The order to sort the names by
   * @param args.options.where - Additional filters to apply to the query
   *
   * @returns a promise of an array of ANSName
   */
  async getAccountDomains(args: GetAccountDomainsArgs): Promise<GetANSNameResponse> {
    return getAccountDomains({ aptosConfig: this.config, ...args });
  }

  /**
   * Fetches all subdomains names for an account
   *
   * @param args
   * @param args.accountAddress - A AccountAddressInput of the address to retrieve subdomains names for.
   * @param args.options.offset - Optional, the offset to start from when fetching names
   * @param args.options.limit - Optional, A number of the names to fetch per request
   * @param args.options.orderBy - The order to sort the names by
   * @param args.options.where - Additional filters to apply to the query
   *
   * @returns a promise of an array of ANSName
   */
  async getAccountSubdomains(args: GetAccountSubdomainsArgs): Promise<GetANSNameResponse> {
    return getAccountSubdomains({ aptosConfig: this.config, ...args });
  }

  /**
   * Fetches all subdomains names for a given domain. Note, this will not return the domain itself.
   *
   * @param args
   * @param args.domain - A string of the domain name: eg. "test.apt" or "test" (without the suffix of .apt)
   * @param args.options.offset - Optional, the offset to start from when fetching names
   * @param args.options.limit - Optional, A number of the names to fetch per request
   * @param args.options.orderBy - The order to sort the names by
   * @param args.options.where - Additional filters to apply to the query
   *
   * @returns a promise of an array of ANSName
   */
  async getDomainSubdomains(args: GetDomainSubdomainsArgs): Promise<GetANSNameResponse> {
    return getDomainSubdomains({ aptosConfig: this.config, ...args });
  }
}
