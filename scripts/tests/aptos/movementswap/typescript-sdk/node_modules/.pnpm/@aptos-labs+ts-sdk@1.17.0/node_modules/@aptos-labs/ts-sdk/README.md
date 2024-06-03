# Typescript SDK for Aptos

![License][github-license]
[![Discord][discord-image]][discord-url]
[![NPM Package Downloads][npm-image-downloads]][npm-url]

### Reference Docs
 1. For SDK documentation, check out the [TypeScript SDK documentation](https://aptos.dev/sdks/new-ts-sdk/)
 2. For detailed reference documentation you can search, check out the [API reference documentation](https://aptos-labs.github.io/aptos-ts-sdk/) for the associated version.
 3. For in-depth examples, check out the [examples](./examples) folder with ready-made `package.json` files to get you going quickly!

### Latest Version

[![NPM Package Version][npm-image-version]][npm-url]
![Node Version](https://img.shields.io/node/v/%40aptos-labs%2Fts-sdk)
![NPM bundle size](https://img.shields.io/bundlephobia/min/%40aptos-labs/ts-sdk)

### Experimental Development Version

[![NPM Experimental Version](https://img.shields.io/npm/v/%40aptos-labs/ts-sdk/experimental)][experimental-url]
![Experimental Node Version](https://img.shields.io/node/v/%40aptos-labs%2Fts-sdk/experimental)
![Experimental bundle size](https://img.shields.io/bundlephobia/min/%40aptos-labs/ts-sdk/experimental)

The Aptos TypeScript SDK provides a convenient way to interact with the Aptos blockchain using TypeScript. It offers a
set of utility functions, classes, and types to simplify the integration process and enhance developer productivity.

This repository supports version >= 0.0.0 of the [Aptos SDK npm package](https://www.npmjs.com/package/@aptos-labs/ts-sdk).

## Installation

##### For use in Node.js or a web application

Install with your favorite package manager such as npm, yarn, or pnpm:

```bash
pnpm install @aptos-labs/ts-sdk
```

##### For use in a browser (<= 1.9.1 version only)

You can add the SDK to your web application using a script tag:

```html
<script src="https://unpkg.com/@aptos-labs/ts-sdk/dist/browser/index.global.js" />
```

Then, the SDK can be accessed through `window.aptosSDK`.

## Usage

Initialize `Aptos` to access the SDK API.

```ts
// initiate the main entry point into Aptos SDK
const aptos = new Aptos();
```

If you want to pass in a custom config

```ts
// an optional config information for the SDK client instance.
const config = new AptosConfig({ network: Network.LOCAL }); // default network is devnet
const aptos = new Aptos(config);
```

### Read data from chain

---

```ts
const modules = await aptos.getAccountModules({ accountAddress: "0x123" });
```

### Account management (default to Ed25519)

> Note: We introduce a Single Sender authentication (as introduced in [AIP-55](https://github.com/aptos-foundation/AIPs/pull/263)). Generating an account defaults to Legacy Ed25519 authentication with the option to use the Single Sender unified authentication

---

#### Generate new keys

```ts
const account = Account.generate(); // defaults to Legacy Ed25519
const account = Account.generate({ scheme: SigningSchemeInput.Secp256k1Ecdsa }); // Single Sender Secp256k1
const account = Account.generate({ scheme: SigningSchemeInput.Ed25519, legacy: false }); // Single Sender Ed25519
```

#### Derive from private key

```ts
// Create a private key instance for Ed25519 scheme
const privateKey = new Ed25519PrivateKey("myEd25519privatekeystring");
// Or for Secp256k1 scheme
const privateKey = new Secp256k1PrivateKey("mySecp256k1privatekeystring");

// Derive an account from private key

// This is used as a local calculation and therefore is used to instantiate an `Account`
// that has not had its authentication key rotated
const account = await Account.fromPrivateKey({ privateKey });

// Also, can use this function that resolves the provided private key type and derives the public key from it
// to support key rotation and differentiation between Legacy Ed25519 and Unified authentications
// Read more https://github.com/aptos-labs/aptos-ts-sdk/blob/main/src/api/account.ts#L364
const aptos = new Aptos();
const account = await aptos.deriveAccountFromPrivateKey({ privateKey });
```

#### Derive from private key and address

```ts
// Create a private key instance for Ed25519 scheme
const privateKey = new Ed25519PrivateKey("myEd25519privatekeystring");
// Or for Secp256k1 scheme
const privateKey = new Secp256k1PrivateKey("mySecp256k1privatekeystring");

// Derive an account from private key and address

// create an AccountAddress instance from the account address string
const address = AccountAddress.from("myaccountaddressstring");
// Derieve an account from private key and address
const account = await Account.fromPrivateKeyAndAddress({ privateKey, address });
```

#### Derive from path

```ts
const path = "m/44'/637'/0'/0'/1";
const mnemonic = "various float stumble...";
const account = Account.fromDerivationPath({ path, mnemonic });
```

### Submit transaction

---

#### Single Signer transaction

Using transaction submission api

```ts
const alice: Account = Account.generate();
const bobAddress = "0xb0b";
// build transaction
const transaction = await aptos.transaction.build.simple({
  sender: alice.accountAddress,
  data: {
    function: "0x1::coin::transfer",
    typeArguments: ["0x1::aptos_coin::AptosCoin"],
    functionArguments: [bobAddress, 100],
  },
});

// using sign and submit separately
const senderAuthenticator = aptos.transaction.sign({ signer: alice, transaction });
const committedTransaction = await aptos.transaction.submit.simple({ transaction, senderAuthenticator });

// using signAndSubmit combined
const committedTransaction = await aptos.signAndSubmitTransaction({ signer: alice, transaction });
```

Using built in `transferCoinTransaction`

```ts
const alice: Account = Account.generate();
const bobAddress = "0xb0b";
// build transaction
const transaction = await aptos.transferCoinTransaction({
  sender: alice,
  recipient: bobAddress,
  amount: 100,
});

const pendingTransaction = await aptos.signAndSubmitTransaction({ signer: alice, transaction });
```

### Testing

To run the SDK tests, simply run from the root of this repository:

> Note: for a better experience, make sure there is no aptos local node process up and running (can check if there is a process running on port 8080).

```bash
pnpm i
pnpm test
```

## Troubleshooting

If you see import error when you do this

```typescript
import { Aptos, AptosConfig, Network } from "@aptos-labs/ts-sdk";
```

It could be your `tsconfig.json` is not incompatible, make sure your `moduleResolution` is set to `node` instead of `bundler`.

## Contributing

If you found a bug or would like to request a feature, please file an [issue](https://github.com/aptos-labs/aptos-ts-sdk/issues/new/choose).
If, based on the discussion on an issue you would like to offer a code change, please make a [pull request](https://github.com/aptos-labs/aptos-ts-sdk/CONTRIBUTING.md).
If neither of these describes what you would like to contribute, checkout out the [contributing guide](https://github.com/aptos-labs/aptos-ts-sdk/CONTRIBUTING.md).

[npm-image-version]: https://img.shields.io/npm/v/%40aptos-labs%2Fts-sdk.svg
[npm-image-downloads]: https://img.shields.io/npm/dm/%40aptos-labs%2Fts-sdk.svg
[npm-url]: https://npmjs.org/package/@aptos-labs/ts-sdk
[experimental-url]: https://www.npmjs.com/package/@aptos-labs/ts-sdk/v/experimental
[discord-image]: https://img.shields.io/discord/945856774056083548?label=Discord&logo=discord&style=flat~~~~
[discord-url]: https://discord.gg/aptosnetwork
[github-license]: https://img.shields.io/github/license/aptos-labs/aptos-ts-sdk
[discord-url]: https://discord.gg/aptosnetwork
