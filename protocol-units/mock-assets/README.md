# Mock Tokens


## Introduction
<!-- Provide an introduction to mock tokens and their purpose in the movement networks -->

## Testnets

### Suzuka (aptos)

#### Faucet Tokens

The following tokens can be minted through the faucet once per hour and using the provided coin types as parameter:

- USDC: `{address}::tokens::USDC::mint`
- USDT: `{address}::tokens::USDT::mint`
- WBTC: `{address}::tokens::WBTC::mint`
- WETH: `{address}::tokens::WETH::mint`

To mint a specific token, replace `{address}` with the desired address and with the corresponding token name.

#### Bridge Tokens

The following tokens can only be minted by using the Bridge Service by sending ETH from Holesky:

- MOVETH

## Devnets

### M1 (mevm)

#### Mintable Tokens

The following tokens can be minted through their own contract once per hour by calling the mint function:

- USDC
- USDT
- WBTC
- WETH
- MOVETH

#### Wrapped Tokens

The following tokens cam be minted by depositing the network native asset (MOVE) to it:

- WMOVE

### M2 (sui)

#### Mintable Tokens

The following tokens can be minted through their own module once per hour by calling the mint function or mint_and_transfer:

- USDC
- USDT
- WBTC
- WETH
- MOVETH