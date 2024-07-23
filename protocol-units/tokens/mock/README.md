# Mock Tokens


## Introduction
<!-- Provide an introduction to mock tokens and their purpose in the movement networks -->

## Testnets

### Suzuka (APTOS)

#### Faucet Tokens

Faucet Address: `0x275f508689de8756169d1ee02d889c777de1cebda3a7bbcce63ba8a27c563c6f`

The following tokens can be minted through the faucet once per hour by calling `mint` and using the provided coin types as parameter:

- USDC: `0x275f508689de8756169d1ee02d889c777de1cebda3a7bbcce63ba8a27c563c6f::tokens::USDC`
- USDT: `0x275f508689de8756169d1ee02d889c777de1cebda3a7bbcce63ba8a27c563c6f::tokens::USDT`
- WBTC: `0x275f508689de8756169d1ee02d889c777de1cebda3a7bbcce63ba8a27c563c6f::tokens::WBTC`
- WETH: `0x275f508689de8756169d1ee02d889c777de1cebda3a7bbcce63ba8a27c563c6f::tokens::WETH`

### Imola (MEVM)

#### Mintable Tokens

The following tokens can be minted through their own contract once per hour by calling the mint function:

- USDC: `0xdfd318a689EF63833C4e9ab6Da17F0d5e3010013`
- USDT: `0x3150DC83cc9985f2433E546e725C9B5E6feb2E8c`
- WBTC: `0x8507bC108d0e8b8bd404d04084692B118B4F8332`
- WETH: `0x56c035c3f0e8e11fA34F79aaEf6a28A4cc8e31a8`

#### Wrapped Tokens

The following tokens cam be minted by depositing the network native asset (MOVE) to it:

### Imola (SUI)

#### Mintable Tokens

Package ID: `0x11ae349b278ee9c775483b4d61a8b2d0ac54a8e3eb7aba0fce57ac501f6bc738`

The following tokens can be minted through their own module once per hour by calling the mint function or mint_and_transfer:

- WBTC: `0x11ae349b278ee9c775483b4d61a8b2d0ac54a8e3eb7aba0fce57ac501f6bc738::wbtc::WBTC`
  - Treasury Cap Object ID: `0x091c640d0b1a3b2ed4a6142a9e9b3c1aa9ecd4d96f9dff44ec21731e6a22464c`
- WETH: `0x11ae349b278ee9c775483b4d61a8b2d0ac54a8e3eb7aba0fce57ac501f6bc738::weth::WETH`
  - Treasury Cap Object ID: `0x6ead36c02cabf5de036725b698f5210c75b6880711ded921355d92330ad6cd03`
- USDC: `0x11ae349b278ee9c775483b4d61a8b2d0ac54a8e3eb7aba0fce57ac501f6bc738::usdc::USDC`
  - Treasury Cap Object ID: `0x4bf99b8530de038b3a32f40d012f82846ce47a5d50124a4a99deea4dca0cc17e`
- USDT: `0x11ae349b278ee9c775483b4d61a8b2d0ac54a8e3eb7aba0fce57ac501f6bc738::usdt::USDT`
  - Treasury Cap Object ID: `0x196d59ed9a105100c1b5f8be7778512f062446b5441c8f09b645e60418f58a7e`

## Devnets

### M1 (MEVM)

#### Mintable Tokens

The following tokens can be minted through their own contract once per hour by calling the mint function:

- USDC: `0xdfd318a689EF63833C4e9ab6Da17F0d5e3010013`
- USDT: `0x3150DC83cc9985f2433E546e725C9B5E6feb2E8c`
- WBTC: `0x8507bC108d0e8b8bd404d04084692B118B4F8332`
- WETH: `0x56c035c3f0e8e11fA34F79aaEf6a28A4cc8e31a8`

#### Wrapped Tokens

The following tokens cam be minted by depositing the network native asset (MOVE) to it:

- WMOVE: `0xBcD2b1D0263b7735138fBCAd05Df7f08dD5F73DA`

### M2 (SUI)

#### Mintable Tokens

Package ID: `0x457abead7283c8af79b0902e71decf173f88624fe8dd2e76be97b6132c39e9c9`

The following tokens can be minted through their own module once per hour by calling the mint function or mint_and_transfer:

- BTC: `0x457abead7283c8af79b0902e71decf173f88624fe8dd2e76be97b6132c39e9c9::wbtc::WBTC`
- ETH: `0x457abead7283c8af79b0902e71decf173f88624fe8dd2e76be97b6132c39e9c9::weth::WETH`
- USDC: `0x457abead7283c8af79b0902e71decf173f88624fe8dd2e76be97b6132c39e9c9::usdc::USDC`
- USDT: `0x457abead7283c8af79b0902e71decf173f88624fe8dd2e76be97b6132c39e9c9::usdt::USDT`
