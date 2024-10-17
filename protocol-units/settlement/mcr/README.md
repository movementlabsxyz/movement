# MCR - Multi-Commit Rollup

**MCR** (Multi-Commit Rollup) implements a staking-based settlement contract where validators commit to specific blocks for each epoch. The contract validates blocks based on a two-thirds supermajority stake, and rewards or slashes validators based on their actions.

Validators stake tokens in each epoch to participate in block validation. The contract on Layer 1 ensures that only the earliest block commitment with a supermajority of validators is accepted.

The protocol tracks block commitments, epochs, and stake. 



## Features

This module provides key functionalities like:
- Staking and Unstaking tokens
- Block commitment validation
- Epoch management
- Validator and custodian management
- Slashing of malicious or inactive validators

## Architecture

- **Contract Types**: Includes settlement contracts for block commitments, staking contracts for validator management, token contracts for custody, and slashing contracts for penalizing misbehavior.
- **Manager**: Manages block commitments by batching and submitting them, interacts with clients, and processes commitment events (acceptance or rejection) for the settlement system.
- **Setup**: Prepares local environments or deploys contracts, manages configuration for local and deployment setups, and ensures contract deployment when needed.
- **Runner**: Orchestrates the setup and execution of configuration tasks, applies setup steps, and logs processes for debugging.
- **Client**: Handles interaction with the MCR system by posting block commitments, streaming commitment data, and managing Ethereum blockchain interactions.


## Testing

The contracts include extensive tests to verify their correctness. You can run tests using Foundry:

```
forge test
```