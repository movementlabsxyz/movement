// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "./LZAddresses.sol";
import "./interfaces/ILZProtocol.sol";

/// @title LayerZero Protocol Address Provider
/// @notice Provides LayerZero protocol addresses from the generated address book
/// @dev Implements ILZProtocol interface for clean access to core protocol addresses
contract LZProtocol is ILZProtocol {
    // Storage for chain data
    mapping(uint32 => ProtocolAddresses) private _protocolAddresses;
    
    // List of registered EIDs for iteration
    uint32[] private _registeredEids;
    
    // Mapping from chain name to EID for reverse lookup
    mapping(string => uint32) private _chainNameToEid;
    
    constructor() {
        _registerAllChains();
    }
    
    /// @notice Register all supported chains from the AddressBook
    function _registerAllChains() private {
        // Mainnets
        _registerChain(LayerZeroV2AbstractMainnet.EID, address(LayerZeroV2AbstractMainnet.ENDPOINT_V2), address(LayerZeroV2AbstractMainnet.SEND_ULN_302), address(LayerZeroV2AbstractMainnet.RECEIVE_ULN_302), LayerZeroV2AbstractMainnet.EXECUTOR, LayerZeroV2AbstractMainnet.CHAIN_ID, "abstract-mainnet");
        _registerChain(LayerZeroV2AnimechainMainnet.EID, address(LayerZeroV2AnimechainMainnet.ENDPOINT_V2), address(LayerZeroV2AnimechainMainnet.SEND_ULN_302), address(LayerZeroV2AnimechainMainnet.RECEIVE_ULN_302), LayerZeroV2AnimechainMainnet.EXECUTOR, LayerZeroV2AnimechainMainnet.CHAIN_ID, "animechain-mainnet");
        _registerChain(LayerZeroV2ApeMainnet.EID, address(LayerZeroV2ApeMainnet.ENDPOINT_V2), address(LayerZeroV2ApeMainnet.SEND_ULN_302), address(LayerZeroV2ApeMainnet.RECEIVE_ULN_302), LayerZeroV2ApeMainnet.EXECUTOR, LayerZeroV2ApeMainnet.CHAIN_ID, "ape-mainnet");
        _registerChain(LayerZeroV2ApexfusionnexusMainnet.EID, address(LayerZeroV2ApexfusionnexusMainnet.ENDPOINT_V2), address(LayerZeroV2ApexfusionnexusMainnet.SEND_ULN_302), address(LayerZeroV2ApexfusionnexusMainnet.RECEIVE_ULN_302), LayerZeroV2ApexfusionnexusMainnet.EXECUTOR, LayerZeroV2ApexfusionnexusMainnet.CHAIN_ID, "apexfusionnexus-mainnet");
        _registerChain(LayerZeroV2ArbitrumMainnet.EID, address(LayerZeroV2ArbitrumMainnet.ENDPOINT_V2), address(LayerZeroV2ArbitrumMainnet.SEND_ULN_302), address(LayerZeroV2ArbitrumMainnet.RECEIVE_ULN_302), LayerZeroV2ArbitrumMainnet.EXECUTOR, LayerZeroV2ArbitrumMainnet.CHAIN_ID, "arbitrum-mainnet");
        _registerChain(LayerZeroV2AstarMainnet.EID, address(LayerZeroV2AstarMainnet.ENDPOINT_V2), address(LayerZeroV2AstarMainnet.SEND_ULN_302), address(LayerZeroV2AstarMainnet.RECEIVE_ULN_302), LayerZeroV2AstarMainnet.EXECUTOR, LayerZeroV2AstarMainnet.CHAIN_ID, "astar-mainnet");
        _registerChain(LayerZeroV2AuroraMainnet.EID, address(LayerZeroV2AuroraMainnet.ENDPOINT_V2), address(LayerZeroV2AuroraMainnet.SEND_ULN_302), address(LayerZeroV2AuroraMainnet.RECEIVE_ULN_302), LayerZeroV2AuroraMainnet.EXECUTOR, LayerZeroV2AuroraMainnet.CHAIN_ID, "aurora-mainnet");
        _registerChain(LayerZeroV2AvalancheMainnet.EID, address(LayerZeroV2AvalancheMainnet.ENDPOINT_V2), address(LayerZeroV2AvalancheMainnet.SEND_ULN_302), address(LayerZeroV2AvalancheMainnet.RECEIVE_ULN_302), LayerZeroV2AvalancheMainnet.EXECUTOR, LayerZeroV2AvalancheMainnet.CHAIN_ID, "avalanche-mainnet");
        _registerChain(LayerZeroV2BahamutMainnet.EID, address(LayerZeroV2BahamutMainnet.ENDPOINT_V2), address(LayerZeroV2BahamutMainnet.SEND_ULN_302), address(LayerZeroV2BahamutMainnet.RECEIVE_ULN_302), LayerZeroV2BahamutMainnet.EXECUTOR, LayerZeroV2BahamutMainnet.CHAIN_ID, "bahamut-mainnet");
        _registerChain(LayerZeroV2BaseMainnet.EID, address(LayerZeroV2BaseMainnet.ENDPOINT_V2), address(LayerZeroV2BaseMainnet.SEND_ULN_302), address(LayerZeroV2BaseMainnet.RECEIVE_ULN_302), LayerZeroV2BaseMainnet.EXECUTOR, LayerZeroV2BaseMainnet.CHAIN_ID, "base-mainnet");
        _registerChain(LayerZeroV2Bb1Mainnet.EID, address(LayerZeroV2Bb1Mainnet.ENDPOINT_V2), address(LayerZeroV2Bb1Mainnet.SEND_ULN_302), address(LayerZeroV2Bb1Mainnet.RECEIVE_ULN_302), LayerZeroV2Bb1Mainnet.EXECUTOR, LayerZeroV2Bb1Mainnet.CHAIN_ID, "bb1-mainnet");
        _registerChain(LayerZeroV2BeraMainnet.EID, address(LayerZeroV2BeraMainnet.ENDPOINT_V2), address(LayerZeroV2BeraMainnet.SEND_ULN_302), address(LayerZeroV2BeraMainnet.RECEIVE_ULN_302), LayerZeroV2BeraMainnet.EXECUTOR, LayerZeroV2BeraMainnet.CHAIN_ID, "bera-mainnet");
        _registerChain(LayerZeroV2BevmMainnet.EID, address(LayerZeroV2BevmMainnet.ENDPOINT_V2), address(LayerZeroV2BevmMainnet.SEND_ULN_302), address(LayerZeroV2BevmMainnet.RECEIVE_ULN_302), LayerZeroV2BevmMainnet.EXECUTOR, LayerZeroV2BevmMainnet.CHAIN_ID, "bevm-mainnet");
        _registerChain(LayerZeroV2BitlayerMainnet.EID, address(LayerZeroV2BitlayerMainnet.ENDPOINT_V2), address(LayerZeroV2BitlayerMainnet.SEND_ULN_302), address(LayerZeroV2BitlayerMainnet.RECEIVE_ULN_302), LayerZeroV2BitlayerMainnet.EXECUTOR, LayerZeroV2BitlayerMainnet.CHAIN_ID, "bitlayer-mainnet");
        _registerChain(LayerZeroV2BlastMainnet.EID, address(LayerZeroV2BlastMainnet.ENDPOINT_V2), address(LayerZeroV2BlastMainnet.SEND_ULN_302), address(LayerZeroV2BlastMainnet.RECEIVE_ULN_302), LayerZeroV2BlastMainnet.EXECUTOR, LayerZeroV2BlastMainnet.CHAIN_ID, "blast-mainnet");
        _registerChain(LayerZeroV2BobMainnet.EID, address(LayerZeroV2BobMainnet.ENDPOINT_V2), address(LayerZeroV2BobMainnet.SEND_ULN_302), address(LayerZeroV2BobMainnet.RECEIVE_ULN_302), LayerZeroV2BobMainnet.EXECUTOR, LayerZeroV2BobMainnet.CHAIN_ID, "bob-mainnet");
        _registerChain(LayerZeroV2BotanixMainnet.EID, address(LayerZeroV2BotanixMainnet.ENDPOINT_V2), address(LayerZeroV2BotanixMainnet.SEND_ULN_302), address(LayerZeroV2BotanixMainnet.RECEIVE_ULN_302), LayerZeroV2BotanixMainnet.EXECUTOR, LayerZeroV2BotanixMainnet.CHAIN_ID, "botanix-mainnet");
        _registerChain(LayerZeroV2BouncebitMainnet.EID, address(LayerZeroV2BouncebitMainnet.ENDPOINT_V2), address(LayerZeroV2BouncebitMainnet.SEND_ULN_302), address(LayerZeroV2BouncebitMainnet.RECEIVE_ULN_302), LayerZeroV2BouncebitMainnet.EXECUTOR, LayerZeroV2BouncebitMainnet.CHAIN_ID, "bouncebit-mainnet");
        _registerChain(LayerZeroV2BscMainnet.EID, address(LayerZeroV2BscMainnet.ENDPOINT_V2), address(LayerZeroV2BscMainnet.SEND_ULN_302), address(LayerZeroV2BscMainnet.RECEIVE_ULN_302), LayerZeroV2BscMainnet.EXECUTOR, LayerZeroV2BscMainnet.CHAIN_ID, "bsc-mainnet");
        _registerChain(LayerZeroV2CampMainnet.EID, address(LayerZeroV2CampMainnet.ENDPOINT_V2), address(LayerZeroV2CampMainnet.SEND_ULN_302), address(LayerZeroV2CampMainnet.RECEIVE_ULN_302), LayerZeroV2CampMainnet.EXECUTOR, LayerZeroV2CampMainnet.CHAIN_ID, "camp-mainnet");
        _registerChain(LayerZeroV2CantoMainnet.EID, address(LayerZeroV2CantoMainnet.ENDPOINT_V2), address(LayerZeroV2CantoMainnet.SEND_ULN_302), address(LayerZeroV2CantoMainnet.RECEIVE_ULN_302), LayerZeroV2CantoMainnet.EXECUTOR, LayerZeroV2CantoMainnet.CHAIN_ID, "canto-mainnet");
        _registerChain(LayerZeroV2CeloMainnet.EID, address(LayerZeroV2CeloMainnet.ENDPOINT_V2), address(LayerZeroV2CeloMainnet.SEND_ULN_302), address(LayerZeroV2CeloMainnet.RECEIVE_ULN_302), LayerZeroV2CeloMainnet.EXECUTOR, LayerZeroV2CeloMainnet.CHAIN_ID, "celo-mainnet");
        _registerChain(LayerZeroV2CodexMainnet.EID, address(LayerZeroV2CodexMainnet.ENDPOINT_V2), address(LayerZeroV2CodexMainnet.SEND_ULN_302), address(LayerZeroV2CodexMainnet.RECEIVE_ULN_302), LayerZeroV2CodexMainnet.EXECUTOR, LayerZeroV2CodexMainnet.CHAIN_ID, "codex-mainnet");
        _registerChain(LayerZeroV2ConcreteMainnet.EID, address(LayerZeroV2ConcreteMainnet.ENDPOINT_V2), address(LayerZeroV2ConcreteMainnet.SEND_ULN_302), address(LayerZeroV2ConcreteMainnet.RECEIVE_ULN_302), LayerZeroV2ConcreteMainnet.EXECUTOR, LayerZeroV2ConcreteMainnet.CHAIN_ID, "concrete-mainnet");
        _registerChain(LayerZeroV2ConfluxMainnet.EID, address(LayerZeroV2ConfluxMainnet.ENDPOINT_V2), address(LayerZeroV2ConfluxMainnet.SEND_ULN_302), address(LayerZeroV2ConfluxMainnet.RECEIVE_ULN_302), LayerZeroV2ConfluxMainnet.EXECUTOR, LayerZeroV2ConfluxMainnet.CHAIN_ID, "conflux-mainnet");
        _registerChain(LayerZeroV2ConvergeMainnet.EID, address(LayerZeroV2ConvergeMainnet.ENDPOINT_V2), address(LayerZeroV2ConvergeMainnet.SEND_ULN_302), address(LayerZeroV2ConvergeMainnet.RECEIVE_ULN_302), LayerZeroV2ConvergeMainnet.EXECUTOR, LayerZeroV2ConvergeMainnet.CHAIN_ID, "converge-mainnet");
        _registerChain(LayerZeroV2CoredaoMainnet.EID, address(LayerZeroV2CoredaoMainnet.ENDPOINT_V2), address(LayerZeroV2CoredaoMainnet.SEND_ULN_302), address(LayerZeroV2CoredaoMainnet.RECEIVE_ULN_302), LayerZeroV2CoredaoMainnet.EXECUTOR, LayerZeroV2CoredaoMainnet.CHAIN_ID, "coredao-mainnet");
        _registerChain(LayerZeroV2CronosevmMainnet.EID, address(LayerZeroV2CronosevmMainnet.ENDPOINT_V2), address(LayerZeroV2CronosevmMainnet.SEND_ULN_302), address(LayerZeroV2CronosevmMainnet.RECEIVE_ULN_302), LayerZeroV2CronosevmMainnet.EXECUTOR, LayerZeroV2CronosevmMainnet.CHAIN_ID, "cronosevm-mainnet");
        _registerChain(LayerZeroV2CronoszkevmMainnet.EID, address(LayerZeroV2CronoszkevmMainnet.ENDPOINT_V2), address(LayerZeroV2CronoszkevmMainnet.SEND_ULN_302), address(LayerZeroV2CronoszkevmMainnet.RECEIVE_ULN_302), LayerZeroV2CronoszkevmMainnet.EXECUTOR, LayerZeroV2CronoszkevmMainnet.CHAIN_ID, "cronoszkevm-mainnet");
        _registerChain(LayerZeroV2CyberMainnet.EID, address(LayerZeroV2CyberMainnet.ENDPOINT_V2), address(LayerZeroV2CyberMainnet.SEND_ULN_302), address(LayerZeroV2CyberMainnet.RECEIVE_ULN_302), LayerZeroV2CyberMainnet.EXECUTOR, LayerZeroV2CyberMainnet.CHAIN_ID, "cyber-mainnet");
        _registerChain(LayerZeroV2DegenMainnet.EID, address(LayerZeroV2DegenMainnet.ENDPOINT_V2), address(LayerZeroV2DegenMainnet.SEND_ULN_302), address(LayerZeroV2DegenMainnet.RECEIVE_ULN_302), LayerZeroV2DegenMainnet.EXECUTOR, LayerZeroV2DegenMainnet.CHAIN_ID, "degen-mainnet");
        _registerChain(LayerZeroV2DexalotMainnet.EID, address(LayerZeroV2DexalotMainnet.ENDPOINT_V2), address(LayerZeroV2DexalotMainnet.SEND_ULN_302), address(LayerZeroV2DexalotMainnet.RECEIVE_ULN_302), LayerZeroV2DexalotMainnet.EXECUTOR, LayerZeroV2DexalotMainnet.CHAIN_ID, "dexalot-mainnet");
        _registerChain(LayerZeroV2DfkMainnet.EID, address(LayerZeroV2DfkMainnet.ENDPOINT_V2), address(LayerZeroV2DfkMainnet.SEND_ULN_302), address(LayerZeroV2DfkMainnet.RECEIVE_ULN_302), LayerZeroV2DfkMainnet.EXECUTOR, LayerZeroV2DfkMainnet.CHAIN_ID, "dfk-mainnet");
        _registerChain(LayerZeroV2DinariMainnet.EID, address(LayerZeroV2DinariMainnet.ENDPOINT_V2), address(LayerZeroV2DinariMainnet.SEND_ULN_302), address(LayerZeroV2DinariMainnet.RECEIVE_ULN_302), LayerZeroV2DinariMainnet.EXECUTOR, LayerZeroV2DinariMainnet.CHAIN_ID, "dinari-mainnet");
        _registerChain(LayerZeroV2Dm2verseMainnet.EID, address(LayerZeroV2Dm2verseMainnet.ENDPOINT_V2), address(LayerZeroV2Dm2verseMainnet.SEND_ULN_302), address(LayerZeroV2Dm2verseMainnet.RECEIVE_ULN_302), LayerZeroV2Dm2verseMainnet.EXECUTOR, LayerZeroV2Dm2verseMainnet.CHAIN_ID, "dm2verse-mainnet");
        _registerChain(LayerZeroV2DomaMainnet.EID, address(LayerZeroV2DomaMainnet.ENDPOINT_V2), address(LayerZeroV2DomaMainnet.SEND_ULN_302), address(LayerZeroV2DomaMainnet.RECEIVE_ULN_302), LayerZeroV2DomaMainnet.EXECUTOR, LayerZeroV2DomaMainnet.CHAIN_ID, "doma-mainnet");
        _registerChain(LayerZeroV2DosMainnet.EID, address(LayerZeroV2DosMainnet.ENDPOINT_V2), address(LayerZeroV2DosMainnet.SEND_ULN_302), address(LayerZeroV2DosMainnet.RECEIVE_ULN_302), LayerZeroV2DosMainnet.EXECUTOR, LayerZeroV2DosMainnet.CHAIN_ID, "dos-mainnet");
        _registerChain(LayerZeroV2EbiMainnet.EID, address(LayerZeroV2EbiMainnet.ENDPOINT_V2), address(LayerZeroV2EbiMainnet.SEND_ULN_302), address(LayerZeroV2EbiMainnet.RECEIVE_ULN_302), LayerZeroV2EbiMainnet.EXECUTOR, LayerZeroV2EbiMainnet.CHAIN_ID, "ebi-mainnet");
        _registerChain(LayerZeroV2EduMainnet.EID, address(LayerZeroV2EduMainnet.ENDPOINT_V2), address(LayerZeroV2EduMainnet.SEND_ULN_302), address(LayerZeroV2EduMainnet.RECEIVE_ULN_302), LayerZeroV2EduMainnet.EXECUTOR, LayerZeroV2EduMainnet.CHAIN_ID, "edu-mainnet");
        _registerChain(LayerZeroV2EonMainnet.EID, address(LayerZeroV2EonMainnet.ENDPOINT_V2), address(LayerZeroV2EonMainnet.SEND_ULN_302), address(LayerZeroV2EonMainnet.RECEIVE_ULN_302), LayerZeroV2EonMainnet.EXECUTOR, LayerZeroV2EonMainnet.CHAIN_ID, "eon-mainnet");
        _registerChain(LayerZeroV2EtherealMainnet.EID, address(LayerZeroV2EtherealMainnet.ENDPOINT_V2), address(LayerZeroV2EtherealMainnet.SEND_ULN_302), address(LayerZeroV2EtherealMainnet.RECEIVE_ULN_302), LayerZeroV2EtherealMainnet.EXECUTOR, LayerZeroV2EtherealMainnet.CHAIN_ID, "ethereal-mainnet");
        _registerChain(LayerZeroV2EthereumMainnet.EID, address(LayerZeroV2EthereumMainnet.ENDPOINT_V2), address(LayerZeroV2EthereumMainnet.SEND_ULN_302), address(LayerZeroV2EthereumMainnet.RECEIVE_ULN_302), LayerZeroV2EthereumMainnet.EXECUTOR, LayerZeroV2EthereumMainnet.CHAIN_ID, "ethereum-mainnet");
        _registerChain(LayerZeroV2EtherlinkMainnet.EID, address(LayerZeroV2EtherlinkMainnet.ENDPOINT_V2), address(LayerZeroV2EtherlinkMainnet.SEND_ULN_302), address(LayerZeroV2EtherlinkMainnet.RECEIVE_ULN_302), LayerZeroV2EtherlinkMainnet.EXECUTOR, LayerZeroV2EtherlinkMainnet.CHAIN_ID, "etherlink-mainnet");
        _registerChain(LayerZeroV2FantomMainnet.EID, address(LayerZeroV2FantomMainnet.ENDPOINT_V2), address(LayerZeroV2FantomMainnet.SEND_ULN_302), address(LayerZeroV2FantomMainnet.RECEIVE_ULN_302), LayerZeroV2FantomMainnet.EXECUTOR, LayerZeroV2FantomMainnet.CHAIN_ID, "fantom-mainnet");
        _registerChain(LayerZeroV2FlareMainnet.EID, address(LayerZeroV2FlareMainnet.ENDPOINT_V2), address(LayerZeroV2FlareMainnet.SEND_ULN_302), address(LayerZeroV2FlareMainnet.RECEIVE_ULN_302), LayerZeroV2FlareMainnet.EXECUTOR, LayerZeroV2FlareMainnet.CHAIN_ID, "flare-mainnet");
        _registerChain(LayerZeroV2FlowMainnet.EID, address(LayerZeroV2FlowMainnet.ENDPOINT_V2), address(LayerZeroV2FlowMainnet.SEND_ULN_302), address(LayerZeroV2FlowMainnet.RECEIVE_ULN_302), LayerZeroV2FlowMainnet.EXECUTOR, LayerZeroV2FlowMainnet.CHAIN_ID, "flow-mainnet");
        _registerChain(LayerZeroV2FraxtalMainnet.EID, address(LayerZeroV2FraxtalMainnet.ENDPOINT_V2), address(LayerZeroV2FraxtalMainnet.SEND_ULN_302), address(LayerZeroV2FraxtalMainnet.RECEIVE_ULN_302), LayerZeroV2FraxtalMainnet.EXECUTOR, LayerZeroV2FraxtalMainnet.CHAIN_ID, "fraxtal-mainnet");
        _registerChain(LayerZeroV2FuseMainnet.EID, address(LayerZeroV2FuseMainnet.ENDPOINT_V2), address(LayerZeroV2FuseMainnet.SEND_ULN_302), address(LayerZeroV2FuseMainnet.RECEIVE_ULN_302), LayerZeroV2FuseMainnet.EXECUTOR, LayerZeroV2FuseMainnet.CHAIN_ID, "fuse-mainnet");
        _registerChain(LayerZeroV2GatelayerMainnet.EID, address(LayerZeroV2GatelayerMainnet.ENDPOINT_V2), address(LayerZeroV2GatelayerMainnet.SEND_ULN_302), address(LayerZeroV2GatelayerMainnet.RECEIVE_ULN_302), LayerZeroV2GatelayerMainnet.EXECUTOR, LayerZeroV2GatelayerMainnet.CHAIN_ID, "gatelayer-mainnet");
        _registerChain(LayerZeroV2GlueMainnet.EID, address(LayerZeroV2GlueMainnet.ENDPOINT_V2), address(LayerZeroV2GlueMainnet.SEND_ULN_302), address(LayerZeroV2GlueMainnet.RECEIVE_ULN_302), LayerZeroV2GlueMainnet.EXECUTOR, LayerZeroV2GlueMainnet.CHAIN_ID, "glue-mainnet");
        _registerChain(LayerZeroV2GnosisMainnet.EID, address(LayerZeroV2GnosisMainnet.ENDPOINT_V2), address(LayerZeroV2GnosisMainnet.SEND_ULN_302), address(LayerZeroV2GnosisMainnet.RECEIVE_ULN_302), LayerZeroV2GnosisMainnet.EXECUTOR, LayerZeroV2GnosisMainnet.CHAIN_ID, "gnosis-mainnet");
        _registerChain(LayerZeroV2GoatMainnet.EID, address(LayerZeroV2GoatMainnet.ENDPOINT_V2), address(LayerZeroV2GoatMainnet.SEND_ULN_302), address(LayerZeroV2GoatMainnet.RECEIVE_ULN_302), LayerZeroV2GoatMainnet.EXECUTOR, LayerZeroV2GoatMainnet.CHAIN_ID, "goat-mainnet");
        _registerChain(LayerZeroV2GravityMainnet.EID, address(LayerZeroV2GravityMainnet.ENDPOINT_V2), address(LayerZeroV2GravityMainnet.SEND_ULN_302), address(LayerZeroV2GravityMainnet.RECEIVE_ULN_302), LayerZeroV2GravityMainnet.EXECUTOR, LayerZeroV2GravityMainnet.CHAIN_ID, "gravity-mainnet");
        _registerChain(LayerZeroV2GunzMainnet.EID, address(LayerZeroV2GunzMainnet.ENDPOINT_V2), address(LayerZeroV2GunzMainnet.SEND_ULN_302), address(LayerZeroV2GunzMainnet.RECEIVE_ULN_302), LayerZeroV2GunzMainnet.EXECUTOR, LayerZeroV2GunzMainnet.CHAIN_ID, "gunz-mainnet");
        _registerChain(LayerZeroV2HarmonyMainnet.EID, address(LayerZeroV2HarmonyMainnet.ENDPOINT_V2), address(LayerZeroV2HarmonyMainnet.SEND_ULN_302), address(LayerZeroV2HarmonyMainnet.RECEIVE_ULN_302), LayerZeroV2HarmonyMainnet.EXECUTOR, LayerZeroV2HarmonyMainnet.CHAIN_ID, "harmony-mainnet");
        _registerChain(LayerZeroV2HederaMainnet.EID, address(LayerZeroV2HederaMainnet.ENDPOINT_V2), address(LayerZeroV2HederaMainnet.SEND_ULN_302), address(LayerZeroV2HederaMainnet.RECEIVE_ULN_302), LayerZeroV2HederaMainnet.EXECUTOR, LayerZeroV2HederaMainnet.CHAIN_ID, "hedera-mainnet");
        _registerChain(LayerZeroV2HemiMainnet.EID, address(LayerZeroV2HemiMainnet.ENDPOINT_V2), address(LayerZeroV2HemiMainnet.SEND_ULN_302), address(LayerZeroV2HemiMainnet.RECEIVE_ULN_302), LayerZeroV2HemiMainnet.EXECUTOR, LayerZeroV2HemiMainnet.CHAIN_ID, "hemi-mainnet");
        _registerChain(LayerZeroV2HomeverseMainnet.EID, address(LayerZeroV2HomeverseMainnet.ENDPOINT_V2), address(LayerZeroV2HomeverseMainnet.SEND_ULN_302), address(LayerZeroV2HomeverseMainnet.RECEIVE_ULN_302), LayerZeroV2HomeverseMainnet.EXECUTOR, LayerZeroV2HomeverseMainnet.CHAIN_ID, "homeverse-mainnet");
        _registerChain(LayerZeroV2HubbleMainnet.EID, address(LayerZeroV2HubbleMainnet.ENDPOINT_V2), address(LayerZeroV2HubbleMainnet.SEND_ULN_302), address(LayerZeroV2HubbleMainnet.RECEIVE_ULN_302), LayerZeroV2HubbleMainnet.EXECUTOR, LayerZeroV2HubbleMainnet.CHAIN_ID, "hubble-mainnet");
        _registerChain(LayerZeroV2HumanityMainnet.EID, address(LayerZeroV2HumanityMainnet.ENDPOINT_V2), address(LayerZeroV2HumanityMainnet.SEND_ULN_302), address(LayerZeroV2HumanityMainnet.RECEIVE_ULN_302), LayerZeroV2HumanityMainnet.EXECUTOR, LayerZeroV2HumanityMainnet.CHAIN_ID, "humanity-mainnet");
        _registerChain(LayerZeroV2HyperliquidMainnet.EID, address(LayerZeroV2HyperliquidMainnet.ENDPOINT_V2), address(LayerZeroV2HyperliquidMainnet.SEND_ULN_302), address(LayerZeroV2HyperliquidMainnet.RECEIVE_ULN_302), LayerZeroV2HyperliquidMainnet.EXECUTOR, LayerZeroV2HyperliquidMainnet.CHAIN_ID, "hyperliquid-mainnet");
        _registerChain(LayerZeroV2InjectiveevmMainnet.EID, address(LayerZeroV2InjectiveevmMainnet.ENDPOINT_V2), address(LayerZeroV2InjectiveevmMainnet.SEND_ULN_302), address(LayerZeroV2InjectiveevmMainnet.RECEIVE_ULN_302), LayerZeroV2InjectiveevmMainnet.EXECUTOR, LayerZeroV2InjectiveevmMainnet.CHAIN_ID, "injectiveevm-mainnet");
        _registerChain(LayerZeroV2InkMainnet.EID, address(LayerZeroV2InkMainnet.ENDPOINT_V2), address(LayerZeroV2InkMainnet.SEND_ULN_302), address(LayerZeroV2InkMainnet.RECEIVE_ULN_302), LayerZeroV2InkMainnet.EXECUTOR, LayerZeroV2InkMainnet.CHAIN_ID, "ink-mainnet");
        _registerChain(LayerZeroV2IotaMainnet.EID, address(LayerZeroV2IotaMainnet.ENDPOINT_V2), address(LayerZeroV2IotaMainnet.SEND_ULN_302), address(LayerZeroV2IotaMainnet.RECEIVE_ULN_302), LayerZeroV2IotaMainnet.EXECUTOR, LayerZeroV2IotaMainnet.CHAIN_ID, "iota-mainnet");
        _registerChain(LayerZeroV2IslanderMainnet.EID, address(LayerZeroV2IslanderMainnet.ENDPOINT_V2), address(LayerZeroV2IslanderMainnet.SEND_ULN_302), address(LayerZeroV2IslanderMainnet.RECEIVE_ULN_302), LayerZeroV2IslanderMainnet.EXECUTOR, LayerZeroV2IslanderMainnet.CHAIN_ID, "islander-mainnet");
        _registerChain(LayerZeroV2JocMainnet.EID, address(LayerZeroV2JocMainnet.ENDPOINT_V2), address(LayerZeroV2JocMainnet.SEND_ULN_302), address(LayerZeroV2JocMainnet.RECEIVE_ULN_302), LayerZeroV2JocMainnet.EXECUTOR, LayerZeroV2JocMainnet.CHAIN_ID, "joc-mainnet");
        _registerChain(LayerZeroV2KatanaMainnet.EID, address(LayerZeroV2KatanaMainnet.ENDPOINT_V2), address(LayerZeroV2KatanaMainnet.SEND_ULN_302), address(LayerZeroV2KatanaMainnet.RECEIVE_ULN_302), LayerZeroV2KatanaMainnet.EXECUTOR, LayerZeroV2KatanaMainnet.CHAIN_ID, "katana-mainnet");
        _registerChain(LayerZeroV2KavaMainnet.EID, address(LayerZeroV2KavaMainnet.ENDPOINT_V2), address(LayerZeroV2KavaMainnet.SEND_ULN_302), address(LayerZeroV2KavaMainnet.RECEIVE_ULN_302), LayerZeroV2KavaMainnet.EXECUTOR, LayerZeroV2KavaMainnet.CHAIN_ID, "kava-mainnet");
        _registerChain(LayerZeroV2KlaytnMainnet.EID, address(LayerZeroV2KlaytnMainnet.ENDPOINT_V2), address(LayerZeroV2KlaytnMainnet.SEND_ULN_302), address(LayerZeroV2KlaytnMainnet.RECEIVE_ULN_302), LayerZeroV2KlaytnMainnet.EXECUTOR, LayerZeroV2KlaytnMainnet.CHAIN_ID, "klaytn-mainnet");
        _registerChain(LayerZeroV2LensMainnet.EID, address(LayerZeroV2LensMainnet.ENDPOINT_V2), address(LayerZeroV2LensMainnet.SEND_ULN_302), address(LayerZeroV2LensMainnet.RECEIVE_ULN_302), LayerZeroV2LensMainnet.EXECUTOR, LayerZeroV2LensMainnet.CHAIN_ID, "lens-mainnet");
        _registerChain(LayerZeroV2LightlinkMainnet.EID, address(LayerZeroV2LightlinkMainnet.ENDPOINT_V2), address(LayerZeroV2LightlinkMainnet.SEND_ULN_302), address(LayerZeroV2LightlinkMainnet.RECEIVE_ULN_302), LayerZeroV2LightlinkMainnet.EXECUTOR, LayerZeroV2LightlinkMainnet.CHAIN_ID, "lightlink-mainnet");
        _registerChain(LayerZeroV2LiskMainnet.EID, address(LayerZeroV2LiskMainnet.ENDPOINT_V2), address(LayerZeroV2LiskMainnet.SEND_ULN_302), address(LayerZeroV2LiskMainnet.RECEIVE_ULN_302), LayerZeroV2LiskMainnet.EXECUTOR, LayerZeroV2LiskMainnet.CHAIN_ID, "lisk-mainnet");
        _registerChain(LayerZeroV2LootMainnet.EID, address(LayerZeroV2LootMainnet.ENDPOINT_V2), address(LayerZeroV2LootMainnet.SEND_ULN_302), address(LayerZeroV2LootMainnet.RECEIVE_ULN_302), LayerZeroV2LootMainnet.EXECUTOR, LayerZeroV2LootMainnet.CHAIN_ID, "loot-mainnet");
        _registerChain(LayerZeroV2LyraMainnet.EID, address(LayerZeroV2LyraMainnet.ENDPOINT_V2), address(LayerZeroV2LyraMainnet.SEND_ULN_302), address(LayerZeroV2LyraMainnet.RECEIVE_ULN_302), LayerZeroV2LyraMainnet.EXECUTOR, LayerZeroV2LyraMainnet.CHAIN_ID, "lyra-mainnet");
        _registerChain(LayerZeroV2MantaMainnet.EID, address(LayerZeroV2MantaMainnet.ENDPOINT_V2), address(LayerZeroV2MantaMainnet.SEND_ULN_302), address(LayerZeroV2MantaMainnet.RECEIVE_ULN_302), LayerZeroV2MantaMainnet.EXECUTOR, LayerZeroV2MantaMainnet.CHAIN_ID, "manta-mainnet");
        _registerChain(LayerZeroV2MantleMainnet.EID, address(LayerZeroV2MantleMainnet.ENDPOINT_V2), address(LayerZeroV2MantleMainnet.SEND_ULN_302), address(LayerZeroV2MantleMainnet.RECEIVE_ULN_302), LayerZeroV2MantleMainnet.EXECUTOR, LayerZeroV2MantleMainnet.CHAIN_ID, "mantle-mainnet");
        _registerChain(LayerZeroV2MasaMainnet.EID, address(LayerZeroV2MasaMainnet.ENDPOINT_V2), address(LayerZeroV2MasaMainnet.SEND_ULN_302), address(LayerZeroV2MasaMainnet.RECEIVE_ULN_302), LayerZeroV2MasaMainnet.EXECUTOR, LayerZeroV2MasaMainnet.CHAIN_ID, "masa-mainnet");
        _registerChain(LayerZeroV2MeritcircleMainnet.EID, address(LayerZeroV2MeritcircleMainnet.ENDPOINT_V2), address(LayerZeroV2MeritcircleMainnet.SEND_ULN_302), address(LayerZeroV2MeritcircleMainnet.RECEIVE_ULN_302), LayerZeroV2MeritcircleMainnet.EXECUTOR, LayerZeroV2MeritcircleMainnet.CHAIN_ID, "meritcircle-mainnet");
        _registerChain(LayerZeroV2MerlinMainnet.EID, address(LayerZeroV2MerlinMainnet.ENDPOINT_V2), address(LayerZeroV2MerlinMainnet.SEND_ULN_302), address(LayerZeroV2MerlinMainnet.RECEIVE_ULN_302), LayerZeroV2MerlinMainnet.EXECUTOR, LayerZeroV2MerlinMainnet.CHAIN_ID, "merlin-mainnet");
        _registerChain(LayerZeroV2MeterMainnet.EID, address(LayerZeroV2MeterMainnet.ENDPOINT_V2), address(LayerZeroV2MeterMainnet.SEND_ULN_302), address(LayerZeroV2MeterMainnet.RECEIVE_ULN_302), LayerZeroV2MeterMainnet.EXECUTOR, LayerZeroV2MeterMainnet.CHAIN_ID, "meter-mainnet");
        _registerChain(LayerZeroV2MetisMainnet.EID, address(LayerZeroV2MetisMainnet.ENDPOINT_V2), address(LayerZeroV2MetisMainnet.SEND_ULN_302), address(LayerZeroV2MetisMainnet.RECEIVE_ULN_302), LayerZeroV2MetisMainnet.EXECUTOR, LayerZeroV2MetisMainnet.CHAIN_ID, "metis-mainnet");
        _registerChain(LayerZeroV2ModeMainnet.EID, address(LayerZeroV2ModeMainnet.ENDPOINT_V2), address(LayerZeroV2ModeMainnet.SEND_ULN_302), address(LayerZeroV2ModeMainnet.RECEIVE_ULN_302), LayerZeroV2ModeMainnet.EXECUTOR, LayerZeroV2ModeMainnet.CHAIN_ID, "mode-mainnet");
        _registerChain(LayerZeroV2MonadMainnet.EID, address(LayerZeroV2MonadMainnet.ENDPOINT_V2), address(LayerZeroV2MonadMainnet.SEND_ULN_302), address(LayerZeroV2MonadMainnet.RECEIVE_ULN_302), LayerZeroV2MonadMainnet.EXECUTOR, LayerZeroV2MonadMainnet.CHAIN_ID, "monad-mainnet");
        _registerChain(LayerZeroV2MoonbeamMainnet.EID, address(LayerZeroV2MoonbeamMainnet.ENDPOINT_V2), address(LayerZeroV2MoonbeamMainnet.SEND_ULN_302), address(LayerZeroV2MoonbeamMainnet.RECEIVE_ULN_302), LayerZeroV2MoonbeamMainnet.EXECUTOR, LayerZeroV2MoonbeamMainnet.CHAIN_ID, "moonbeam-mainnet");
        _registerChain(LayerZeroV2MoonriverMainnet.EID, address(LayerZeroV2MoonriverMainnet.ENDPOINT_V2), address(LayerZeroV2MoonriverMainnet.SEND_ULN_302), address(LayerZeroV2MoonriverMainnet.RECEIVE_ULN_302), LayerZeroV2MoonriverMainnet.EXECUTOR, LayerZeroV2MoonriverMainnet.CHAIN_ID, "moonriver-mainnet");
        _registerChain(LayerZeroV2MorphMainnet.EID, address(LayerZeroV2MorphMainnet.ENDPOINT_V2), address(LayerZeroV2MorphMainnet.SEND_ULN_302), address(LayerZeroV2MorphMainnet.RECEIVE_ULN_302), LayerZeroV2MorphMainnet.EXECUTOR, LayerZeroV2MorphMainnet.CHAIN_ID, "morph-mainnet");
        _registerChain(LayerZeroV2Mp1Mainnet.EID, address(LayerZeroV2Mp1Mainnet.ENDPOINT_V2), address(LayerZeroV2Mp1Mainnet.SEND_ULN_302), address(LayerZeroV2Mp1Mainnet.RECEIVE_ULN_302), LayerZeroV2Mp1Mainnet.EXECUTOR, LayerZeroV2Mp1Mainnet.CHAIN_ID, "mp1-mainnet");
        _registerChain(LayerZeroV2NexeraMainnet.EID, address(LayerZeroV2NexeraMainnet.ENDPOINT_V2), address(LayerZeroV2NexeraMainnet.SEND_ULN_302), address(LayerZeroV2NexeraMainnet.RECEIVE_ULN_302), LayerZeroV2NexeraMainnet.EXECUTOR, LayerZeroV2NexeraMainnet.CHAIN_ID, "nexera-mainnet");
        _registerChain(LayerZeroV2NibiruMainnet.EID, address(LayerZeroV2NibiruMainnet.ENDPOINT_V2), address(LayerZeroV2NibiruMainnet.SEND_ULN_302), address(LayerZeroV2NibiruMainnet.RECEIVE_ULN_302), LayerZeroV2NibiruMainnet.EXECUTOR, LayerZeroV2NibiruMainnet.CHAIN_ID, "nibiru-mainnet");
        _registerChain(LayerZeroV2NovaMainnet.EID, address(LayerZeroV2NovaMainnet.ENDPOINT_V2), address(LayerZeroV2NovaMainnet.SEND_ULN_302), address(LayerZeroV2NovaMainnet.RECEIVE_ULN_302), LayerZeroV2NovaMainnet.EXECUTOR, LayerZeroV2NovaMainnet.CHAIN_ID, "nova-mainnet");
        _registerChain(LayerZeroV2OgMainnet.EID, address(LayerZeroV2OgMainnet.ENDPOINT_V2), address(LayerZeroV2OgMainnet.SEND_ULN_302), address(LayerZeroV2OgMainnet.RECEIVE_ULN_302), LayerZeroV2OgMainnet.EXECUTOR, LayerZeroV2OgMainnet.CHAIN_ID, "og-mainnet");
        _registerChain(LayerZeroV2OkxMainnet.EID, address(LayerZeroV2OkxMainnet.ENDPOINT_V2), address(LayerZeroV2OkxMainnet.SEND_ULN_302), address(LayerZeroV2OkxMainnet.RECEIVE_ULN_302), LayerZeroV2OkxMainnet.EXECUTOR, LayerZeroV2OkxMainnet.CHAIN_ID, "okx-mainnet");
        _registerChain(LayerZeroV2OpbnbMainnet.EID, address(LayerZeroV2OpbnbMainnet.ENDPOINT_V2), address(LayerZeroV2OpbnbMainnet.SEND_ULN_302), address(LayerZeroV2OpbnbMainnet.RECEIVE_ULN_302), LayerZeroV2OpbnbMainnet.EXECUTOR, LayerZeroV2OpbnbMainnet.CHAIN_ID, "opbnb-mainnet");
        _registerChain(LayerZeroV2OpenledgerMainnet.EID, address(LayerZeroV2OpenledgerMainnet.ENDPOINT_V2), address(LayerZeroV2OpenledgerMainnet.SEND_ULN_302), address(LayerZeroV2OpenledgerMainnet.RECEIVE_ULN_302), LayerZeroV2OpenledgerMainnet.EXECUTOR, LayerZeroV2OpenledgerMainnet.CHAIN_ID, "openledger-mainnet");
        _registerChain(LayerZeroV2OptimismMainnet.EID, address(LayerZeroV2OptimismMainnet.ENDPOINT_V2), address(LayerZeroV2OptimismMainnet.SEND_ULN_302), address(LayerZeroV2OptimismMainnet.RECEIVE_ULN_302), LayerZeroV2OptimismMainnet.EXECUTOR, LayerZeroV2OptimismMainnet.CHAIN_ID, "optimism-mainnet");
        _registerChain(LayerZeroV2OrderlyMainnet.EID, address(LayerZeroV2OrderlyMainnet.ENDPOINT_V2), address(LayerZeroV2OrderlyMainnet.SEND_ULN_302), address(LayerZeroV2OrderlyMainnet.RECEIVE_ULN_302), LayerZeroV2OrderlyMainnet.EXECUTOR, LayerZeroV2OrderlyMainnet.CHAIN_ID, "orderly-mainnet");
        _registerChain(LayerZeroV2PeaqMainnet.EID, address(LayerZeroV2PeaqMainnet.ENDPOINT_V2), address(LayerZeroV2PeaqMainnet.SEND_ULN_302), address(LayerZeroV2PeaqMainnet.RECEIVE_ULN_302), LayerZeroV2PeaqMainnet.EXECUTOR, LayerZeroV2PeaqMainnet.CHAIN_ID, "peaq-mainnet");
        _registerChain(LayerZeroV2PlasmaMainnet.EID, address(LayerZeroV2PlasmaMainnet.ENDPOINT_V2), address(LayerZeroV2PlasmaMainnet.SEND_ULN_302), address(LayerZeroV2PlasmaMainnet.RECEIVE_ULN_302), LayerZeroV2PlasmaMainnet.EXECUTOR, LayerZeroV2PlasmaMainnet.CHAIN_ID, "plasma-mainnet");
        _registerChain(LayerZeroV2PlumeMainnet.EID, address(LayerZeroV2PlumeMainnet.ENDPOINT_V2), address(LayerZeroV2PlumeMainnet.SEND_ULN_302), address(LayerZeroV2PlumeMainnet.RECEIVE_ULN_302), LayerZeroV2PlumeMainnet.EXECUTOR, LayerZeroV2PlumeMainnet.CHAIN_ID, "plume-mainnet");
        _registerChain(LayerZeroV2PlumephoenixMainnet.EID, address(LayerZeroV2PlumephoenixMainnet.ENDPOINT_V2), address(LayerZeroV2PlumephoenixMainnet.SEND_ULN_302), address(LayerZeroV2PlumephoenixMainnet.RECEIVE_ULN_302), LayerZeroV2PlumephoenixMainnet.EXECUTOR, LayerZeroV2PlumephoenixMainnet.CHAIN_ID, "plumephoenix-mainnet");
        _registerChain(LayerZeroV2PolygonMainnet.EID, address(LayerZeroV2PolygonMainnet.ENDPOINT_V2), address(LayerZeroV2PolygonMainnet.SEND_ULN_302), address(LayerZeroV2PolygonMainnet.RECEIVE_ULN_302), LayerZeroV2PolygonMainnet.EXECUTOR, LayerZeroV2PolygonMainnet.CHAIN_ID, "polygon-mainnet");
        _registerChain(LayerZeroV2RaribleMainnet.EID, address(LayerZeroV2RaribleMainnet.ENDPOINT_V2), address(LayerZeroV2RaribleMainnet.SEND_ULN_302), address(LayerZeroV2RaribleMainnet.RECEIVE_ULN_302), LayerZeroV2RaribleMainnet.EXECUTOR, LayerZeroV2RaribleMainnet.CHAIN_ID, "rarible-mainnet");
        _registerChain(LayerZeroV2RealMainnet.EID, address(LayerZeroV2RealMainnet.ENDPOINT_V2), address(LayerZeroV2RealMainnet.SEND_ULN_302), address(LayerZeroV2RealMainnet.RECEIVE_ULN_302), LayerZeroV2RealMainnet.EXECUTOR, LayerZeroV2RealMainnet.CHAIN_ID, "real-mainnet");
        _registerChain(LayerZeroV2ReyaMainnet.EID, address(LayerZeroV2ReyaMainnet.ENDPOINT_V2), address(LayerZeroV2ReyaMainnet.SEND_ULN_302), address(LayerZeroV2ReyaMainnet.RECEIVE_ULN_302), LayerZeroV2ReyaMainnet.EXECUTOR, LayerZeroV2ReyaMainnet.CHAIN_ID, "reya-mainnet");
        _registerChain(LayerZeroV2RootstockMainnet.EID, address(LayerZeroV2RootstockMainnet.ENDPOINT_V2), address(LayerZeroV2RootstockMainnet.SEND_ULN_302), address(LayerZeroV2RootstockMainnet.RECEIVE_ULN_302), LayerZeroV2RootstockMainnet.EXECUTOR, LayerZeroV2RootstockMainnet.CHAIN_ID, "rootstock-mainnet");
        _registerChain(LayerZeroV2SankoMainnet.EID, address(LayerZeroV2SankoMainnet.ENDPOINT_V2), address(LayerZeroV2SankoMainnet.SEND_ULN_302), address(LayerZeroV2SankoMainnet.RECEIVE_ULN_302), LayerZeroV2SankoMainnet.EXECUTOR, LayerZeroV2SankoMainnet.CHAIN_ID, "sanko-mainnet");
        _registerChain(LayerZeroV2ScrollMainnet.EID, address(LayerZeroV2ScrollMainnet.ENDPOINT_V2), address(LayerZeroV2ScrollMainnet.SEND_ULN_302), address(LayerZeroV2ScrollMainnet.RECEIVE_ULN_302), LayerZeroV2ScrollMainnet.EXECUTOR, LayerZeroV2ScrollMainnet.CHAIN_ID, "scroll-mainnet");
        _registerChain(LayerZeroV2SeiMainnet.EID, address(LayerZeroV2SeiMainnet.ENDPOINT_V2), address(LayerZeroV2SeiMainnet.SEND_ULN_302), address(LayerZeroV2SeiMainnet.RECEIVE_ULN_302), LayerZeroV2SeiMainnet.EXECUTOR, LayerZeroV2SeiMainnet.CHAIN_ID, "sei-mainnet");
        _registerChain(LayerZeroV2ShimmerMainnet.EID, address(LayerZeroV2ShimmerMainnet.ENDPOINT_V2), address(LayerZeroV2ShimmerMainnet.SEND_ULN_302), address(LayerZeroV2ShimmerMainnet.RECEIVE_ULN_302), LayerZeroV2ShimmerMainnet.EXECUTOR, LayerZeroV2ShimmerMainnet.CHAIN_ID, "shimmer-mainnet");
        _registerChain(LayerZeroV2SiliconMainnet.EID, address(LayerZeroV2SiliconMainnet.ENDPOINT_V2), address(LayerZeroV2SiliconMainnet.SEND_ULN_302), address(LayerZeroV2SiliconMainnet.RECEIVE_ULN_302), LayerZeroV2SiliconMainnet.EXECUTOR, LayerZeroV2SiliconMainnet.CHAIN_ID, "silicon-mainnet");
        _registerChain(LayerZeroV2SkaleMainnet.EID, address(LayerZeroV2SkaleMainnet.ENDPOINT_V2), address(LayerZeroV2SkaleMainnet.SEND_ULN_302), address(LayerZeroV2SkaleMainnet.RECEIVE_ULN_302), LayerZeroV2SkaleMainnet.EXECUTOR, LayerZeroV2SkaleMainnet.CHAIN_ID, "skale-mainnet");
        _registerChain(LayerZeroV2SomniaMainnet.EID, address(LayerZeroV2SomniaMainnet.ENDPOINT_V2), address(LayerZeroV2SomniaMainnet.SEND_ULN_302), address(LayerZeroV2SomniaMainnet.RECEIVE_ULN_302), LayerZeroV2SomniaMainnet.EXECUTOR, LayerZeroV2SomniaMainnet.CHAIN_ID, "somnia-mainnet");
        _registerChain(LayerZeroV2SoneiumMainnet.EID, address(LayerZeroV2SoneiumMainnet.ENDPOINT_V2), address(LayerZeroV2SoneiumMainnet.SEND_ULN_302), address(LayerZeroV2SoneiumMainnet.RECEIVE_ULN_302), LayerZeroV2SoneiumMainnet.EXECUTOR, LayerZeroV2SoneiumMainnet.CHAIN_ID, "soneium-mainnet");
        _registerChain(LayerZeroV2SonicMainnet.EID, address(LayerZeroV2SonicMainnet.ENDPOINT_V2), address(LayerZeroV2SonicMainnet.SEND_ULN_302), address(LayerZeroV2SonicMainnet.RECEIVE_ULN_302), LayerZeroV2SonicMainnet.EXECUTOR, LayerZeroV2SonicMainnet.CHAIN_ID, "sonic-mainnet");
        _registerChain(LayerZeroV2SophonMainnet.EID, address(LayerZeroV2SophonMainnet.ENDPOINT_V2), address(LayerZeroV2SophonMainnet.SEND_ULN_302), address(LayerZeroV2SophonMainnet.RECEIVE_ULN_302), LayerZeroV2SophonMainnet.EXECUTOR, LayerZeroV2SophonMainnet.CHAIN_ID, "sophon-mainnet");
        _registerChain(LayerZeroV2SpaceMainnet.EID, address(LayerZeroV2SpaceMainnet.ENDPOINT_V2), address(LayerZeroV2SpaceMainnet.SEND_ULN_302), address(LayerZeroV2SpaceMainnet.RECEIVE_ULN_302), LayerZeroV2SpaceMainnet.EXECUTOR, LayerZeroV2SpaceMainnet.CHAIN_ID, "space-mainnet");
        _registerChain(LayerZeroV2StableMainnet.EID, address(LayerZeroV2StableMainnet.ENDPOINT_V2), address(LayerZeroV2StableMainnet.SEND_ULN_302), address(LayerZeroV2StableMainnet.RECEIVE_ULN_302), LayerZeroV2StableMainnet.EXECUTOR, LayerZeroV2StableMainnet.CHAIN_ID, "stable-mainnet");
        _registerChain(LayerZeroV2StoryMainnet.EID, address(LayerZeroV2StoryMainnet.ENDPOINT_V2), address(LayerZeroV2StoryMainnet.SEND_ULN_302), address(LayerZeroV2StoryMainnet.RECEIVE_ULN_302), LayerZeroV2StoryMainnet.EXECUTOR, LayerZeroV2StoryMainnet.CHAIN_ID, "story-mainnet");
        _registerChain(LayerZeroV2SubtensorevmMainnet.EID, address(LayerZeroV2SubtensorevmMainnet.ENDPOINT_V2), address(LayerZeroV2SubtensorevmMainnet.SEND_ULN_302), address(LayerZeroV2SubtensorevmMainnet.RECEIVE_ULN_302), LayerZeroV2SubtensorevmMainnet.EXECUTOR, LayerZeroV2SubtensorevmMainnet.CHAIN_ID, "subtensorevm-mainnet");
        _registerChain(LayerZeroV2SuperpositionMainnet.EID, address(LayerZeroV2SuperpositionMainnet.ENDPOINT_V2), address(LayerZeroV2SuperpositionMainnet.SEND_ULN_302), address(LayerZeroV2SuperpositionMainnet.RECEIVE_ULN_302), LayerZeroV2SuperpositionMainnet.EXECUTOR, LayerZeroV2SuperpositionMainnet.CHAIN_ID, "superposition-mainnet");
        _registerChain(LayerZeroV2SwellMainnet.EID, address(LayerZeroV2SwellMainnet.ENDPOINT_V2), address(LayerZeroV2SwellMainnet.SEND_ULN_302), address(LayerZeroV2SwellMainnet.RECEIVE_ULN_302), LayerZeroV2SwellMainnet.EXECUTOR, LayerZeroV2SwellMainnet.CHAIN_ID, "swell-mainnet");
        _registerChain(LayerZeroV2TacMainnet.EID, address(LayerZeroV2TacMainnet.ENDPOINT_V2), address(LayerZeroV2TacMainnet.SEND_ULN_302), address(LayerZeroV2TacMainnet.RECEIVE_ULN_302), LayerZeroV2TacMainnet.EXECUTOR, LayerZeroV2TacMainnet.CHAIN_ID, "tac-mainnet");
        _registerChain(LayerZeroV2TaikoMainnet.EID, address(LayerZeroV2TaikoMainnet.ENDPOINT_V2), address(LayerZeroV2TaikoMainnet.SEND_ULN_302), address(LayerZeroV2TaikoMainnet.RECEIVE_ULN_302), LayerZeroV2TaikoMainnet.EXECUTOR, LayerZeroV2TaikoMainnet.CHAIN_ID, "taiko-mainnet");
        _registerChain(LayerZeroV2TelosMainnet.EID, address(LayerZeroV2TelosMainnet.ENDPOINT_V2), address(LayerZeroV2TelosMainnet.SEND_ULN_302), address(LayerZeroV2TelosMainnet.RECEIVE_ULN_302), LayerZeroV2TelosMainnet.EXECUTOR, LayerZeroV2TelosMainnet.CHAIN_ID, "telos-mainnet");
        _registerChain(LayerZeroV2TenetMainnet.EID, address(LayerZeroV2TenetMainnet.ENDPOINT_V2), address(LayerZeroV2TenetMainnet.SEND_ULN_302), address(LayerZeroV2TenetMainnet.RECEIVE_ULN_302), LayerZeroV2TenetMainnet.EXECUTOR, LayerZeroV2TenetMainnet.CHAIN_ID, "tenet-mainnet");
        _registerChain(LayerZeroV2TiltyardMainnet.EID, address(LayerZeroV2TiltyardMainnet.ENDPOINT_V2), address(LayerZeroV2TiltyardMainnet.SEND_ULN_302), address(LayerZeroV2TiltyardMainnet.RECEIVE_ULN_302), LayerZeroV2TiltyardMainnet.EXECUTOR, LayerZeroV2TiltyardMainnet.CHAIN_ID, "tiltyard-mainnet");
        _registerChain(LayerZeroV2TomoMainnet.EID, address(LayerZeroV2TomoMainnet.ENDPOINT_V2), address(LayerZeroV2TomoMainnet.SEND_ULN_302), address(LayerZeroV2TomoMainnet.RECEIVE_ULN_302), LayerZeroV2TomoMainnet.EXECUTOR, LayerZeroV2TomoMainnet.CHAIN_ID, "tomo-mainnet");
        _registerChain(LayerZeroV2UnichainMainnet.EID, address(LayerZeroV2UnichainMainnet.ENDPOINT_V2), address(LayerZeroV2UnichainMainnet.SEND_ULN_302), address(LayerZeroV2UnichainMainnet.RECEIVE_ULN_302), LayerZeroV2UnichainMainnet.EXECUTOR, LayerZeroV2UnichainMainnet.CHAIN_ID, "unichain-mainnet");
        _registerChain(LayerZeroV2WorldchainMainnet.EID, address(LayerZeroV2WorldchainMainnet.ENDPOINT_V2), address(LayerZeroV2WorldchainMainnet.SEND_ULN_302), address(LayerZeroV2WorldchainMainnet.RECEIVE_ULN_302), LayerZeroV2WorldchainMainnet.EXECUTOR, LayerZeroV2WorldchainMainnet.CHAIN_ID, "worldchain-mainnet");
        _registerChain(LayerZeroV2XaiMainnet.EID, address(LayerZeroV2XaiMainnet.ENDPOINT_V2), address(LayerZeroV2XaiMainnet.SEND_ULN_302), address(LayerZeroV2XaiMainnet.RECEIVE_ULN_302), LayerZeroV2XaiMainnet.EXECUTOR, LayerZeroV2XaiMainnet.CHAIN_ID, "xai-mainnet");
        _registerChain(LayerZeroV2XchainMainnet.EID, address(LayerZeroV2XchainMainnet.ENDPOINT_V2), address(LayerZeroV2XchainMainnet.SEND_ULN_302), address(LayerZeroV2XchainMainnet.RECEIVE_ULN_302), LayerZeroV2XchainMainnet.EXECUTOR, LayerZeroV2XchainMainnet.CHAIN_ID, "xchain-mainnet");
        _registerChain(LayerZeroV2XdcMainnet.EID, address(LayerZeroV2XdcMainnet.ENDPOINT_V2), address(LayerZeroV2XdcMainnet.SEND_ULN_302), address(LayerZeroV2XdcMainnet.RECEIVE_ULN_302), LayerZeroV2XdcMainnet.EXECUTOR, LayerZeroV2XdcMainnet.CHAIN_ID, "xdc-mainnet");
        _registerChain(LayerZeroV2XlayerMainnet.EID, address(LayerZeroV2XlayerMainnet.ENDPOINT_V2), address(LayerZeroV2XlayerMainnet.SEND_ULN_302), address(LayerZeroV2XlayerMainnet.RECEIVE_ULN_302), LayerZeroV2XlayerMainnet.EXECUTOR, LayerZeroV2XlayerMainnet.CHAIN_ID, "xlayer-mainnet");
        _registerChain(LayerZeroV2XplaMainnet.EID, address(LayerZeroV2XplaMainnet.ENDPOINT_V2), address(LayerZeroV2XplaMainnet.SEND_ULN_302), address(LayerZeroV2XplaMainnet.RECEIVE_ULN_302), LayerZeroV2XplaMainnet.EXECUTOR, LayerZeroV2XplaMainnet.CHAIN_ID, "xpla-mainnet");
        _registerChain(LayerZeroV2ZircuitMainnet.EID, address(LayerZeroV2ZircuitMainnet.ENDPOINT_V2), address(LayerZeroV2ZircuitMainnet.SEND_ULN_302), address(LayerZeroV2ZircuitMainnet.RECEIVE_ULN_302), LayerZeroV2ZircuitMainnet.EXECUTOR, LayerZeroV2ZircuitMainnet.CHAIN_ID, "zircuit-mainnet");
        _registerChain(LayerZeroV2ZkatanaMainnet.EID, address(LayerZeroV2ZkatanaMainnet.ENDPOINT_V2), address(LayerZeroV2ZkatanaMainnet.SEND_ULN_302), address(LayerZeroV2ZkatanaMainnet.RECEIVE_ULN_302), LayerZeroV2ZkatanaMainnet.EXECUTOR, LayerZeroV2ZkatanaMainnet.CHAIN_ID, "zkatana-mainnet");
        _registerChain(LayerZeroV2ZkconsensysMainnet.EID, address(LayerZeroV2ZkconsensysMainnet.ENDPOINT_V2), address(LayerZeroV2ZkconsensysMainnet.SEND_ULN_302), address(LayerZeroV2ZkconsensysMainnet.RECEIVE_ULN_302), LayerZeroV2ZkconsensysMainnet.EXECUTOR, LayerZeroV2ZkconsensysMainnet.CHAIN_ID, "zkconsensys-mainnet");
        _registerChain(LayerZeroV2ZklinkMainnet.EID, address(LayerZeroV2ZklinkMainnet.ENDPOINT_V2), address(LayerZeroV2ZklinkMainnet.SEND_ULN_302), address(LayerZeroV2ZklinkMainnet.RECEIVE_ULN_302), LayerZeroV2ZklinkMainnet.EXECUTOR, LayerZeroV2ZklinkMainnet.CHAIN_ID, "zklink-mainnet");
        _registerChain(LayerZeroV2ZkpolygonMainnet.EID, address(LayerZeroV2ZkpolygonMainnet.ENDPOINT_V2), address(LayerZeroV2ZkpolygonMainnet.SEND_ULN_302), address(LayerZeroV2ZkpolygonMainnet.RECEIVE_ULN_302), LayerZeroV2ZkpolygonMainnet.EXECUTOR, LayerZeroV2ZkpolygonMainnet.CHAIN_ID, "zkpolygon-mainnet");
        _registerChain(LayerZeroV2ZksyncMainnet.EID, address(LayerZeroV2ZksyncMainnet.ENDPOINT_V2), address(LayerZeroV2ZksyncMainnet.SEND_ULN_302), address(LayerZeroV2ZksyncMainnet.RECEIVE_ULN_302), LayerZeroV2ZksyncMainnet.EXECUTOR, LayerZeroV2ZksyncMainnet.CHAIN_ID, "zksync-mainnet");
        _registerChain(LayerZeroV2ZkverifyMainnet.EID, address(LayerZeroV2ZkverifyMainnet.ENDPOINT_V2), address(LayerZeroV2ZkverifyMainnet.SEND_ULN_302), address(LayerZeroV2ZkverifyMainnet.RECEIVE_ULN_302), LayerZeroV2ZkverifyMainnet.EXECUTOR, LayerZeroV2ZkverifyMainnet.CHAIN_ID, "zkverify-mainnet");
        _registerChain(LayerZeroV2ZoraMainnet.EID, address(LayerZeroV2ZoraMainnet.ENDPOINT_V2), address(LayerZeroV2ZoraMainnet.SEND_ULN_302), address(LayerZeroV2ZoraMainnet.RECEIVE_ULN_302), LayerZeroV2ZoraMainnet.EXECUTOR, LayerZeroV2ZoraMainnet.CHAIN_ID, "zora-mainnet");
    }
    
    /// @notice Register a single chain
    function _registerChain(
        uint32 eid,
        address endpoint,
        address sendUln,
        address receiveUln,
        address executor,
        uint256 chainId,
        string memory chainName
    ) private {
        _protocolAddresses[eid] = ProtocolAddresses({
            endpointV2: endpoint,
            sendUln302: sendUln,
            receiveUln302: receiveUln,
            executor: executor,
            chainId: chainId,
            chainName: chainName,
            exists: true
        });
        _registeredEids.push(eid);
        _chainNameToEid[chainName] = eid;
    }
    
    function getProtocolAddresses(uint32 eid) public view override returns (ProtocolAddresses memory) {
        ProtocolAddresses memory addresses = _protocolAddresses[eid];
        require(addresses.exists, string.concat("Chain not registered: ", _toString(eid)));
        return addresses;
    }
    
    function isChainSupported(uint32 eid) public view override returns (bool) {
        return _protocolAddresses[eid].exists;
    }
    
    function getSupportedEids() public view override returns (uint32[] memory) {
        return _registeredEids;
    }
    
    /// @notice Get EID by chain name
    function getEidByChainName(string memory chainName) public view returns (uint32) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0 || _protocolAddresses[0].exists, "Chain name not found");
        return eid;
    }
    
    /// @notice Get EID by chain ID
    function getEidByChainId(uint256 chainId) public view returns (uint32) {
        // Auto-generated chain ID to EID mappings
        if (chainId == 1) return 30101; // ethereum-mainnet
        if (chainId == 10) return 30111; // optimism-mainnet
        if (chainId == 14) return 30295; // flare-mainnet
        if (chainId == 25) return 30359; // cronosevm-mainnet
        if (chainId == 30) return 30333; // rootstock-mainnet
        if (chainId == 37) return 30216; // xpla-mainnet
        if (chainId == 40) return 30199; // telos-mainnet
        if (chainId == 50) return 30365; // xdc-mainnet
        if (chainId == 56) return 30102; // bsc-mainnet
        if (chainId == 66) return 30155; // okx-mainnet
        if (chainId == 81) return 30285; // joc-mainnet
        if (chainId == 82) return 30176; // meter-mainnet
        if (chainId == 88) return 30196; // tomo-mainnet
        if (chainId == 100) return 30145; // gnosis-mainnet
        if (chainId == 122) return 30138; // fuse-mainnet
        if (chainId == 130) return 30320; // unichain-mainnet
        if (chainId == 137) return 30109; // polygon-mainnet
        if (chainId == 143) return 30390; // monad-mainnet
        if (chainId == 146) return 30332; // sonic-mainnet
        if (chainId == 148) return 30230; // shimmer-mainnet
        if (chainId == 169) return 30217; // manta-mainnet
        if (chainId == 196) return 30274; // xlayer-mainnet
        if (chainId == 204) return 30202; // opbnb-mainnet
        if (chainId == 232) return 30373; // lens-mainnet
        if (chainId == 239) return 30377; // tac-mainnet
        if (chainId == 250) return 30112; // fantom-mainnet
        if (chainId == 252) return 30255; // fraxtal-mainnet
        if (chainId == 291) return 30213; // orderly-mainnet
        if (chainId == 295) return 30316; // hedera-mainnet
        if (chainId == 324) return 30165; // zksync-mainnet
        if (chainId == 388) return 30360; // cronoszkevm-mainnet
        if (chainId == 424) return 30218; // pgn-mainnet
        if (chainId == 432) return 30400; // converge-mainnet
        if (chainId == 480) return 30319; // worldchain-mainnet
        if (chainId == 484) return 30381; // camp-mainnet
        if (chainId == 592) return 30210; // astar-mainnet
        if (chainId == 747) return 30336; // flow-mainnet
        if (chainId == 957) return 30311; // lyra-mainnet
        if (chainId == 964) return 30374; // subtensorevm-mainnet
        if (chainId == 988) return 30396; // stable-mainnet
        if (chainId == 999) return 30367; // hyperliquid-mainnet
        if (chainId == 1030) return 30212; // conflux-mainnet
        if (chainId == 1088) return 30151; // metis-mainnet
        if (chainId == 1101) return 30158; // zkpolygon-mainnet
        if (chainId == 1116) return 30153; // coredao-mainnet
        if (chainId == 1135) return 30321; // lisk-mainnet
        if (chainId == 1284) return 30126; // moonbeam-mainnet
        if (chainId == 1285) return 30167; // moonriver-mainnet
        if (chainId == 1300) return 30342; // glue-mainnet
        if (chainId == 1329) return 30280; // sei-mainnet
        if (chainId == 1408) return 30386; // zkverify-mainnet
        if (chainId == 1480) return 30330; // islander-mainnet
        if (chainId == 1514) return 30364; // story-mainnet
        if (chainId == 1559) return 30173; // tenet-mainnet
        if (chainId == 1612) return 30392; // openledger-mainnet
        if (chainId == 1625) return 30294; // gravity-mainnet
        if (chainId == 1729) return 30313; // reya-mainnet
        if (chainId == 1776) return 30394; // injectiveevm-mainnet
        if (chainId == 1868) return 30340; // soneium-mainnet
        if (chainId == 1890) return 30309; // lightlink-mainnet
        if (chainId == 1923) return 30335; // swell-mainnet
        if (chainId == 1992) return 30182; // hubble-mainnet
        if (chainId == 1996) return 30278; // sanko-mainnet
        if (chainId == 2044) return 30148; // shrapnel-mainnet
        if (chainId == 2222) return 30177; // kava-mainnet
        if (chainId == 2345) return 30361; // goat-mainnet
        if (chainId == 2355) return 30379; // silicon-mainnet
        if (chainId == 2525) return 30234; // bb1-mainnet
        if (chainId == 2741) return 30324; // abstract-mainnet
        if (chainId == 2818) return 30322; // morph-mainnet
        if (chainId == 3338) return 30302; // peaq-mainnet
        if (chainId == 3637) return 30376; // botanix-mainnet
        if (chainId == 3776) return 30257; // zkatana-mainnet
        if (chainId == 4200) return 30266; // merlin-mainnet
        if (chainId == 4337) return 30198; // meritcircle-mainnet
        if (chainId == 5000) return 30181; // mantle-mainnet
        if (chainId == 5165) return 30363; // bahamut-mainnet
        if (chainId == 6001) return 30293; // bouncebit-mainnet
        if (chainId == 6900) return 30369; // nibiru-mainnet
        if (chainId == 7208) return 30395; // nexera-mainnet
        if (chainId == 7332) return 30215; // eon-mainnet
        if (chainId == 7560) return 30283; // cyber-mainnet
        if (chainId == 7700) return 30159; // canto-mainnet
        if (chainId == 7979) return 30149; // dos-mainnet
        if (chainId == 8217) return 30150; // klaytn-mainnet
        if (chainId == 8227) return 30341; // space-mainnet
        if (chainId == 8453) return 30184; // base-mainnet
        if (chainId == 8822) return 30284; // iota-mainnet
        if (chainId == 9069) return 30384; // apexfusionnexus-mainnet
        if (chainId == 9745) return 30383; // plasma-mainnet
        if (chainId == 10088) return 30389; // gatelayer-mainnet
        if (chainId == 11501) return 30317; // bevm-mainnet
        if (chainId == 12739) return 30366; // concrete-mainnet
        if (chainId == 13396) return 30263; // masa-mainnet
        if (chainId == 16661) return 30388; // og-mainnet
        if (chainId == 19011) return 30265; // homeverse-mainnet
        if (chainId == 33139) return 30312; // ape-mainnet
        if (chainId == 34443) return 30260; // mode-mainnet
        if (chainId == 41923) return 30328; // edu-mainnet
        if (chainId == 42161) return 30110; // arbitrum-mainnet
        if (chainId == 42170) return 30175; // nova-mainnet
        if (chainId == 42220) return 30125; // celo-mainnet
        if (chainId == 42793) return 30292; // etherlink-mainnet
        if (chainId == 43111) return 30329; // hemi-mainnet
        if (chainId == 43114) return 30106; // avalanche-mainnet
        if (chainId == 43419) return 30371; // gunz-mainnet
        if (chainId == 48900) return 30303; // zircuit-mainnet
        if (chainId == 50104) return 30334; // sophon-mainnet
        if (chainId == 50312) return 30380; // somnia-mainnet
        if (chainId == 53935) return 30115; // dfk-mainnet
        if (chainId == 55244) return 30327; // superposition-mainnet
        if (chainId == 57073) return 30339; // ink-mainnet
        if (chainId == 59144) return 30183; // zkconsensys-mainnet
        if (chainId == 60808) return 30279; // bob-mainnet
        if (chainId == 68770) return 30315; // dm2verse-mainnet
        if (chainId == 69000) return 30372; // animechain-mainnet
        if (chainId == 80094) return 30362; // bera-mainnet
        if (chainId == 81224) return 30323; // codex-mainnet
        if (chainId == 81457) return 30243; // blast-mainnet
        if (chainId == 94524) return 30291; // xchain-mainnet
        if (chainId == 97477) return 30393; // doma-mainnet
        if (chainId == 98865) return 30318; // plume-mainnet
        if (chainId == 98866) return 30370; // plumephoenix-mainnet
        if (chainId == 98881) return 30282; // ebi-mainnet
        if (chainId == 111188) return 30237; // real-mainnet
        if (chainId == 167000) return 30290; // taiko-mainnet
        if (chainId == 200901) return 30314; // bitlayer-mainnet
        if (chainId == 202110) return 30385; // dinari-mainnet
        if (chainId == 432204) return 30118; // dexalot-mainnet
        if (chainId == 534352) return 30214; // scroll-mainnet
        if (chainId == 660279) return 30236; // xai-mainnet
        if (chainId == 710420) return 30238; // tiltyard-mainnet
        if (chainId == 747474) return 30375; // katana-mainnet
        if (chainId == 810180) return 30301; // zklink-mainnet
        if (chainId == 5064014) return 30391; // ethereal-mainnet
        if (chainId == 5151706) return 30197; // loot-mainnet
        if (chainId == 6985385) return 30382; // humanity-mainnet
        if (chainId == 7777777) return 30195; // zora-mainnet
        if (chainId == 21000000) return 30331; // mp1-mainnet
        if (chainId == 666666666) return 30267; // degen-mainnet
        if (chainId == 1313161554) return 30211; // aurora-mainnet
        if (chainId == 1380012617) return 30235; // rarible-mainnet
        if (chainId == 1666600000) return 30116; // harmony-mainnet
        if (chainId == 2046399126) return 30273; // skale-mainnet
        
        revert("Unsupported chain ID");
    }
    
    /// @notice Convert uint to string (helper function)
    function _toString(uint256 value) private pure returns (string memory) {
        if (value == 0) {
            return "0";
        }
        uint256 temp = value;
        uint256 digits;
        while (temp != 0) {
            digits++;
            temp /= 10;
        }
        bytes memory buffer = new bytes(digits);
        while (value != 0) {
            digits -= 1;
            buffer[digits] = bytes1(uint8(48 + uint256(value % 10)));
            value /= 10;
        }
        return string(buffer);
    }
}