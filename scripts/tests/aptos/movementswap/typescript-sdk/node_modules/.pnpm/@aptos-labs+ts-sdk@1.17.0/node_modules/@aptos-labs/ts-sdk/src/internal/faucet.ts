// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/faucet}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * faucet namespace and without having a dependency cycle error.
 */

import { AptosConfig } from "../api/aptosConfig";
import { postAptosFaucet } from "../client";
import { AccountAddress, AccountAddressInput } from "../core";
import { TransactionResponseType, UserTransactionResponse, WaitForTransactionOptions } from "../types";
import { DEFAULT_TXN_TIMEOUT_SEC } from "../utils/const";
import { waitForTransaction } from "./transaction";

export async function fundAccount(args: {
  aptosConfig: AptosConfig;
  accountAddress: AccountAddressInput;
  amount: number;
  options?: WaitForTransactionOptions;
}): Promise<UserTransactionResponse> {
  const { aptosConfig, accountAddress, amount, options } = args;
  const timeout = options?.timeoutSecs || DEFAULT_TXN_TIMEOUT_SEC;
  const { data } = await postAptosFaucet<any, { txn_hashes: Array<string> }>({
    aptosConfig,
    path: "fund",
    body: {
      address: AccountAddress.from(accountAddress).toString(),
      amount,
    },
    originMethod: "fundAccount",
  });

  const txnHash = data.txn_hashes[0];

  const res = await waitForTransaction({
    aptosConfig,
    transactionHash: txnHash,
    options: {
      timeoutSecs: timeout,
      checkSuccess: options?.checkSuccess,
    },
  });

  // Response is always User transaction for a user submitted transaction
  if (res.type === TransactionResponseType.User) {
    return res;
  }

  throw new Error(`Unexpected transaction received for fund account: ${res.type}`);
}
