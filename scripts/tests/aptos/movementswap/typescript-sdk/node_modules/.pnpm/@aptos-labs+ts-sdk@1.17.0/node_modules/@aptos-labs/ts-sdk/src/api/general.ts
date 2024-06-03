// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { AptosConfig } from "./aptosConfig";
import {
  getChainTopUserTransactions,
  getIndexerLastSuccessVersion,
  getLedgerInfo,
  getProcessorStatus,
  queryIndexer,
} from "../internal/general";
import { getBlockByHeight, getBlockByVersion } from "../internal/transaction";
import { view } from "../internal/view";
import {
  AnyNumber,
  Block,
  GetChainTopUserTransactionsResponse,
  GetProcessorStatusResponse,
  GraphqlQuery,
  LedgerInfo,
  LedgerVersionArg,
  MoveValue,
} from "../types";
import { ProcessorType } from "../utils/const";
import { InputViewFunctionData } from "../transactions";

/**
 * A class to query all `General` Aptos related queries
 */
export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries for the Aptos ledger info
   *
   * @returns Aptos Ledger Info
   *
   * @example
   * const ledgerInfo = await aptos.getLedgerInfo()
   * // an example of the returned data
   * ```
   * {
   * "chain_id": 4,
   * "epoch": "8",
   * "ledger_version": "714",
   * "oldest_ledger_version": "0",
   * "ledger_timestamp": "1694695496521775",
   * "node_role": "validator",
   * "oldest_block_height": "0",
   * "block_height": "359",
   * "git_hash": "c82193f36f4e185fed9f68c4ad21f6c6dd390c6e"
   * }
   * ```
   */
  async getLedgerInfo(): Promise<LedgerInfo> {
    return getLedgerInfo({ aptosConfig: this.config });
  }

  /**
   * Queries for the chain id
   *
   * @example
   * const chainId = await aptos.getChainId()
   *
   * @returns The chain id
   */
  async getChainId(): Promise<number> {
    const result = await this.getLedgerInfo();
    return result.chain_id;
  }

  /**
   * Queries for block by transaction version
   *
   * @example
   * const block = await aptos.getBlockByVersion({ledgerVersion:5})
   *
   * @param args.ledgerVersion Ledger version to lookup block information for
   * @param args.options.withTransactions If set to true, include all transactions in the block
   *
   * @returns Block information with optional transactions
   */
  async getBlockByVersion(args: {
    ledgerVersion: AnyNumber;
    options?: { withTransactions?: boolean };
  }): Promise<Block> {
    return getBlockByVersion({
      aptosConfig: this.config,
      ...args,
    });
  }

  /**
   * Get block by block height
   *
   * @example
   * const block = await aptos.getBlockByVersion({blockHeight:5})
   *
   * @param args.blockHeight Block height to lookup.  Starts at 0
   * @param args.options.withTransactions If set to true, include all transactions in the block
   *
   * @returns Block with optional transactions
   */
  async getBlockByHeight(args: { blockHeight: AnyNumber; options?: { withTransactions?: boolean } }): Promise<Block> {
    return getBlockByHeight({ aptosConfig: this.config, ...args });
  }

  /**
   * Queries for a Move view function
   * @param args.payload Payload for the view function
   * @param args.options.ledgerVersion The ledger version to query, if not provided it will get the latest version
   *
   * @example
   * const data = await aptos.view({
   *  payload: {
   *   function: "0x1::coin::balance",
   *   typeArguments: ["0x1::aptos_coin::AptosCoin"],
   *   functionArguments: [accountAddress],
   *  }
   * })
   *
   * @returns an array of Move values
   */
  async view<T extends Array<MoveValue>>(args: {
    payload: InputViewFunctionData;
    options?: LedgerVersionArg;
  }): Promise<T> {
    return view<T>({ aptosConfig: this.config, ...args });
  }

  /**
   * Queries top user transactions
   *
   * @example
   * const topUserTransactions = await aptos.getChainTopUserTransactions({limit:5})
   *
   * @param args.limit The number of transactions to return
   * @returns GetChainTopUserTransactionsResponse
   */
  async getChainTopUserTransactions(args: { limit: number }): Promise<GetChainTopUserTransactionsResponse> {
    return getChainTopUserTransactions({
      aptosConfig: this.config,
      ...args,
    });
  }

  /**
   * A generic function for retrieving data from Aptos Indexer.
   * For more detailed queries specification see
   * {@link https://cloud.hasura.io/public/graphiql?endpoint=https://api.mainnet.aptoslabs.com/v1/graphql}
   *
   * @example
   * const topUserTransactions = await aptos.queryIndexer({
   *  query: `query MyQuery {
   *   ledger_infos {
   *     chain_id
   *   }}`;
   * })
   *
   * @param args.query.query A GraphQL query
   * @param args.query.variables The variables for the query
   *
   * @return The provided T type
   */
  async queryIndexer<T extends {}>(args: { query: GraphqlQuery }): Promise<T> {
    return queryIndexer<T>({
      aptosConfig: this.config,
      ...args,
    });
  }

  /**
   * Queries for the last successful indexer version
   *
   * This is useful to tell what ledger version the indexer is updated to, as it can be behind the full nodes.
   *
   * @example
   * const version = await aptos.getIndexerLastSuccessVersion()
   */
  async getIndexerLastSuccessVersion(): Promise<bigint> {
    return getIndexerLastSuccessVersion({ aptosConfig: this.config });
  }

  /**
   * Query the processor status for a specific processor type.
   *
   * @example
   * const status = await aptos.getProcessorStatus({processorType:"account_transactions_processor"})
   *
   * @param processorType The processor type to query
   * @returns
   */
  async getProcessorStatus(processorType: ProcessorType): Promise<GetProcessorStatusResponse[0]> {
    return getProcessorStatus({ aptosConfig: this.config, processorType });
  }
}
