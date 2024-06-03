// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/name}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * name namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptosConfig";
import { Account } from "../account";
import { AccountAddress, AccountAddressInput } from "../core";
import { InputGenerateTransactionOptions } from "../transactions/types";
import { GetANSNameResponse, MoveAddressType, OrderByArg, PaginationArgs, WhereArg } from "../types";
import { GetNamesQuery } from "../types/generated/operations";
import { GetNames } from "../types/generated/queries";
import { CurrentAptosNamesBoolExp } from "../types/generated/types";
import { Network } from "../utils/apiEndpoints";
import { queryIndexer } from "./general";
import { view } from "./view";
import { generateTransaction } from "./transactionSubmission";
import { SimpleTransaction } from "../transactions/instances/simpleTransaction";

export const VALIDATION_RULES_DESCRIPTION = [
  "A name must be between 3 and 63 characters long,",
  "and can only contain lowercase a-z, 0-9, and hyphens.",
  "A name may not start or end with a hyphen.",
].join(" ");

/**
 *
 * @param fragment A fragment of a name, either the domain or subdomain
 * @returns boolean indicating if the fragment is a valid fragment
 */
export function isValidANSSegment(fragment: string): boolean {
  if (!fragment) return false;
  if (fragment.length < 3) return false;
  if (fragment.length > 63) return false;
  // only lowercase a-z and 0-9 are allowed, along with -. a domain may not start or end with a hyphen
  if (!/^[a-z\d][a-z\d-]{1,61}[a-z\d]$/.test(fragment)) return false;
  return true;
}

/**
 * Checks if an ANS name is valid or not
 *
 * @param name A string of the domain name, can include or exclude the .apt suffix
 */
export function isValidANSName(name: string): { domainName: string; subdomainName?: string } {
  const [first, second, ...rest] = name.replace(/\.apt$/, "").split(".");

  if (rest.length > 0) {
    throw new Error(`${name} is invalid. A name can only have two parts, a domain and a subdomain separated by a "."`);
  }

  if (!isValidANSSegment(first)) {
    throw new Error(`${first} is not valid. ${VALIDATION_RULES_DESCRIPTION}`);
  }

  if (second && !isValidANSSegment(second)) {
    throw new Error(`${second} is not valid. ${VALIDATION_RULES_DESCRIPTION}`);
  }

  return {
    domainName: second || first,
    subdomainName: second ? first : undefined,
  };
}

export const LOCAL_ANS_ACCOUNT_PK =
  process.env.ANS_TEST_ACCOUNT_PRIVATE_KEY ?? "0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221";
export const LOCAL_ANS_ACCOUNT_ADDRESS =
  process.env.ANS_TEST_ACCOUNT_ADDRESS ?? "0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82";

const NetworkToAnsContract: Record<Network, string | null> = {
  [Network.TESTNET]: "0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c",
  [Network.MAINNET]: "0x867ed1f6bf916171b1de3ee92849b8978b7d1b9e0a8cc982a3d19d535dfd9c0c",
  [Network.LOCAL]: LOCAL_ANS_ACCOUNT_ADDRESS,
  [Network.CUSTOM]: null,
  [Network.DEVNET]: null,
};

function getRouterAddress(aptosConfig: AptosConfig): string {
  const address = NetworkToAnsContract[aptosConfig.network];
  if (!address) throw new Error(`The ANS contract is not deployed to ${aptosConfig.network}`);
  return address;
}

const unwrapOption = <T>(option: any): T | undefined => {
  if (!!option && typeof option === "object" && "vec" in option && Array.isArray(option.vec)) {
    return option.vec[0];
  }

  return undefined;
};

export async function getOwnerAddress(args: {
  aptosConfig: AptosConfig;
  name: string;
}): Promise<AccountAddress | undefined> {
  const { aptosConfig, name } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const { domainName, subdomainName } = isValidANSName(name);

  const res = await view({
    aptosConfig,
    payload: {
      function: `${routerAddress}::router::get_owner_addr`,
      functionArguments: [domainName, subdomainName],
    },
  });

  const owner = unwrapOption<MoveAddressType>(res[0]);

  return owner ? AccountAddress.from(owner) : undefined;
}

export interface RegisterNameParameters {
  aptosConfig: AptosConfig;
  sender: Account;
  name: string;
  expiration:
    | { policy: "domain"; years?: 1 }
    | { policy: "subdomain:follow-domain" }
    | { policy: "subdomain:independent"; expirationDate: number };
  transferable?: boolean;
  toAddress?: AccountAddressInput;
  targetAddress?: AccountAddressInput;
  options?: InputGenerateTransactionOptions;
}

export async function registerName(args: RegisterNameParameters): Promise<SimpleTransaction> {
  const { aptosConfig, expiration, name, sender, targetAddress, toAddress, options, transferable } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const { domainName, subdomainName } = isValidANSName(name);

  const hasSubdomainPolicy =
    expiration.policy === "subdomain:independent" || expiration.policy === "subdomain:follow-domain";

  if (subdomainName && !hasSubdomainPolicy) {
    throw new Error(
      "Subdomains must have an expiration policy of either 'subdomain:independent' or 'subdomain:follow-domain'",
    );
  }

  if (hasSubdomainPolicy && !subdomainName) {
    throw new Error(`Policy is set to ${expiration.policy} but no subdomain was provided`);
  }

  if (expiration.policy === "domain") {
    const years = expiration.years ?? 1;
    if (years !== 1) {
      throw new Error("For now, names can only be registered for 1 year at a time");
    }

    const secondsInYear = 31536000;
    const registrationDuration = years * secondsInYear;

    const transaction = await generateTransaction({
      aptosConfig,
      sender: sender.accountAddress.toString(),
      data: {
        function: `${routerAddress}::router::register_domain`,
        functionArguments: [domainName, registrationDuration, targetAddress, toAddress],
      },
      options,
    });

    return transaction;
  }

  // We are a subdomain
  if (!subdomainName) {
    throw new Error(`${expiration.policy} requires a subdomain to be provided.`);
  }

  const tldExpiration = await getExpiration({ aptosConfig, name: domainName });
  if (!tldExpiration) {
    throw new Error("The domain does not exist");
  }

  const expirationDateInMillisecondsSinceEpoch =
    expiration.policy === "subdomain:independent" ? expiration.expirationDate : tldExpiration;

  if (expirationDateInMillisecondsSinceEpoch > tldExpiration) {
    throw new Error("The subdomain expiration time cannot be greater than the domain expiration time");
  }

  const transaction = await generateTransaction({
    aptosConfig,
    sender: sender.accountAddress.toString(),
    data: {
      function: `${routerAddress}::router::register_subdomain`,
      functionArguments: [
        domainName,
        subdomainName,
        Math.round(expirationDateInMillisecondsSinceEpoch / 1000),
        expiration.policy === "subdomain:follow-domain" ? 1 : 0,
        !!transferable,
        targetAddress,
        toAddress,
      ],
    },
    options,
  });

  return transaction;
}

export async function getExpiration(args: { aptosConfig: AptosConfig; name: string }): Promise<number | undefined> {
  const { aptosConfig, name } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const { domainName, subdomainName } = isValidANSName(name);

  try {
    const res = await view({
      aptosConfig,
      payload: {
        function: `${routerAddress}::router::get_expiration`,
        functionArguments: [domainName, subdomainName],
      },
    });

    // Normalize expiration time from epoch seconds to epoch milliseconds
    return Number(res[0]) * 1000;
  } catch (e) {
    return undefined;
  }
}

export async function getPrimaryName(args: {
  aptosConfig: AptosConfig;
  address: AccountAddressInput;
}): Promise<string | undefined> {
  const { aptosConfig, address } = args;
  const routerAddress = getRouterAddress(aptosConfig);

  const res = await view({
    aptosConfig,
    payload: {
      function: `${routerAddress}::router::get_primary_name`,
      functionArguments: [AccountAddress.from(address).toString()],
    },
  });

  const domainName = unwrapOption<MoveAddressType>(res[1]);
  const subdomainName = unwrapOption<MoveAddressType>(res[0]);

  if (!domainName) return undefined;

  return [subdomainName, domainName].filter(Boolean).join(".");
}

export async function setPrimaryName(args: {
  aptosConfig: AptosConfig;
  sender: Account;
  name?: string;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, sender, name, options } = args;
  const routerAddress = getRouterAddress(aptosConfig);

  if (!name) {
    const transaction = await generateTransaction({
      aptosConfig,
      sender: sender.accountAddress.toString(),
      data: {
        function: `${routerAddress}::router::clear_primary_name`,
        functionArguments: [],
      },
      options,
    });

    return transaction;
  }

  const { domainName, subdomainName } = isValidANSName(name);

  const transaction = await generateTransaction({
    aptosConfig,
    sender: sender.accountAddress.toString(),
    data: {
      function: `${routerAddress}::router::set_primary_name`,
      functionArguments: [domainName, subdomainName],
    },
    options,
  });

  return transaction;
}

export async function getTargetAddress(args: {
  aptosConfig: AptosConfig;
  name: string;
}): Promise<AccountAddress | undefined> {
  const { aptosConfig, name } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const { domainName, subdomainName } = isValidANSName(name);

  const res = await view({
    aptosConfig,
    payload: {
      function: `${routerAddress}::router::get_target_addr`,
      functionArguments: [domainName, subdomainName],
    },
  });

  const target = unwrapOption<MoveAddressType>(res[0]);
  return target ? AccountAddress.from(target) : undefined;
}

export async function setTargetAddress(args: {
  aptosConfig: AptosConfig;
  sender: Account;
  name: string;
  address: AccountAddressInput;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, sender, name, address, options } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const { domainName, subdomainName } = isValidANSName(name);

  const transaction = await generateTransaction({
    aptosConfig,
    sender: sender.accountAddress.toString(),
    data: {
      function: `${routerAddress}::router::set_target_addr`,
      functionArguments: [domainName, subdomainName, address],
    },
    options,
  });

  return transaction;
}

export async function getName(args: {
  aptosConfig: AptosConfig;
  name: string;
}): Promise<GetANSNameResponse[0] | undefined> {
  const { aptosConfig, name } = args;
  const { domainName, subdomainName = "" } = isValidANSName(name);

  const where: CurrentAptosNamesBoolExp = {
    domain: { _eq: domainName },
    subdomain: { _eq: subdomainName },
    is_active: { _eq: true },
  };

  const data = await queryIndexer<GetNamesQuery>({
    aptosConfig,
    query: {
      query: GetNames,
      variables: {
        where_condition: where,
        limit: 1,
      },
    },
    originMethod: "getName",
  });

  // Convert the expiration_timestamp from an ISO string to milliseconds since epoch
  let res = data.current_aptos_names[0];
  if (res) {
    res = sanitizeANSName(res);
  }

  return res;
}

interface QueryNamesOptions {
  options?: PaginationArgs & OrderByArg<GetANSNameResponse[0]> & WhereArg<CurrentAptosNamesBoolExp>;
}

export interface GetAccountNamesArgs extends QueryNamesOptions {
  accountAddress: AccountAddressInput;
}

export async function getAccountNames(
  args: { aptosConfig: AptosConfig } & GetAccountNamesArgs,
): Promise<GetANSNameResponse> {
  const { aptosConfig, options, accountAddress } = args;

  const expirationDate = await getANSExpirationDate({ aptosConfig });

  const data = await queryIndexer<GetNamesQuery>({
    aptosConfig,
    originMethod: "getAccountNames",
    query: {
      query: GetNames,
      variables: {
        limit: options?.limit,
        offset: options?.offset,
        order_by: options?.orderBy,
        where_condition: {
          ...(args.options?.where ?? {}),
          owner_address: { _eq: accountAddress.toString() },
          expiration_timestamp: { _gte: expirationDate },
        },
      },
    },
  });

  return data.current_aptos_names.map(sanitizeANSName);
}

export interface GetAccountDomainsArgs extends QueryNamesOptions {
  accountAddress: AccountAddressInput;
}

export async function getAccountDomains(
  args: { aptosConfig: AptosConfig } & GetAccountDomainsArgs,
): Promise<GetANSNameResponse> {
  const { aptosConfig, options, accountAddress } = args;

  const expirationDate = await getANSExpirationDate({ aptosConfig });

  const data = await queryIndexer<GetNamesQuery>({
    aptosConfig,
    originMethod: "getAccountDomains",
    query: {
      query: GetNames,
      variables: {
        limit: options?.limit,
        offset: options?.offset,
        order_by: options?.orderBy,
        where_condition: {
          ...(args.options?.where ?? {}),
          owner_address: { _eq: accountAddress.toString() },
          expiration_timestamp: { _gte: expirationDate },
          subdomain: { _eq: "" },
        },
      },
    },
  });

  return data.current_aptos_names.map(sanitizeANSName);
}

export interface GetAccountSubdomainsArgs extends QueryNamesOptions {
  accountAddress: AccountAddressInput;
}

export async function getAccountSubdomains(
  args: { aptosConfig: AptosConfig } & GetAccountSubdomainsArgs,
): Promise<GetANSNameResponse> {
  const { aptosConfig, options, accountAddress } = args;

  const expirationDate = await getANSExpirationDate({ aptosConfig });

  const data = await queryIndexer<GetNamesQuery>({
    aptosConfig,
    originMethod: "getAccountSubdomains",
    query: {
      query: GetNames,
      variables: {
        limit: options?.limit,
        offset: options?.offset,
        order_by: options?.orderBy,
        where_condition: {
          ...(args.options?.where ?? {}),
          owner_address: { _eq: accountAddress.toString() },
          expiration_timestamp: { _gte: expirationDate },
          subdomain: { _neq: "" },
        },
      },
    },
  });

  return data.current_aptos_names.map(sanitizeANSName);
}

export interface GetDomainSubdomainsArgs extends QueryNamesOptions {
  domain: string;
}

export async function getDomainSubdomains(
  args: { aptosConfig: AptosConfig } & GetDomainSubdomainsArgs,
): Promise<GetANSNameResponse> {
  const { aptosConfig, options, domain } = args;

  const data = await queryIndexer<GetNamesQuery>({
    aptosConfig,
    originMethod: "getDomainSubdomains",
    query: {
      query: GetNames,
      variables: {
        limit: options?.limit,
        offset: options?.offset,
        order_by: options?.orderBy,
        where_condition: {
          ...(args.options?.where ?? {}),
          domain: { _eq: domain },
          subdomain: { _neq: "" },
          is_active: { _eq: true },
        },
      },
    },
  });

  return data.current_aptos_names.map(sanitizeANSName);
}

/**
 * This function returns the expiration date in which a name is fully expired as
 * defined by the contract.  The grace period allows for names to be past
 * expiration for a certain amount of time before they are released to the
 * public. The names will not function as normal, but the owner can renew
 * without others taking ownership of the name. At the time of writing, the
 * contract specified 30 days.
 *
 * @param args.aptosConfig an AptosConfig object
 * @returns
 */
async function getANSExpirationDate(args: { aptosConfig: AptosConfig }): Promise<string> {
  const { aptosConfig } = args;
  const routerAddress = getRouterAddress(aptosConfig);

  const [gracePeriodInSeconds] = await view<[number]>({
    aptosConfig,
    payload: {
      function: `${routerAddress}::config::reregistration_grace_sec`,
      functionArguments: [],
    },
  });

  const gracePeriodInDays = gracePeriodInSeconds / 60 / 60 / 24;
  const now = () => new Date();
  return new Date(now().setDate(now().getDate() - gracePeriodInDays)).toISOString();
}

export async function renewDomain(args: {
  aptosConfig: AptosConfig;
  sender: Account;
  name: string;
  years?: 1;
  options?: InputGenerateTransactionOptions;
}): Promise<SimpleTransaction> {
  const { aptosConfig, sender, name, years = 1, options } = args;
  const routerAddress = getRouterAddress(aptosConfig);
  const renewalDuration = years * 31536000;
  const { domainName, subdomainName } = isValidANSName(name);

  if (subdomainName) {
    throw new Error("Subdomains cannot be renewed");
  }

  if (years !== 1) {
    throw new Error("Currently, only 1 year renewals are supported");
  }

  const transaction = await generateTransaction({
    aptosConfig,
    sender: sender.accountAddress.toString(),
    data: {
      function: `${routerAddress}::router::renew_domain`,
      functionArguments: [domainName, renewalDuration],
    },
    options,
  });

  return transaction;
}

/**
 * The indexer returns ISO strings for expiration, however the contract works in
 * epoch milliseconds. This function converts the ISO string to epoch
 * milliseconds. In the future, if other properties need sanitization, this can
 * be extended.
 */
function sanitizeANSName(name: GetANSNameResponse[0]): GetANSNameResponse[0] {
  return {
    ...name,
    expiration_timestamp: new Date(name.expiration_timestamp).getTime(),
  };
}
