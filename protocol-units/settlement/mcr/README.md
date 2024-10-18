# MCR - Multi-Commit Rollup

Validators stake tokens in each epoch to participate in block validation. 

**MCR** (Multi-Commit Rollup) implements a staking-based settlement where validators commit L2-blocks on Layer 1 (L1). 

The contract on L1 tracks block commitments, epochs, and stake. The contract validates if commitments have reached two-thirds supermajority stake, and rewards or slashes validators based on their actions. 

For further details see the [RFC for MCR](https://github.com/movementlabsxyz/rfcs/pulls).


## Features

This module provides key functionalities like:
- Staking and Unstaking tokens
- Block commitment validation
- Epoch management
- Validator and custodian management
- Rewarding validators
- Slashing of malicious validators

## Architecture

- [Contracts](./contracts/README.md): Includes settlement contracts for block commitments, staking contracts for validator management, token contracts for custody, and slashing contracts for penalizing misbehavior.
- **Manager**: Manages block commitments by batching and submitting them, interacts with clients, and processes commitment events (acceptance or rejection) for the settlement system.
- **Setup**: Prepares local environments or deploys contracts, manages configuration for local and deployment setups, and ensures contract deployment when needed.
- **Runner**: Orchestrates the setup and execution of configuration tasks, applies setup steps, and logs processes for debugging.
- **Client**: Handles interaction with the MCR system by posting block commitments, streaming commitment data, and managing Ethereum blockchain interactions.