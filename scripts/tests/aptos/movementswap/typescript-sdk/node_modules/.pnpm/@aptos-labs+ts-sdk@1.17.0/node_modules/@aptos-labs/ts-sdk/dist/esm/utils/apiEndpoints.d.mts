declare const NetworkToIndexerAPI: Record<string, string>;
declare const NetworkToNodeAPI: Record<string, string>;
declare const NetworkToFaucetAPI: Record<string, string>;
declare enum Network {
    MAINNET = "mainnet",
    TESTNET = "testnet",
    DEVNET = "devnet",
    LOCAL = "local",
    CUSTOM = "custom"
}
declare const NetworkToChainId: Record<string, number>;
declare const NetworkToNetworkName: Record<string, Network>;

export { Network, NetworkToChainId, NetworkToFaucetAPI, NetworkToIndexerAPI, NetworkToNetworkName, NetworkToNodeAPI };
