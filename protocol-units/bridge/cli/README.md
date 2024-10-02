# Bridge CLI

This program is a CLI tool designed to interact with a cross-chain bridge service. The tool provides the ability to initiate, complete, refund, or abort transfers between Ethereum and Movement chains. The bridge facilitates secure transfers using hash locks and ensures the safe exchange of assets across chains.

## Installation

To build the project, run:

```bash
cargo build
``````

## 1. Initiate a transfer from Ethereum to Movement
./target/debug/bridge-cli eth-to-movement initiate "0x1234567890abcdef" 100 "2bb80d537b1da3e38bd30361aa855686bde0b2f16f48e5b536b0f7625a529f33"

## 2. Complete a transfer from Ethereum to Movement
./target/debug/bridge-cli eth-to-movement complete --transfer-id "deadbeef12345678" --preimage "abcdef1234567890"

## 3. Get transfer details from Ethereum to Movement
./target/debug/bridge-cli eth-to-movement details --transfer-id "deadbeef12345678"

## 4. Refund a transfer on Ethereum to Movement
./target/debug/bridge-cli eth-to-movement initiator-refund --transfer-id "deadbeef12345678"

## 5. Abort a transfer on Ethereum to Movement by the counterparty
./target/debug/bridge-cli eth-to-movement counterparty-abort --transfer-id "deadbeef12345678"

## 6. Initiate a transfer from Movement to Ethereum
./target/debug/bridge-cli movement-to-eth initiate --recipient "0xabcdef1234567890" --amount 200 --hash-lock "1234567890abcdef"

## 7. Complete a transfer from Movement to Ethereum
./target/debug/bridge-cli movement-to-eth complete --transfer-id "feedcafe87654321" --preimage "1234567890abcdef"

## 8. Refund a transfer on Movement to Ethereum
./target/debug/bridge-cli movement-to-eth initiator-refund --transfer-id "feedcafe87654321"

## 9. Abort a transfer on Movement to Ethereum by the counterparty
./target/debug/bridge-cli movement-to-eth counterparty-abort --transfer-id "feedcafe87654321"

## 10. Get transfer details from Movement to Ethereum
./target/debug/bridge-cli movement-to-eth details --transfer-id "feedcafe87654321"

