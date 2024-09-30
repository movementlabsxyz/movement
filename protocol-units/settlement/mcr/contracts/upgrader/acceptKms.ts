//@ts-ignore
import SafeApiKit from "@safe-global/api-kit";
import { KMSClient, SignCommand, SignCommandInput } from "@aws-sdk/client-kms";

import * as fs from "fs";
import { Command } from "commander";
import dotenv from "dotenv";
dotenv.config({ path: "../.env" });

async function main() {
  const program = new Command();

  program
    .option("-c, --contract <string>", "contract name")
    .option("-k, --key <string>", "key id")
    .option("-h, --hash <string>", "hash");
  program.parse(process.argv);

  if (!program.opts().key && !program.opts().hash) {
    throw new Error("PRIVATE_KEY or AWS Key Id is required");
  }

  const rawData = fs.readFileSync(
    `../script/helpers/upgrade/${program.opts().contract}.json`,
    "utf-8"
  );
  const jsonData = JSON.parse(rawData);

  // Create Safe API Kit instance
  const apiKit = new SafeApiKit({
    chainId: jsonData.chainId as bigint,
  });

  const signature = await signData(program.opts().hash, program.opts().keyId);

  // Get transaction from the service
  const transaction = await apiKit.confirmTransaction(program.opts().hash, signature);

  console.log("Transaction confirmed:", transaction);
  console.log("Accepted a transaction with Safe:", jsonData.safeAddress);
  console.log("- safeTxHash:", program.opts().hash);
  console.log("- Sender signature:", signature);
  return signature;
}

async function signData(data: string, keyId: string): Promise<string> {
  const client = new KMSClient({ region: "us-east-1" });

  const dataBuffer = Buffer.from(data, "utf-8");
  const input: SignCommandInput = {
    KeyId: keyId,
    Message: dataBuffer,
    MessageType: "DIGEST",
    SigningAlgorithm: "ECDSA_SHA_256",
  };

  try {
    const command = new SignCommand(input);
    const response = await client.send(command);
    if (!response.Signature) {
      throw new Error("No signature returned");
    }
    const signature = response.Signature.toString();
    return signature;
  } catch (error) {
    console.error("Error signing data:", error);
    throw new Error("Error signing data");
  }
}

main();
