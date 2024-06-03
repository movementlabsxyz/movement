// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { fundAccount } from "../internal/faucet";
import { UserTransactionResponse, WaitForTransactionOptions } from "../types";
import { AccountAddressInput } from "../core";
import { AptosConfig } from "./aptosConfig";
import { waitForIndexer } from "../internal/transaction";

/**
 * A class to query all `Faucet` related queries on Aptos.
 */
export class Faucet {
  constructor(readonly config: AptosConfig) {}

  /**
   * This creates an account if it does not exist and mints the specified amount of
   * coins into that account
   *
   * @example
   * const transaction = await aptos.fundAccount({accountAddress:"0x123", amount: 100})
   *
   * @param args.accountAddress Address of the account to fund
   * @param args.amount Amount of tokens to fund the account with
   * @param args.options Configuration options for waitForTransaction
   * @returns Transaction hash of the transaction that funded the account
   */
  async fundAccount(args: {
    accountAddress: AccountAddressInput;
    amount: number;
    options?: WaitForTransactionOptions;
  }): Promise<UserTransactionResponse> {
    const fundTxn = await fundAccount({ aptosConfig: this.config, ...args });

    if (args.options?.waitForIndexer !== false) {
      await waitForIndexer({ aptosConfig: this.config, minimumLedgerVersion: BigInt(fundTxn.version) });
    }

    return fundTxn;
  }
}
