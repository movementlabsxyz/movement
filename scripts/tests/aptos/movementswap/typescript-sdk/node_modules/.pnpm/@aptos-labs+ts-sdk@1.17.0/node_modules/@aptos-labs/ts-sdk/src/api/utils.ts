import { waitForIndexer } from "../internal/transaction";
import { ProcessorType } from "../utils";
import { AptosConfig } from "./aptosConfig";
import { AnyNumber } from "../types";

/**
 * Utility function to handle optional waiting on indexer for APIs
 *
 * This is purposely placed here to not expose this internal function.
 * @param args
 */
export async function waitForIndexerOnVersion(args: {
  config: AptosConfig;
  minimumLedgerVersion?: AnyNumber;
  processorType: ProcessorType;
}) {
  if (args.minimumLedgerVersion !== undefined) {
    await waitForIndexer({
      aptosConfig: args.config,
      minimumLedgerVersion: args.minimumLedgerVersion,
      processorType: args.processorType,
    });
  }
}
