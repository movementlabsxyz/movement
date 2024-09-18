# `protocol-units`
We identify the following protocol unit categories:
- [Data Availability](./da/m1/README.md): Protocol units concerned with enabling the secure submission and ordered retrieval of transaction data to and from a network. Light node clients and servers are members of this category.
- [Mempool](./mempool/README.md): Protocol units concerned with the acceptance and ordering of transactions in a network prior to consensus. Mempool modules are members of this category.
- [Sequencing](./sequencing/README.md): Protocol units concerned with consensus on the order of transactions in a network. Sequencer node implementations are members of this category.
- [Bridge](./bridge): Protocol units concerned with cross-blockchain bridging using atomic swaps. The atomic bridge consists of several packages and utilities to bridge from Ethereum to Movement, which can be extended to support any blockchains.
- [Cryptography](./cryptography): Protocol units concerned with cryptographic operations. Cryptography and data structure-related utilities are members of this category.
- [Execution](./execution): Protocol units concerned with execution. Block executors and related unities are members of this category.
- [Movement REST service](./movement-rest): Protocol units to support Movement's REST API. `movement-rest` provides additional Movement REST API endpoints. 
- [Settlement](./settlement): Protocol units concerned with settlement. Movement's multi-commitment rollup and related settlement utilities are members of this category. 
- [Storage](./storage): Protocol units concerned with storage. `jelly-move`, `move-access-log`, and `mpt-move` are members of this category.
