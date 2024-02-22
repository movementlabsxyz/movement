# Canonical VM
This doc serves as a general developer onboarding and point of reference for Canonical VM development. It is intended to be a living document that is updated as the project evolves. More detailed documentation can be found in the subfolders associated with various components of the project.

## Overview
The Canonical VM is a virtual machine and storage layer solution to the Move VM interoperability. It commits to the idea that modifying the various storage resolvers and natives associated with the different Move VMs can allow them to operate over the same derived state without having to completely reproduce the VMs individually.

Currently, the greatest challenge facing the Canonical VM project is the difficulty presented by Sui's object runtime which is closely entangled with its unique for of consensus. This is discussed at length elsewhere, and while working on this project you will become closely acquainted with these challenges. The bottom line is: Sui doesn't have traditional blocks and there's a lot you have to do to make Sui-like execution work outside of the Sui network.

## Getting Started
Here are resources that will help you to get started with the project. Some of them are traditional reading material, other videos, others are particularly important docs and code snippets.

### Recommended Reading and Viewing
Before diving into the various crates in this workspace, we recommend you spend considerable time considering the following resources:

#### Sui Transactions
- READ: https://docs.sui.io/concepts/transactions
- READ: https://docs.sui.io/guides/developer/sui-101/shared-owned
- READ and TRY: https://docs.sui.io/guides/developer/sui-101/building-ptb
- READ: https://medium.com/@0xZorz/fundamentals-of-parallelising-smart-contract-execution-8e75694697c3
- READ: https://academy.glassnode.com/concepts/utxo

Sui has a very unique approach to transactions. It is important to understand how they work and how they interface with objects.

**💡 Key Concepts**
- Sui objects--particularly shared objects--[are used to determine ownership and which transactions can run in parallel](https://docs.sui.io/concepts/transactions#transactions-flow---example). 
- [Programmable Transaction Blocks](https://docs.sui.io/concepts/transactions/prog-txn-blocks) are a collection of transactions usually submitted by a single user. They are grouped together for performing large operations quickly and smashing gas.
    - PTBs also help counteract a problem in some UTXO models that prevents mutating owned state multiple times in the same block.
- When a transaction [owns all of its objects](https://docs.sui.io/concepts/transactions/transaction-lifecycle), it is actually not involved in being ordered in a consensus protocol. It is simply executed and all of the cryptographic checks rely on the underlying uniqueness of the object ID, various validity checks, and the signer and validator signatures.
    - In the Canonical VM, we are not initially attempting to implement this fast path. We are sticking with setting up our various subsystems to handle block-based consensus. 
- Sui combines a UTXO and account-based approach to blockchain accounting. Usually, you'll hear people say that it's the owned transactions which are essentially UTXO and the shared transactions which are essentially account-based. For our purposes, we will only be really interested in this distinction when comes to attempting to provide user conveniences on for dealing with Aptos-accounts and Sui Objects. Initially, however, the development plan is to keep these methods of accounting largely separate.

#### Narwhal Mempool and Bullshark Consensus
- WATCH: https://www.youtube.com/watch?v=xKDDuPrYUag
- READ: https://docs.sui.io/concepts/sui-architecture/consensus

While we are not implementing Narwhal Consensus for any of the layers involved in the current Canonical VM push, having some sense of it often helps to understand the decisions made in the Sui source. 

**💡 Key Concepts**
- Narwhal is a high-throughput DAG-based mempool and Bullshark Consensus capitalizes on this with DAG-based fork-selection scheme. 
- Transactions are tracked through this mempool based on their input objects. 

#### The Sui Source
- INSPECT: https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/src/latest.rs#L79
- INSPECT: https://github.com/MystenLabs/sui/blob/dd4514d5d7ebffadcd25ac6e0cbf9a63e375dcf7/crates/sui-core/src/authority.rs#L1339
- INSPECT: https://github.com/MystenLabs/sui/blob/dd4514d5d7ebffadcd25ac6e0cbf9a63e375dcf7/crates/sui-core/src/authority.rs#L950

[`execute_transaction_to_effects`](https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/sui-execution/src/latest.rs#L79) is the highest level of execution where don't actually have to get entangled with Sui's consensus and transactions lifecycle. However, we want to draw heavily from the approaches put forth in the path between [`try_execute_immediately`](https://github.com/MystenLabs/sui/blob/dd4514d5d7ebffadcd25ac6e0cbf9a63e375dcf7/crates/sui-core/src/authority.rs#L1339) and [`prepare_certificate`](https://github.com/MystenLabs/sui/blob/dd4514d5d7ebffadcd25ac6e0cbf9a63e375dcf7/crates/sui-core/src/authority.rs#L950).

**💡 Key Concepts**
- There is a path that ostensibly allows us to do and/or mimic the object runtime without having to get entangled with consensus and the transaction lifecycle too deeply. It requires, however, implementing a lot dependencies that currently are provided from that domain.
- If the above turns out not to be the case, we essentially have to start looking going a bit lower to replace `execute_transaction_to_effects` and building many of the same dependencies that are currently scoped.
- Maybe we're wrong? Do you see something new?

### Our Source
READ: https://github.com/movemntdev/M1/tree/dev/movement-sdk/execution/sui-block-executor
INTERNAL READ: https://github.com/movemntdev/org/blob/main/projects/Canonical.md

Both of the above document progressive learning through the project. Please absorb and critique the findings.

**💡 Key Concepts**
- We began by trying to do what we are now attempting, i.e., to use `execute_transaction_to_effects`.
- We were dissuaded from the above approach by Mysten Labs when we thought that the `CheckpointExecutor` might be a better analog.
- We were dissuaded from the `CheckpointExecutor` approach when we realized that it would be more difficult to make the necessary modifications.
- We are now back to attempting to use `execute_transaction_to_effects` and are trying to build the dependencies that we need to do so.
- **!!![The Dependencies of `execute_transaction_to_effects`](https://github.com/movemntdev/M1/tree/dev/movement-sdk/execution/sui-block-executor#the-dependencies-of-execute_transaction_to_effects) is perhaps the most useful section once you have a sense o things. It maps out the dependencies we need to implement. !!!**