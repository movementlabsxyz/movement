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

- WMOVE: 0xBcD2b1D0263b7735138fBCAd05Df7f08dD5F73DA

### Imola (SUI)

#### Mintable Tokens

Package ID: `0x8ac626e474c33520a815175649fefcbb272678c8c37a7b024e7171fa45d47711`

The following tokens can be minted through their own module once per hour by calling the mint function or mint_and_transfer:

- WBTC: `0x8ac626e474c33520a815175649fefcbb272678c8c37a7b024e7171fa45d47711::wbtc::WBTC`
  - Treasury Cap Object ID: `0xd2c1127a16494f9df5b6f973baebd78e093d66b3c06463c4e930c8545a9b6df2`
- WETH: `0x8ac626e474c33520a815175649fefcbb272678c8c37a7b024e7171fa45d47711::weth::WETH`
  - Treasury Cap Object ID: `0xe02ba3510a9240ba970aed72e0c6188989c3e6d6bd316edfa12bd04da8ebf675`
- USDC: `0x8ac626e474c33520a815175649fefcbb272678c8c37a7b024e7171fa45d47711::usdc::USDC`
  - Treasury Cap Object ID: `0x6bad1a88caef6f9ea56680cd31315b2cfeb6018b105471320407559042e6d067`
- USDT: `0x8ac626e474c33520a815175649fefcbb272678c8c37a7b024e7171fa45d47711::usdt::USDT`
  - Treasury Cap Object ID: `0x8cacf2fd727720db5fc11006786fbcf69408decda4611921da791cc8ed844878`

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
