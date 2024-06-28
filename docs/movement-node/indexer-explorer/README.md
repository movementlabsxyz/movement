# Indexer and Explorer Onboarding

## Indexer Infrastructure

Movement Network’s RPC provides a stable API. However, for those seeking efficient querying of on-chain states for applications, Movement Labs provides an indexing service for Movement Network. 

Endpoints: (forthcoming; Indexer service is being deployed currently)

The Movement Network Indexer API is based on the [Aptos Indexer API](https://aptos.dev/en/build/indexer) and will support all its features including GraphQL queries.

## Explorer Reference Implementation

Repository: https://github.com/movementlabsxyz/explorer/tree/suzuka

This implementation is a modified version of the Aptos explorer. 

`AptosClient` and `Types` from the [Aptos TypeScript SDK](https://github.com/aptos-labs/aptos-ts-sdk) provide methods and types for interacting with Movement Network Testnet.