# Bridge CLI

This program is a CLI tool designed to interact with a cross-chain bridge service. The tool provides the ability to initiate, complete, refund, or abort transfers between Ethereum and Movement chains. The bridge facilitates secure transfers using hash locks and ensures the safe exchange of assets across chains.

## Installation

To build the project, run:

```bash
cargo build
``````

## 1. Initiate a transfer from Ethereum to Movement

`./target/debug/bridge-cli eth-to-movement initiate <RECIPIENT_ADDRESS> <AMOUNT> <HASH_LOCK>`

### Example:### Example:### Example:
`./target/debug/bridge-cli eth-to-movement initiate "0x8bcdbe40eeb01c7451f359318e5709c16ab2f23c3a9fa71531cca57920aa828c" 100 "2bb80d537b1da3e38bd30361aa855686bde0b2f16f48e5b536b0f7625a529f33"`

## 2. Complete a transfer from Ethereum to Movement
`./target/debug/bridge-cli eth-to-movement complete <TRANSFER_ID> <PREIMAGE>`

## 3. Get transfer details from Ethereum to Movement
`./target/debug/bridge-cli eth-to-movement details <TRANSFER_ID>`

## 4. Refund a transfer on Ethereum to Movement, only callable by the owner
`./target/debug/bridge-cli eth-to-movement initiator-refund <TRANSFER_ID>`

## 5. Abort a transfer on Ethereum to Movement by the counterparty, only callable by the owner
`./target/debug/bridge-cli eth-to-movement counterparty-abort <TRANSFER_ID>`

## 6. Initiate a transfer from Movement to Ethereum
`./target/debug/bridge-cli movement-to-eth initiate <RECIPIENT_ADDRESS> <AMOUNT> <HASH_LOCK>`

## 7. Complete a transfer from Movement to Ethereum
`./target/debug/bridge-cli movement-to-eth initiate <RECIPIENT_ADDRESS> <AMOUNT> <HASH_LOCK>`

## 8. Refund a transfer on Movement to Ethereum, only callable by the owner
`./target/debug/bridge-cli movement-to-eth initiator-refund <TRANSFER_ID>`

## 9. Abort a transfer on Movement to Ethereum by the counterparty, only callable by the owner
`./target/debug/bridge-cli movement-to-eth counterparty-abort <TRANSFER_ID>`  

## 10. Get transfer details from Movement to Ethereum 
`./target/debug/bridge-cli movement-to-eth details <TRANSFER_ID>`

