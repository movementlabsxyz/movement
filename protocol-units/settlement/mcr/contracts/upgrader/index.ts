import Safe from "@safe-global/protocol-kit";
//@ts-ignore
import SafeApiKit from '@safe-global/api-kit';
import {
  OperationType,
  SafeTransactionDataPartial,
} from "@safe-global/safe-core-sdk-types";
import * as fs from "fs";
import { Command } from "commander";
// import dotenv and load from the .env file in the parent directory
import dotenv from "dotenv";
dotenv.config({ path: "../.env" });

interface Config {
  CHAIN_ID: bigint;
  RPC_URL: string;
  SIGNER_ADDRESS_PRIVATE_KEY: string;
  SAFE_ADDRESS: string;
}

async function main() {
  const private_key = process.env.PRIVATE_KEY;
  if (!private_key) {
    throw new Error("PRIVATE_KEY is required");
  }

  const program = new Command();

  program.option("-c, --contract <string>", "contract name");
  program.parse(process.argv);

  const rawData = fs.readFileSync(
    `../script/helpers/upgrade/${program.opts().contract}.json`,
    "utf-8"
  );
  const jsonData = JSON.parse(rawData);

  const config: Config = {
    CHAIN_ID: jsonData.chainId as bigint,
    RPC_URL: "https://sepolia.gateway.tenderly.co",
    SIGNER_ADDRESS_PRIVATE_KEY: private_key,
    SAFE_ADDRESS: jsonData.safeAddress,
  };

  // Create Safe API Kit instance
  const apiKit = new SafeApiKit({
    chainId: config.CHAIN_ID,
  });

    
  // Create Safe instance
  const protocolKit = await Safe.init({
    provider: config.RPC_URL,
    signer: config.SIGNER_ADDRESS_PRIVATE_KEY,
    safeAddress: config.SAFE_ADDRESS,
  });


  // Create transaction
  const safeTransactionData: SafeTransactionDataPartial = {
    to: jsonData.to,
    value: jsonData.value || "0",
    data: jsonData.data,
    operation: jsonData.operation || OperationType.Call,
  };
  const safeTransaction = await protocolKit.createTransaction({
    transactions: [safeTransactionData],
  });

  const signerAddress =
    (await protocolKit.getSafeProvider().getSignerAddress()) || "0x";
  const safeTxHash = await protocolKit.getTransactionHash(safeTransaction);
  const signature = await protocolKit.signHash(safeTxHash);

  // Propose transaction to the service
  await apiKit.proposeTransaction({
    safeAddress: config.SAFE_ADDRESS,
    safeTransactionData: safeTransaction.data,
    safeTxHash,
    senderAddress: signerAddress,
    senderSignature: signature.data,
  });

  console.log("Proposed a transaction with Safe:", config.SAFE_ADDRESS);
  console.log("- safeTxHash:", safeTxHash);
  console.log("- Sender:", signerAddress);
  console.log("- Sender signature:", signature.data);
}

main();
