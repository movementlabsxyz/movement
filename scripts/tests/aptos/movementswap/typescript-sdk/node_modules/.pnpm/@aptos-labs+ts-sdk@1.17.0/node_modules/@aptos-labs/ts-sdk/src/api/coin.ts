// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AccountAddressInput } from "../core";
import { transferCoinTransaction } from "../internal/coin";
import { SimpleTransaction } from "../transactions/instances/simpleTransaction";
import { InputGenerateTransactionOptions } from "../transactions/types";
import { AnyNumber, MoveStructId } from "../types";
import { AptosConfig } from "./aptosConfig";

/**
 * A class to handle all `Coin` operations
 */
export class Coin {
  constructor(readonly config: AptosConfig) {}

  /**
   * Generate a transfer coin transaction that can be simulated and/or signed and submitted
   *
   * @example
   * const transferCoinTransaction = await aptos.transferCoinTransaction({
   * sender: "0x123",
   * recipient:"0x456",
   * amount: 10,
   * })
   *
   * @param args.sender The sender account
   * @param args.recipient The recipient address
   * @param args.amount The amount to transfer
   * @param args.coinType optional. The coin struct type to transfer. Defaults to 0x1::aptos_coin::AptosCoin
   *
   * @returns SimpleTransaction
   */
  async transferCoinTransaction(args: {
    sender: AccountAddressInput;
    recipient: AccountAddressInput;
    amount: AnyNumber;
    coinType?: MoveStructId;
    options?: InputGenerateTransactionOptions;
  }): Promise<SimpleTransaction> {
    return transferCoinTransaction({ aptosConfig: this.config, ...args });
  }
}
