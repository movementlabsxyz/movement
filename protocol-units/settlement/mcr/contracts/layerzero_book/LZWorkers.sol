// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "./interfaces/ILZWorkers.sol";

/// @title LayerZero Workers Registry
/// @notice Provides a registry of worker addresses (DVNs, executors) indexed by name and chain
/// @dev Maps worker names to their addresses on each chain for easy lookup
contract LZWorkers is ILZWorkers {
    // Storage: nested mapping for efficient lookups (dvnName => eid => address)
    mapping(string => mapping(uint32 => address)) private _dvnAddresses;
    
    // List of all DVN names for enumeration
    string[] private _dvnNames;
    mapping(string => bool) private _dvnNameExists;
    
    // Reverse mapping for chain name lookups
    mapping(string => uint32) private _chainNameToEid;
    mapping(uint32 => string) private _eidToChainName;
    
    // Track DVNs per chain for enumeration
    mapping(uint32 => string[]) private _dvnsByChain;
    
    constructor() {
        _registerAllDVNs();
        _registerChainMappings();
    }
    
    /// @notice Register all DVNs from metadata
    function _registerAllDVNs() private {

        // 01node
        _registerDVN("01node", 30110, 0x7A205ED4e3d7f9d0777594501705D8CD405c3B05); // arbitrum-mainnet
        _registerDVN("01node", 30106, 0xA80AA110f05C9C6140018aAE0C4E08A70f43350d); // avalanche-mainnet
        _registerDVN("01node", 30102, 0x8Fc629aa400D4D9c0B118F2685a49316552ABf27); // bsc-mainnet
        _registerDVN("01node", 30101, 0x58DfF8622759eA75910a08DBA5D060579271dcD7); // ethereum-mainnet
        _registerDVN("01node", 30112, 0x8Fc629aa400D4D9c0B118F2685a49316552ABf27); // fantom-mainnet
        _registerDVN("01node", 30111, 0x969A0bdd86A230345AD87A6a381DE5ED9E6cda85); // optimism-mainnet
        _registerDVN("01node", 30109, 0xf0809F6e760a5452Ee567975EdA7a28dA4a83D38); // polygon-mainnet

        // ABDB
        _registerDVN("ABDB", 30110, 0xddaa92ce2d2faC3f7c5eae19136E438902Ab46cc); // arbitrum-mainnet
        _registerDVN("ABDB", 30106, 0xFfe42DC3927A240f3459e5ec27EAaBD88727173E); // avalanche-mainnet
        _registerDVN("ABDB", 30362, 0xa2d10677441230C4AeD58030e4EA6Ba7Bfd80393); // bera-mainnet
        _registerDVN("ABDB", 30102, 0x313328609a9C38459CaE56625FFf7F2AD6dcde3b); // bsc-mainnet
        _registerDVN("ABDB", 30101, 0x7E65BDd15C8Db8995F80aBf0D6593b57dc8BE437); // ethereum-mainnet
        _registerDVN("ABDB", 30112, 0x313328609a9C38459CaE56625FFf7F2AD6dcde3b); // fantom-mainnet
        _registerDVN("ABDB", 30111, 0x7B8a0fD9D6ae5011d5cBD3E85Ed6D5510F98c9Bf); // optimism-mainnet
        _registerDVN("ABDB", 30109, 0xa6F5DDBF0Bd4D03334523465439D301080574742); // polygon-mainnet
        _registerDVN("ABDB", 30320, 0x1337834fd822065Af36a13657d2E847616129F3f); // unichain-mainnet

        // Axelar
        _registerDVN("Axelar", 30110, 0x9D3979c7E3DD26653C52256307709C09f47741e0); // arbitrum-mainnet
        _registerDVN("Axelar", 30106, 0xc390Fd7Ca590a505655eB6c454ed0783C99a2Ea9); // avalanche-mainnet
        _registerDVN("Axelar", 30243, 0xB830a5AfCBEBb936c30C607a18BbbA9f5B0a592f); // blast-mainnet
        _registerDVN("Axelar", 30102, 0x878c20D3685cdBc5e2680A8a0E7FB97389344fe1); // bsc-mainnet
        _registerDVN("Axelar", 30101, 0xCE5B47FA5139fC5f3c8c5f4C278ad5F56A7b2016); // ethereum-mainnet
        _registerDVN("Axelar", 30255, 0x025BAB5B7271790F9cF188FdcE2c4214857f48D3); // fraxtal-mainnet
        _registerDVN("Axelar", 30177, 0x80C4c3768dD5A3Dd105cf2BD868fdc50280E398B); // kava-mainnet
        _registerDVN("Axelar", 30181, 0x6e6359A9abe2E235eF2b82e48f0F93D1eC16aFbb); // mantle-mainnet
        _registerDVN("Axelar", 30111, 0x218B462e19d00c8feD4adbCe78f33aEf88d2ccFc); // optimism-mainnet
        _registerDVN("Axelar", 30214, 0x70CEDF51c199Fad12C6c0A71cD876af948059540); // scroll-mainnet

        // BCW
        _registerDVN("BCW", 30110, 0x78203678D264063815Dac114eA810E9837Cd80f7); // arbitrum-mainnet
        _registerDVN("BCW", 30210, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // astar-mainnet
        _registerDVN("BCW", 30211, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // aurora-mainnet
        _registerDVN("BCW", 30106, 0x7B8a0fD9D6ae5011d5cBD3E85Ed6D5510F98c9Bf); // avalanche-mainnet
        _registerDVN("BCW", 30184, 0xB3Ce0A5D132Cd9Bf965aBa435E650c55Edce0062); // base-mainnet
        _registerDVN("BCW", 30102, 0xd36246C322Ee102A2203bCA9cafb84c179D306F6); // bsc-mainnet
        _registerDVN("BCW", 30159, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // canto-mainnet
        _registerDVN("BCW", 30212, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // conflux-mainnet
        _registerDVN("BCW", 30153, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // coredao-mainnet
        _registerDVN("BCW", 30118, 0x58DfF8622759eA75910a08DBA5D060579271dcD7); // dexalot-mainnet
        _registerDVN("BCW", 30115, 0x6A110d94e1baA6984A3d904bab37Ae49b90E6B4f); // dfk-mainnet
        _registerDVN("BCW", 30149, 0x2AC038606fff3FB00317B8F0CcFB4081694aCDD0); // dos-mainnet
        _registerDVN("BCW", 30101, 0xe552485d02EDd3067FE7FCbD4dd56BB1D3A998D2); // ethereum-mainnet
        _registerDVN("BCW", 30138, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // fuse-mainnet
        _registerDVN("BCW", 30316, 0x97841D4AB18E9A923322A002d5b8Eb42b31Ccdb5); // hedera-mainnet
        _registerDVN("BCW", 30177, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // kava-mainnet
        _registerDVN("BCW", 30150, 0x28af4dADbc5066e994986E8bb105240023dC44B6); // klaytn-mainnet
        _registerDVN("BCW", 30197, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // loot-mainnet
        _registerDVN("BCW", 30217, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // manta-mainnet
        _registerDVN("BCW", 30181, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // mantle-mainnet
        _registerDVN("BCW", 30198, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // meritcircle-mainnet
        _registerDVN("BCW", 30176, 0xC4e1b199C3B24954022FcE7ba85419B3f0669142); // meter-mainnet
        _registerDVN("BCW", 30151, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // metis-mainnet
        _registerDVN("BCW", 30167, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // moonriver-mainnet
        _registerDVN("BCW", 30175, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // nova-mainnet
        _registerDVN("BCW", 30202, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // opbnb-mainnet
        _registerDVN("BCW", 30111, 0x73DDc92E39aEdA95FEb8D3E0008016d9F1268c76); // optimism-mainnet
        _registerDVN("BCW", 30213, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // orderly-mainnet
        _registerDVN("BCW", 30109, 0xd410dDB726991f372b69A05b006D2ae5A8CedBD6); // polygon-mainnet
        _registerDVN("BCW", 30214, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // scroll-mainnet
        _registerDVN("BCW", 30280, 0x1feB08B1A53A9710AfcE82D380B8c2833C69a37e); // sei-mainnet
        _registerDVN("BCW", 30230, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // shimmer-mainnet
        _registerDVN("BCW", 30199, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // telos-mainnet
        _registerDVN("BCW", 30173, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // tenet-mainnet
        _registerDVN("BCW", 30196, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // tomo-mainnet
        _registerDVN("BCW", 30236, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // xai-mainnet
        _registerDVN("BCW", 30216, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // xpla-mainnet
        _registerDVN("BCW", 30158, 0x12b4E588BeB7154519c0C6f737bB8cBa1D4E5BC7); // zkpolygon-mainnet
        _registerDVN("BCW", 30165, 0x0D1bc4Efd08940eB109Ef3040c1386d09B6334E0); // zksync-mainnet
        _registerDVN("BCW", 30195, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // zora-mainnet

        // BWare
        _registerDVN("BWare", 30110, 0x9bCd17A654bffAa6f8fEa38D19661a7210e22196); // arbitrum-mainnet
        _registerDVN("BWare", 30210, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // astar-mainnet
        _registerDVN("BWare", 30106, 0xcFf5b0608Fa638333f66e0dA9d4f1eB906Ac18e3); // avalanche-mainnet
        _registerDVN("BWare", 30184, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // base-mainnet
        _registerDVN("BWare", 30243, 0xabC9b1819cc4D9846550F928B985993cF6240439); // blast-mainnet
        _registerDVN("BWare", 30279, 0x58DfF8622759eA75910a08DBA5D060579271dcD7); // bob-mainnet
        _registerDVN("BWare", 30102, 0xfE1cD27827E16b07E61A4AC96b521bDB35e00328); // bsc-mainnet
        _registerDVN("BWare", 30101, 0x7a23612F07d81F16B26cF0b5a4C3eca0E8668df2); // ethereum-mainnet
        _registerDVN("BWare", 30112, 0x247624e2143504730aeC22912ed41F092498bEf2); // fantom-mainnet
        _registerDVN("BWare", 30145, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // gnosis-mainnet
        _registerDVN("BWare", 30294, 0xcced05c3667877B545285B25f19F794436A1c481); // gravity-mainnet
        _registerDVN("BWare", 30284, 0xD7bB44516b476ca805FB9d6fc5b508ef3Ee9448D); // iota-mainnet
        _registerDVN("BWare", 30217, 0xabC9b1819cc4D9846550F928B985993cF6240439); // manta-mainnet
        _registerDVN("BWare", 30181, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // mantle-mainnet
        _registerDVN("BWare", 30151, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // metis-mainnet
        _registerDVN("BWare", 30260, 0x10901f74caE315f674D3f6FC0645217FE4faD77C); // mode-mainnet
        _registerDVN("BWare", 30126, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // moonbeam-mainnet
        _registerDVN("BWare", 30167, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // moonriver-mainnet
        _registerDVN("BWare", 30175, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // nova-mainnet
        _registerDVN("BWare", 30155, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // okx-mainnet
        _registerDVN("BWare", 30202, 0x2AC038606fff3FB00317B8F0CcFB4081694aCDD0); // opbnb-mainnet
        _registerDVN("BWare", 30111, 0x19670Df5E16bEa2ba9b9e68b48C054C5bAEa06B8); // optimism-mainnet
        _registerDVN("BWare", 30302, 0x790d7B1E97a086eb0012393b65a5B32cE58a04Dc); // peaq-mainnet
        _registerDVN("BWare", 30109, 0x247624e2143504730aeC22912ed41F092498bEf2); // polygon-mainnet
        _registerDVN("BWare", 30214, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // scroll-mainnet
        _registerDVN("BWare", 30183, 0xF45742BbfaBCEe739eA2a2d0BA2dd140F1f2C6A3); // zkconsensys-mainnet
        _registerDVN("BWare", 30301, 0x1253E268Bc04bB43CB96D2F7Ee858b8A1433Cf6D); // zklink-mainnet
        _registerDVN("BWare", 30158, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // zkpolygon-mainnet
        _registerDVN("BWare", 30165, 0x3A5a74f863ec48c1769C4Ee85f6C3d70f5655E2A); // zksync-mainnet

        // Bera
        _registerDVN("Bera", 30110, 0xf2e89Ed7E342c708BA8CD79b293AD9244f5FCcb3); // arbitrum-mainnet
        _registerDVN("Bera", 30106, 0xF18F2C3d86Ec9A350D5E10Cb67c614201f210D3D); // avalanche-mainnet
        _registerDVN("Bera", 30362, 0x10473BD2f7320476B5E5E59649e3Dc129d9d0029); // bera-mainnet
        _registerDVN("Bera", 30102, 0x8ed0A851964604BB1b6b1a703F4c8234EE684d76); // bsc-mainnet
        _registerDVN("Bera", 30101, 0xE2e558C85E00B4d7529433C1cc78Ab678Cf62538); // ethereum-mainnet
        _registerDVN("Bera", 30112, 0x1a53015B6b4d88a943Ed512bD179FbD89a768B6b); // fantom-mainnet
        _registerDVN("Bera", 30111, 0x5F5512c760f69A338Cf2758d1E6A957571bB6ee0); // optimism-mainnet
        _registerDVN("Bera", 30109, 0xCF46153f01355036bF07E5f7Fb1eb262f25dFeDd); // polygon-mainnet

        // Bitgo
        _registerDVN("Bitgo", 30110, 0x0711dd777AE626ef5E0a4F50e199C7a0E0666857); // arbitrum-mainnet
        _registerDVN("Bitgo", 30106, 0xc18d69d1a83294d0886E1b79f241405F1fA86cB6); // avalanche-mainnet
        _registerDVN("Bitgo", 30184, 0x133e9fB2D339D8428476A714B1113B024343811E); // base-mainnet
        _registerDVN("Bitgo", 30362, 0xdfF3F73C260b3361d4F006B02972c6aF6C5c5417); // bera-mainnet
        _registerDVN("Bitgo", 30279, 0xaA391622e42aE501371CD745CE0BAD588a8C65fd); // bob-mainnet
        _registerDVN("Bitgo", 30102, 0xa2CEB887f545400B8247Dfb7E9cCAda7abAbBDE8); // bsc-mainnet
        _registerDVN("Bitgo", 30101, 0xc9ca319f6Da263910fd9B037eC3d817A814ef3d8); // ethereum-mainnet
        _registerDVN("Bitgo", 30112, 0x3b247F1B48F055EbF2DB593672B98C9597E3081E); // fantom-mainnet
        _registerDVN("Bitgo", 30336, 0xCCdeBdb5aCFD6Ae062859ac66653b10ED77586C2); // flow-mainnet
        _registerDVN("Bitgo", 30316, 0xA83A87a0bDce466edfBB6794404E1D7F556B8F20); // hedera-mainnet
        _registerDVN("Bitgo", 30367, 0xf55E9dAef79eeC17F76e800F059495F198ef8348); // hyperliquid-mainnet
        _registerDVN("Bitgo", 30390, 0xdD107f9B5209670840f9cD58e241F460651C16C4); // monad-mainnet
        _registerDVN("Bitgo", 30111, 0xF24Dc834039a1E39F6b99A51Df05Df9c91e35b2d); // optimism-mainnet
        _registerDVN("Bitgo", 30109, 0x02152F4624596602dCBB8B8EAD2988Ad44EDc865); // polygon-mainnet
        _registerDVN("Bitgo", 30280, 0x26cD5aBaDf7eC3f0F02b48314bfcA6b2342cddD4); // sei-mainnet
        _registerDVN("Bitgo", 30380, 0xdD9B12623ec1C7E744819708B9217b309fDE4080); // somnia-mainnet
        _registerDVN("Bitgo", 30340, 0x04584d612802A3a26b160E3F90341E6443dDB76A); // soneium-mainnet
        _registerDVN("Bitgo", 30332, 0xdfBb5C677dB41b5EF3a180509CDe27B5c9784655); // sonic-mainnet
        _registerDVN("Bitgo", 30335, 0x1269D1777bA8cF7498FE741869Ed4B73f2A47e93); // swell-mainnet
        _registerDVN("Bitgo", 30199, 0x31B8c7CD7226eA79E833FaBDcCbcA0fa38d6E0a1); // telos-mainnet
        _registerDVN("Bitgo", 30320, 0x6a4C9096F162f0ab3C0517B0a40dc1CE44785e16); // unichain-mainnet

        // Blockhunters
        _registerDVN("Blockhunters", 30110, 0xD074B6bbCBEC2f2B4c4265DE3D95e521f82bF669); // arbitrum-mainnet
        _registerDVN("Blockhunters", 30106, 0xD074B6bbCBEC2f2B4c4265DE3D95e521f82bF669); // avalanche-mainnet
        _registerDVN("Blockhunters", 30102, 0x547bF6889B1095b7CC6e525A1F8E8Fdb26134a38); // bsc-mainnet
        _registerDVN("Blockhunters", 30101, 0x6E70FCdc42D3d63748B7d8883399Dcb16BBB5c8c); // ethereum-mainnet
        _registerDVN("Blockhunters", 30112, 0x547bF6889B1095b7CC6e525A1F8E8Fdb26134a38); // fantom-mainnet
        _registerDVN("Blockhunters", 30111, 0xB3Ce0A5D132Cd9Bf965aBa435E650c55Edce0062); // optimism-mainnet
        _registerDVN("Blockhunters", 30109, 0xBD40c9047980500C46B8aed4462e2f889299FEbE); // polygon-mainnet

        // CCIP
        _registerDVN("CCIP", 30106, 0xd46270746acBcA85Dab8dE1CE1d71c46C2F2994C); // avalanche-mainnet
        _registerDVN("CCIP", 30102, 0x53561BcfE6B3F23BC72E5b9919c12322729942e8); // bsc-mainnet
        _registerDVN("CCIP", 30101, 0x771D10D0C86E26eA8d3b778ad4d31B30533B9Cbf); // ethereum-mainnet

        // Canary
        _registerDVN("Canary", 30324, 0xCB773CAf620D2A6703d2cd30C567A6c2906ccfbb); // abstract-mainnet
        _registerDVN("Canary", 30312, 0x9bB011796fC3604D3a4FaA5863f587a33F6224AF); // ape-mainnet
        _registerDVN("Canary", 30110, 0xf2E380c90e6c09721297526dbC74f870e114dfCb); // arbitrum-mainnet
        _registerDVN("Canary", 30210, 0x31B8c7CD7226eA79E833FaBDcCbcA0fa38d6E0a1); // astar-mainnet
        _registerDVN("Canary", 30211, 0xb4CaA217dD195B3B40eEe24b82c8093c2ea659cd); // aurora-mainnet
        _registerDVN("Canary", 30106, 0xcC49E6fca014c77E1Eb604351cc1E08C84511760); // avalanche-mainnet
        _registerDVN("Canary", 30363, 0x4FE90e0f2A99e464d6E97B161d72101CD03C20fe); // bahamut-mainnet
        _registerDVN("Canary", 30184, 0x554833698Ae0FB22ECC90B01222903fD62CA4B47); // base-mainnet
        _registerDVN("Canary", 30234, 0x2A22804Cfa992D5cb1324863ec4aDa920256B908); // bb1-mainnet
        _registerDVN("Canary", 30362, 0x06e8042729CeF3aE6D6DB5350f48F9D736C3675d); // bera-mainnet
        _registerDVN("Canary", 30317, 0x4FE90e0f2A99e464d6E97B161d72101CD03C20fe); // bevm-mainnet
        _registerDVN("Canary", 30243, 0x6398E91001Cc1682bBA103E6B2489Fa5675a5a64); // blast-mainnet
        _registerDVN("Canary", 30279, 0x46d6E532A913cDf688fb7863Ce1CF360a81Ec5E4); // bob-mainnet
        _registerDVN("Canary", 30376, 0xbCefdAdB8d24b1d36c26B522235012Cd4cf162f6); // botanix-mainnet
        _registerDVN("Canary", 30102, 0xfA9bA83C102283958B997Adc8B44ED3A3CdB5dDa); // bsc-mainnet
        _registerDVN("Canary", 30159, 0x73DDc92E39aEdA95FEb8D3E0008016d9F1268c76); // canto-mainnet
        _registerDVN("Canary", 30125, 0x94AAfe0A92A8300f0A2100A7f3DE47d6845747A9); // celo-mainnet
        _registerDVN("Canary", 30323, 0x391A2021483cB476D059a78130f95165C79604b7); // codex-mainnet
        _registerDVN("Canary", 30212, 0xE0F0FbBDBF9d398eCA0dd8c86d1F308D895b9Eb7); // conflux-mainnet
        _registerDVN("Canary", 30153, 0xC133Fd6b4c44277eD592E903C0585936D7585Fa5); // coredao-mainnet
        _registerDVN("Canary", 30359, 0xF1042Bba248634583d0678d53FB33Bc885E09F11); // cronosevm-mainnet
        _registerDVN("Canary", 30360, 0xc4A1F52fDA034A9A5E1B3b27D14451d15776Fef6); // cronoszkevm-mainnet
        _registerDVN("Canary", 30283, 0x3a855FCE450F7AEf93b14251D94CC4A47F9ff010); // cyber-mainnet
        _registerDVN("Canary", 30267, 0xf10Ea2c0D43bC4973cfBCc94eBAfC39d1D4aF118); // degen-mainnet
        _registerDVN("Canary", 30118, 0x9bB011796fC3604D3a4FaA5863f587a33F6224AF); // dexalot-mainnet
        _registerDVN("Canary", 30115, 0xAb6d3d37D8Dc309e7d8086B2e85a953b84Ee5fA9); // dfk-mainnet
        _registerDVN("Canary", 30149, 0x45A7305c65AAd28384F20a80F87a5183772E4F70); // dos-mainnet
        _registerDVN("Canary", 30328, 0x73ddc44AA34A838744c53AA23886e784a7B1F734); // edu-mainnet
        _registerDVN("Canary", 30391, 0x56053A8f4db677e5774F8Ee5BdD9D2dC270075f3); // ethereal-mainnet
        _registerDVN("Canary", 30101, 0xa4fE5A5B9A846458a70Cd0748228aED3bF65c2cd); // ethereum-mainnet
        _registerDVN("Canary", 30112, 0xE5BFfd46776251b70895517D4AB635a640dA61E9); // fantom-mainnet
        _registerDVN("Canary", 30295, 0xD791948db16AB4373FA394B74C727DDb7FB02520); // flare-mainnet
        _registerDVN("Canary", 30336, 0xe4e65D80DEb0E2c8391215bcBA4b5f7603420407); // flow-mainnet
        _registerDVN("Canary", 30255, 0x6398E91001Cc1682bBA103E6B2489Fa5675a5a64); // fraxtal-mainnet
        _registerDVN("Canary", 30138, 0x7A3D18E2324536294CD6F054cDde7c994f40391A); // fuse-mainnet
        _registerDVN("Canary", 30342, 0xe4e65D80DEb0E2c8391215bcBA4b5f7603420407); // glue-mainnet
        _registerDVN("Canary", 30145, 0x90EE303d4743F460B9a38415e09f3799b85a4efc); // gnosis-mainnet
        _registerDVN("Canary", 30361, 0x396dC0A78F789586E2982fCCD830C5954C193F3c); // goat-mainnet
        _registerDVN("Canary", 30294, 0xe9C24dD582e37FAACa7d44c799530688DE92Da73); // gravity-mainnet
        _registerDVN("Canary", 30371, 0x0D7cb4033e0C193F65B3639E61b6986A29Bf1DD4); // gunz-mainnet
        _registerDVN("Canary", 30116, 0xa6F5DDBF0Bd4D03334523465439D301080574742); // harmony-mainnet
        _registerDVN("Canary", 30316, 0x4b92BC2A7d681bf5230472C80d92aCFE9A6b9435); // hedera-mainnet
        _registerDVN("Canary", 30329, 0x396dC0A78F789586E2982fCCD830C5954C193F3c); // hemi-mainnet
        _registerDVN("Canary", 30382, 0x97841D4AB18E9A923322A002d5b8Eb42b31Ccdb5); // humanity-mainnet
        _registerDVN("Canary", 30367, 0x83342EC538dF0460e730a8F543Fe63063e2D44C4); // hyperliquid-mainnet
        _registerDVN("Canary", 30339, 0x1E4CE74ccf5498B19900649D9196e64BAb592451); // ink-mainnet
        _registerDVN("Canary", 30284, 0xeCbaA45c33ce6Fa284995e5F8314f5bC7F1C2008); // iota-mainnet
        _registerDVN("Canary", 30330, 0x45A7305c65AAd28384F20a80F87a5183772E4F70); // islander-mainnet
        _registerDVN("Canary", 30285, 0x5488a4ca201421cF100dC1B90D1dE5B26b421f64); // joc-mainnet
        _registerDVN("Canary", 30375, 0x53fF818a1c492e667E2cD0b5AFe0FC82c66d33c7); // katana-mainnet
        _registerDVN("Canary", 30177, 0x06b85533967179eD5bC9C754b84aE7d02f7eD830); // kava-mainnet
        _registerDVN("Canary", 30150, 0x1154d04d07AEe26ff2C200Bd373eb76a7e5694d6); // klaytn-mainnet
        _registerDVN("Canary", 30309, 0xF1042Bba248634583d0678d53FB33Bc885E09F11); // lightlink-mainnet
        _registerDVN("Canary", 30321, 0x0D155ec1Dfc983E919C318964fD16078408E99CC); // lisk-mainnet
        _registerDVN("Canary", 30197, 0xA1491AdA1168f04df32F72913fc3F27522950acf); // loot-mainnet
        _registerDVN("Canary", 30311, 0x047d9DBe4fC6B5c916F37237F547f9F42809935a); // lyra-mainnet
        _registerDVN("Canary", 30217, 0xDF44a1594d3D516f7CDFb4DC275a79a5F6e3Db1d); // manta-mainnet
        _registerDVN("Canary", 30181, 0xa2447e5B58D357c49Bf74B50B14421e6A100e525); // mantle-mainnet
        _registerDVN("Canary", 30198, 0xA1Bc1B9af01A0ec78883AA5DC7DECDCe897E1E76); // meritcircle-mainnet
        _registerDVN("Canary", 30266, 0x4134190B4CC18A9745ee0422CbC91c94F46a4cc1); // merlin-mainnet
        _registerDVN("Canary", 30151, 0xAf75bfD402f3d4EE84978179a6c87D16c4Bd1724); // metis-mainnet
        _registerDVN("Canary", 30260, 0x5D8aeD4182A8EcC47386e88Aa8753Dde7423996e); // mode-mainnet
        _registerDVN("Canary", 30390, 0x493626C5D852B9B187a9eb709D0b0978a3877238); // monad-mainnet
        _registerDVN("Canary", 30126, 0x33E5fcC13D7439cC62d54c41AA966197145b3Cd7); // moonbeam-mainnet
        _registerDVN("Canary", 30167, 0x8fa9eEf18c2A1459024f0B44714e5aCc1Ce7f5e8); // moonriver-mainnet
        _registerDVN("Canary", 30322, 0xf10Ea2c0D43bC4973cfBCc94eBAfC39d1D4aF118); // morph-mainnet
        _registerDVN("Canary", 30331, 0x5311241a20055f9C0b02d18d6c52F2b711c07B03); // mp1-mainnet
        _registerDVN("Canary", 30175, 0xE4193136B92bA91402313e95347c8e9FAD8d27d0); // nova-mainnet
        _registerDVN("Canary", 30388, 0x183940c4855a01da92bc2f96F7e0A8Aecbf797ff); // og-mainnet
        _registerDVN("Canary", 30155, 0x07653d28b0f53D4c54b70eb1f9025795B23a9D6e); // okx-mainnet
        _registerDVN("Canary", 30202, 0xE5491Fac6965Aa664EFD6d1aE5e7D1d56Da4FDDa); // opbnb-mainnet
        _registerDVN("Canary", 30111, 0x5b6735c66d97479cCD18294fc96B3084EcB2fa3f); // optimism-mainnet
        _registerDVN("Canary", 30213, 0xd42306DF1a805d8053Bc652cE0Cd9F62BDe80146); // orderly-mainnet
        _registerDVN("Canary", 30302, 0x0C0C8fd5351fd936A987c790d88B137df4E73D64); // peaq-mainnet
        _registerDVN("Canary", 30383, 0x2465eE263149A18d61c9224244c61a5871dc0473); // plasma-mainnet
        _registerDVN("Canary", 30370, 0x395B14700812cccC38b8e64F0a06ce2045FE9bA3); // plumephoenix-mainnet
        _registerDVN("Canary", 30109, 0x13feb7234Ff60A97af04477d6421415766753Ba3); // polygon-mainnet
        _registerDVN("Canary", 30333, 0xF1042Bba248634583d0678d53FB33Bc885E09F11); // rootstock-mainnet
        _registerDVN("Canary", 30278, 0x21cAF0BCE846AAA78C9f23C5A4eC5988EcBf9988); // sanko-mainnet
        _registerDVN("Canary", 30214, 0xDF44a1594d3D516f7CDFb4DC275a79a5F6e3Db1d); // scroll-mainnet
        _registerDVN("Canary", 30280, 0x33051Ad47157A50Bb49a646256b854C60f707C86); // sei-mainnet
        _registerDVN("Canary", 30230, 0x7E65BDd15C8Db8995F80aBf0D6593b57dc8BE437); // shimmer-mainnet
        _registerDVN("Canary", 30148, 0xE4193136B92bA91402313e95347c8e9FAD8d27d0); // shrapnel-mainnet
        _registerDVN("Canary", 30340, 0xdd1564F68aa802E30819F9E8043664584A8a3E87); // soneium-mainnet
        _registerDVN("Canary", 30332, 0xb2c7832aA8DDA878De6f949485f927e9e532E92C); // sonic-mainnet
        _registerDVN("Canary", 30334, 0x99aA6f70535873AC8167D69893a2CF70ECA544C3); // sophon-mainnet
        _registerDVN("Canary", 30341, 0xE4193136B92bA91402313e95347c8e9FAD8d27d0); // space-mainnet
        _registerDVN("Canary", 30396, 0x8D6cC20D84FbEb5733c60436CeB8957Da2ac02C8); // stable-mainnet
        _registerDVN("Canary", 30364, 0x77aAF86B4466A67869667BaBe02c6Ebe7E7791d6); // story-mainnet
        _registerDVN("Canary", 30374, 0xc71a3d16F00d93c78AB89aAEDde7C0012A3b26Cb); // subtensorevm-mainnet
        _registerDVN("Canary", 30327, 0x396dC0A78F789586E2982fCCD830C5954C193F3c); // superposition-mainnet
        _registerDVN("Canary", 30335, 0xa2447e5B58D357c49Bf74B50B14421e6A100e525); // swell-mainnet
        _registerDVN("Canary", 30377, 0x07ff86c392588254Ad10F0811dbBcad45f4C7d87); // tac-mainnet
        _registerDVN("Canary", 30290, 0xa9Ff468ad000A4D5729826459197a0dB843F433E); // taiko-mainnet
        _registerDVN("Canary", 30199, 0xae5273d893aD87c8f38D45E822A7b4321cCeFB4c); // telos-mainnet
        _registerDVN("Canary", 30173, 0xA1491AdA1168f04df32F72913fc3F27522950acf); // tenet-mainnet
        _registerDVN("Canary", 30320, 0x00A979a5D306E9c5F8Cf473659e75f8002E06fc8); // unichain-mainnet
        _registerDVN("Canary", 30319, 0xe9C24dD582e37FAACa7d44c799530688DE92Da73); // worldchain-mainnet
        _registerDVN("Canary", 30236, 0x7A3D18E2324536294CD6F054cDde7c994f40391A); // xai-mainnet
        _registerDVN("Canary", 30365, 0x307d81ef09c72730f57667bF1e9b62DB4904053f); // xdc-mainnet
        _registerDVN("Canary", 30274, 0x047d9DBe4fC6B5c916F37237F547f9F42809935a); // xlayer-mainnet
        _registerDVN("Canary", 30216, 0x73DDc92E39aEdA95FEb8D3E0008016d9F1268c76); // xpla-mainnet
        _registerDVN("Canary", 30303, 0xe552485d02EDd3067FE7FCbD4dd56BB1D3A998D2); // zircuit-mainnet
        _registerDVN("Canary", 30183, 0xDA63525a0Fc42Bcc2cAD1dD28708d5ed11849347); // zkconsensys-mainnet
        _registerDVN("Canary", 30301, 0x0D1bc4Efd08940eB109Ef3040c1386d09B6334E0); // zklink-mainnet
        _registerDVN("Canary", 30165, 0x05Db3a229293C09F639a16526bB2481704716Df0); // zksync-mainnet
        _registerDVN("Canary", 30195, 0x3AD8537B6936c596ca36D736063380810f7d3252); // zora-mainnet

        // Chainlink
        _registerDVN("Chainlink", 30110, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // arbitrum-mainnet
        _registerDVN("Chainlink", 30106, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // avalanche-mainnet
        _registerDVN("Chainlink", 30102, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // bsc-mainnet
        _registerDVN("Chainlink", 30101, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // ethereum-mainnet
        _registerDVN("Chainlink", 30112, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // fantom-mainnet
        _registerDVN("Chainlink", 30111, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // optimism-mainnet
        _registerDVN("Chainlink", 30109, 0x150A58e9E6BF69ccEb1DBA5ae97C166DC8792539); // polygon-mainnet

        // Curve
        _registerDVN("Curve", 30110, 0x4066b6e7BfD761B579902E7e8D03F4feb9B9536E); // arbitrum-mainnet
        _registerDVN("Curve", 30211, 0x315b0e76A510607bB0F706B17716F426D5b385b8); // aurora-mainnet
        _registerDVN("Curve", 30106, 0x6821Ea3d3a52421D2b7DF330f2316eD157314d7f); // avalanche-mainnet
        _registerDVN("Curve", 30184, 0x1a3A8421e48b7536f3F71d8B14A1449c90eFA909); // base-mainnet
        _registerDVN("Curve", 30102, 0xc6e1f0fc326913bD31fB11699c61F6f1F2A5e6d2); // bsc-mainnet
        _registerDVN("Curve", 30125, 0xc4305B4a16cb631CBd34b33380FB8c221cdf63Ab); // celo-mainnet
        _registerDVN("Curve", 30101, 0xcc35923c43893CC31F2815e216afD7EFB60f1fB0); // ethereum-mainnet
        _registerDVN("Curve", 30112, 0x06a32EFaFC7698c00e87f5225178d7364773E93b); // fantom-mainnet
        _registerDVN("Curve", 30255, 0x05dF4949f0B4dC4c4b1ADc0e01700Bc669E935c3); // fraxtal-mainnet
        _registerDVN("Curve", 30145, 0x30c673a1F34B91C4bf4951670A2B7c8C0663b100); // gnosis-mainnet
        _registerDVN("Curve", 30367, 0x8a41c07623cdF8995aE8769BfC45859D7cA99e82); // hyperliquid-mainnet
        _registerDVN("Curve", 30339, 0x18766cA3fEcDE0C1251922Be6D3a088aDf5f53e6); // ink-mainnet
        _registerDVN("Curve", 30177, 0x05dF4949f0B4dC4c4b1ADc0e01700Bc669E935c3); // kava-mainnet
        _registerDVN("Curve", 30181, 0xF18F2C3d86Ec9A350D5E10Cb67c614201f210D3D); // mantle-mainnet
        _registerDVN("Curve", 30126, 0x7DEcC6Df3aF9CFc275E25d2f9703eCF7ad800D5D); // moonbeam-mainnet
        _registerDVN("Curve", 30331, 0xDD779AAAd20E62275457Af91b123bB13dd5aFd0B); // mp1-mainnet
        _registerDVN("Curve", 30111, 0xb908Fc507fE3145e855cf63127349756b9eCF3a6); // optimism-mainnet
        _registerDVN("Curve", 30370, 0x8Ede21203E062D7D1EAeC11c4c72Ad04cDc15658); // plumephoenix-mainnet
        _registerDVN("Curve", 30109, 0x4066b6e7BfD761B579902E7e8D03F4feb9B9536E); // polygon-mainnet
        _registerDVN("Curve", 30332, 0x3c4e4c3baA5D883654970fe34Db8c943ebf43ab2); // sonic-mainnet
        _registerDVN("Curve", 30377, 0x4d45727d2de5ffC811E6A161c3a7233a2Ea2e0E7); // tac-mainnet
        _registerDVN("Curve", 30290, 0x8bAFe0299CB4D3fF75d3f7045554474Bf414FD11); // taiko-mainnet
        _registerDVN("Curve", 30365, 0xAb6d3d37D8Dc309e7d8086B2e85a953b84Ee5fA9); // xdc-mainnet
        _registerDVN("Curve", 30274, 0x1a92C25Cb7Cd80E1138E8125FC0a0B0642688C0B); // xlayer-mainnet

        // Delegate
        _registerDVN("Delegate", 30110, 0xdf30C9f6A70cE65A152c5Bd09826525D7E97Ba49); // arbitrum-mainnet
        _registerDVN("Delegate", 30106, 0x83d06212b6647B0d0865e730270751e3FDF5036E); // avalanche-mainnet
        _registerDVN("Delegate", 30102, 0x9EEee79F5dBC4D99354b5CB547c138Af432F937b); // bsc-mainnet
        _registerDVN("Delegate", 30101, 0x87048402c32632B7c4d0A892d82bC1160E8B2393); // ethereum-mainnet
        _registerDVN("Delegate", 30112, 0x9EEee79F5dBC4D99354b5CB547c138Af432F937b); // fantom-mainnet
        _registerDVN("Delegate", 30111, 0x7A205ED4e3d7f9d0777594501705D8CD405c3B05); // optimism-mainnet
        _registerDVN("Delegate", 30109, 0x4D52f5bc932cf1A854381A85ad9ED79B8497c153); // polygon-mainnet

        // Deutsche Telekom
        _registerDVN("Deutsche Telekom", 30110, 0xEae839784e5F6C79bBaf34b6023a2f62e134AB39); // arbitrum-mainnet
        _registerDVN("Deutsche Telekom", 30106, 0xbe57e9E7d9eB16B92C6383792aBe28D64a18c0F1); // avalanche-mainnet
        _registerDVN("Deutsche Telekom", 30184, 0xc2A0C36f5939A14966705c7Cec813163FaEEa1F0); // base-mainnet
        _registerDVN("Deutsche Telekom", 30362, 0x0273fbfF931704884668a9eFe50e7a2b3FC30505); // bera-mainnet
        _registerDVN("Deutsche Telekom", 30376, 0x7A3D18E2324536294CD6F054cDde7c994f40391A); // botanix-mainnet
        _registerDVN("Deutsche Telekom", 30102, 0xf0a5C5306adbFd4e3dfD5d4B148B451c411d3878); // bsc-mainnet
        _registerDVN("Deutsche Telekom", 30125, 0xEa928f8E62F3DAc51288056015B1D4e3eCfacdAC); // celo-mainnet
        _registerDVN("Deutsche Telekom", 30212, 0x45A7305c65AAd28384F20a80F87a5183772E4F70); // conflux-mainnet
        _registerDVN("Deutsche Telekom", 30153, 0xDe79818C75649773Fc462E9d3134b23B81741481); // coredao-mainnet
        _registerDVN("Deutsche Telekom", 30101, 0x373a6E5c0C4E89E24819f00AA37ea370917AAfF4); // ethereum-mainnet
        _registerDVN("Deutsche Telekom", 30112, 0x8181F551c95928c0648d4378Dc4d95E847bc3945); // fantom-mainnet
        _registerDVN("Deutsche Telekom", 30145, 0x93d2d7AADC9F2Cf5EbC88e9703E06dB09b8Fd85B); // gnosis-mainnet
        _registerDVN("Deutsche Telekom", 30367, 0x32fFd21260172518A8844feC76A88C8F239C384b); // hyperliquid-mainnet
        _registerDVN("Deutsche Telekom", 30375, 0x7cC59B5062A8291804A21a2a793c6Ce9ea2f0Eb9); // katana-mainnet
        _registerDVN("Deutsche Telekom", 30150, 0xca29B2be45F1D609189dc467e0f1E48ee202eD0E); // klaytn-mainnet
        _registerDVN("Deutsche Telekom", 30181, 0x45f1d581F704B3203d0a4EAb2A572658d7A2E678); // mantle-mainnet
        _registerDVN("Deutsche Telekom", 30390, 0x2c7185f5B0976397d9eB5c19d639d4005e6708f0); // monad-mainnet
        _registerDVN("Deutsche Telekom", 30388, 0x2EF2097f8C2467A0e274C9022142dc91aaE457A8); // og-mainnet
        _registerDVN("Deutsche Telekom", 30111, 0x427bd19a0463fc4eDc2e247d35eB61323d7E5541); // optimism-mainnet
        _registerDVN("Deutsche Telekom", 30383, 0xF81da1B0f3ac725503AD0c2c229d1Edc57204787); // plasma-mainnet
        _registerDVN("Deutsche Telekom", 30109, 0x5CcCb8DE6Cdba9D2Af9d84465653af7390FDf9Dd); // polygon-mainnet
        _registerDVN("Deutsche Telekom", 30332, 0xDe79818C75649773Fc462E9d3134b23B81741481); // sonic-mainnet
        _registerDVN("Deutsche Telekom", 30374, 0x58DfF8622759eA75910a08DBA5D060579271dcD7); // subtensorevm-mainnet
        _registerDVN("Deutsche Telekom", 30377, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // tac-mainnet
        _registerDVN("Deutsche Telekom", 30365, 0xdFDB9b369eF5821e9E6cB9B3329C74C38fe93194); // xdc-mainnet
        _registerDVN("Deutsche Telekom", 30183, 0xa2d10677441230C4AeD58030e4EA6Ba7Bfd80393); // zkconsensys-mainnet

        // EigenZero
        _registerDVN("EigenZero", 30106, 0xD3333aA4fA669D3eB036676ec01CB0ACaaEc0cc0); // avalanche-mainnet
        _registerDVN("EigenZero", 30102, 0x9188B373378D284C9174AE474C2B0A937924B34B); // bsc-mainnet
        _registerDVN("EigenZero", 30101, 0x4184Dd22692c8B50D8d7ee0d7B6028e45dbf8108); // ethereum-mainnet

        // FBTC
        _registerDVN("FBTC", 30362, 0xE900e073BADaFdC6F72541F34E6b701bde835487); // bera-mainnet
        _registerDVN("FBTC", 30101, 0x45C09Dc691B0Ad798e10D38B97e9Dfd917d4B680); // ethereum-mainnet

        // Flowdesk
        _registerDVN("Flowdesk", 30110, 0xc07125d75BfA05A0108De0f64c4D6Ebb12B357F6); // arbitrum-mainnet
        _registerDVN("Flowdesk", 30106, 0x795c62387ef3022b61F2C705BfBE5d94a78B971d); // avalanche-mainnet
        _registerDVN("Flowdesk", 30102, 0x00E91548787Caf130D811EF1872f2Bc2C0583d90); // bsc-mainnet
        _registerDVN("Flowdesk", 30101, 0x2fdBb1e2419e13a7e043D07EAf412934B73ad7a8); // ethereum-mainnet
        _registerDVN("Flowdesk", 30112, 0x4C4552785d09a422231A1fB3da02b49a3e99426c); // fantom-mainnet
        _registerDVN("Flowdesk", 30111, 0x57F61546bd2259Db04EE7132AC385e5447174403); // optimism-mainnet
        _registerDVN("Flowdesk", 30109, 0x2cabF8F2c0AAe35A771a1c19487684E94388B03a); // polygon-mainnet

        // Frax
        _registerDVN("Frax", 30324, 0xA8c83FEbAb692d6F08cfA303e5D53B3B34F9046d); // abstract-mainnet
        _registerDVN("Frax", 30110, 0xB42726e41dBE96fc4ea6d73Cd792167608353698); // arbitrum-mainnet
        _registerDVN("Frax", 30211, 0x045d70bf1939aF0489cb44533750A2E15c92D7D4); // aurora-mainnet
        _registerDVN("Frax", 30106, 0xfe4c37cd401F58eE0bF4D214447bF306C2BBd41B); // avalanche-mainnet
        _registerDVN("Frax", 30184, 0x187cF227F81c287303ee765eE001e151347FAaA2); // base-mainnet
        _registerDVN("Frax", 30362, 0x559A194dAe0508342e7CE1aD96e7E90A77F8BC4c); // bera-mainnet
        _registerDVN("Frax", 30243, 0xfa06f93ad99825114C8f8738943734b07FdD162F); // blast-mainnet
        _registerDVN("Frax", 30376, 0xf835Af1DceA24C255149E0ad7C9FF1a5E8611Fa2); // botanix-mainnet
        _registerDVN("Frax", 30102, 0xd4Bf35cE3BfC7F7D7dfC0694a7d4aA8b8c60a38c); // bsc-mainnet
        _registerDVN("Frax", 30101, 0x38654142F5E672Ae86a1b21523AAfC765E6A1e08); // ethereum-mainnet
        _registerDVN("Frax", 30255, 0x26cD5aBaDf7eC3f0F02b48314bfcA6b2342cddD4); // fraxtal-mainnet
        _registerDVN("Frax", 30367, 0x082A08F524C043ec7F6b9a42BAE79A1990D8499a); // hyperliquid-mainnet
        _registerDVN("Frax", 30339, 0xF007f1Fef50C0aCAF4418741454BCAEaeCB96B87); // ink-mainnet
        _registerDVN("Frax", 30375, 0x5FA12ebC08e183C1F5d44678cF897edEfe68738B); // katana-mainnet
        _registerDVN("Frax", 30260, 0x315b0e76A510607bB0F706B17716F426D5b385b8); // mode-mainnet
        _registerDVN("Frax", 30111, 0x7240264781Aa2f97Cb994C6231297A8606483242); // optimism-mainnet
        _registerDVN("Frax", 30383, 0x0a5a808dc5f9280B26EBE11b356D200e34a48517); // plasma-mainnet
        _registerDVN("Frax", 30370, 0x1E4CE74ccf5498B19900649D9196e64BAb592451); // plumephoenix-mainnet
        _registerDVN("Frax", 30109, 0xdab6E6ecB3513A8D2614AD75199b4b264A731050); // polygon-mainnet
        _registerDVN("Frax", 30214, 0x93bB6f93Fa90a18e88A27bcfBcB048F7e14733c6); // scroll-mainnet
        _registerDVN("Frax", 30280, 0xE016F0f39fb7DCf14E9412D92f2049668d4D2612); // sei-mainnet
        _registerDVN("Frax", 30332, 0x805ed883FA3453E7Ac588667785a4495C573Cd13); // sonic-mainnet
        _registerDVN("Frax", 30396, 0x45A7305c65AAd28384F20a80F87a5183772E4F70); // stable-mainnet
        _registerDVN("Frax", 30320, 0x97faa2a9c9bf8B4082B175A5B894Ce6bac6697a8); // unichain-mainnet
        _registerDVN("Frax", 30319, 0xE16561B56BDf003B785347d237905BaE24f5F973); // worldchain-mainnet
        _registerDVN("Frax", 30274, 0x2Ae36A544b904F2f2960F6Fd1a6084b4b11ba334); // xlayer-mainnet
        _registerDVN("Frax", 30183, 0xF4064220871e3B94Ca6aB3b0CEE8e29178bF47dE); // zkconsensys-mainnet
        _registerDVN("Frax", 30158, 0x651b1cf59014420112f8B7fCFDA840a16Ad763e0); // zkpolygon-mainnet
        _registerDVN("Frax", 30165, 0x2Eb85384CAd49A67eBd8e2AfB0f72B3F586bAF03); // zksync-mainnet

        // Gitcoin
        _registerDVN("Gitcoin", 30110, 0x82c1D3209E32BEB7A55069B2C8AF5b6F62e2DBD1); // arbitrum-mainnet
        _registerDVN("Gitcoin", 30106, 0xe9d8f20AE42E4634B07b9c994e23CE71c266D205); // avalanche-mainnet
        _registerDVN("Gitcoin", 30102, 0xE4b64c3a70cD7F7C38D48F66BA5Db9A41C485f64); // bsc-mainnet
        _registerDVN("Gitcoin", 30101, 0x15FaDad87913c3bb95f8f9f2e2b287E71Ba7817D); // ethereum-mainnet
        _registerDVN("Gitcoin", 30112, 0xF4213560F316007082731e8574e1A9630F91f1b1); // fantom-mainnet
        _registerDVN("Gitcoin", 30111, 0x37a37d958a43810Ec169eeCE501C525664ddF890); // optimism-mainnet
        _registerDVN("Gitcoin", 30109, 0xeCB3ac94976F11a53ae74Af1f36FB89E247FAEEF); // polygon-mainnet

        // Google
        _registerDVN("Google", 30110, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // arbitrum-mainnet
        _registerDVN("Google", 30106, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // avalanche-mainnet
        _registerDVN("Google", 30184, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // base-mainnet
        _registerDVN("Google", 30102, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // bsc-mainnet
        _registerDVN("Google", 30125, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // celo-mainnet
        _registerDVN("Google", 30101, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // ethereum-mainnet
        _registerDVN("Google", 30112, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // fantom-mainnet
        _registerDVN("Google", 30145, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // gnosis-mainnet
        _registerDVN("Google", 30116, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // harmony-mainnet
        _registerDVN("Google", 30126, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // moonbeam-mainnet
        _registerDVN("Google", 30175, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // nova-mainnet
        _registerDVN("Google", 30111, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // optimism-mainnet
        _registerDVN("Google", 30109, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // polygon-mainnet
        _registerDVN("Google", 30183, 0xD56e4eAb23cb81f43168F9F45211Eb027b9aC7cc); // zkconsensys-mainnet

        // Horizen
        _registerDVN("Horizen", 30324, 0x264fE346Fcd0A89E3B41A6499BAC80dEa7e908D2); // abstract-mainnet
        _registerDVN("Horizen", 30312, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // ape-mainnet
        _registerDVN("Horizen", 30110, 0x19670Df5E16bEa2ba9b9e68b48C054C5bAEa06B8); // arbitrum-mainnet
        _registerDVN("Horizen", 30210, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // astar-mainnet
        _registerDVN("Horizen", 30211, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // aurora-mainnet
        _registerDVN("Horizen", 30106, 0x07C05EaB7716AcB6f83ebF6268F8EECDA8892Ba1); // avalanche-mainnet
        _registerDVN("Horizen", 30363, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bahamut-mainnet
        _registerDVN("Horizen", 30184, 0xa7b5189bcA84Cd304D8553977c7C614329750d99); // base-mainnet
        _registerDVN("Horizen", 30234, 0xC9c1B26505bf3f4D6562159a119f6eDe1e245DeB); // bb1-mainnet
        _registerDVN("Horizen", 30362, 0xeCbaA45c33ce6Fa284995e5F8314f5bC7F1C2008); // bera-mainnet
        _registerDVN("Horizen", 30317, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bevm-mainnet
        _registerDVN("Horizen", 30314, 0x95729Ea44326f8adD8A9b1d987279DBdC1DD3dFf); // bitlayer-mainnet
        _registerDVN("Horizen", 30243, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // blast-mainnet
        _registerDVN("Horizen", 30279, 0xf2067660520F79eB7a8326Dc1266DCE0167D64E7); // bob-mainnet
        _registerDVN("Horizen", 30376, 0x713d826aaa1f974c1dc0472fC71219e93c3B1614); // botanix-mainnet
        _registerDVN("Horizen", 30293, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // bouncebit-mainnet
        _registerDVN("Horizen", 30102, 0x247624e2143504730aeC22912ed41F092498bEf2); // bsc-mainnet
        _registerDVN("Horizen", 30159, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // canto-mainnet
        _registerDVN("Horizen", 30125, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // celo-mainnet
        _registerDVN("Horizen", 30323, 0x5131E3A44C499B11Bd694d1070635cf49EBFeBf3); // codex-mainnet
        _registerDVN("Horizen", 30366, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // concrete-mainnet
        _registerDVN("Horizen", 30212, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // conflux-mainnet
        _registerDVN("Horizen", 30153, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // coredao-mainnet
        _registerDVN("Horizen", 30359, 0xE33de1A8cf9bcdC6b509C44EEF66f47c65dA6d47); // cronosevm-mainnet
        _registerDVN("Horizen", 30360, 0x1253E268Bc04bB43CB96D2F7Ee858b8A1433Cf6D); // cronoszkevm-mainnet
        _registerDVN("Horizen", 30283, 0x9885110b909E88bb94f7f767A68ec2558B2AfA73); // cyber-mainnet
        _registerDVN("Horizen", 30267, 0x01a998260Da061EfB9a85b26d42F8f8662bF3d5F); // degen-mainnet
        _registerDVN("Horizen", 30118, 0xd42306DF1a805d8053Bc652cE0Cd9F62BDe80146); // dexalot-mainnet
        _registerDVN("Horizen", 30115, 0xa9Ff468ad000A4D5729826459197a0dB843F433E); // dfk-mainnet
        _registerDVN("Horizen", 30393, 0xf835Af1DceA24C255149E0ad7C9FF1a5E8611Fa2); // doma-mainnet
        _registerDVN("Horizen", 30149, 0x33E5fcC13D7439cC62d54c41AA966197145b3Cd7); // dos-mainnet
        _registerDVN("Horizen", 30328, 0xf4672690eF45b46EAa3b688fe2f0Fc09e9366b20); // edu-mainnet
        _registerDVN("Horizen", 30391, 0x2afa3787cd95fee5D5753cd717EF228eb259f4ea); // ethereal-mainnet
        _registerDVN("Horizen", 30101, 0x380275805876Ff19055EA900CDb2B46a94ecF20D); // ethereum-mainnet
        _registerDVN("Horizen", 30292, 0xe7411fca6D67De2aA856570dBD59A19FCde81bD8); // etherlink-mainnet
        _registerDVN("Horizen", 30112, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // fantom-mainnet
        _registerDVN("Horizen", 30295, 0xeAA5a170d2588F84773f965281F8611D61312832); // flare-mainnet
        _registerDVN("Horizen", 30336, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // flow-mainnet
        _registerDVN("Horizen", 30255, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // fraxtal-mainnet
        _registerDVN("Horizen", 30138, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // fuse-mainnet
        _registerDVN("Horizen", 30389, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // gatelayer-mainnet
        _registerDVN("Horizen", 30342, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // glue-mainnet
        _registerDVN("Horizen", 30145, 0x6ABdb569Dc985504cCcB541ADE8445E5266e7388); // gnosis-mainnet
        _registerDVN("Horizen", 30361, 0xDF0771128BD4B9b18eD883d5Af41a6C725C51B38); // goat-mainnet
        _registerDVN("Horizen", 30294, 0xe95B63C4dA1D94fa5022e7C23c984F278B416ca7); // gravity-mainnet
        _registerDVN("Horizen", 30371, 0xFC977A4e88157B697417aDe965cEF0a2dfA39C70); // gunz-mainnet
        _registerDVN("Horizen", 30116, 0x462A63dBE8Ca43a57D379c88a382C02862B9A2cE); // harmony-mainnet
        _registerDVN("Horizen", 30316, 0xd0f50363E1aE33feAC8e0E067e42d0070C394525); // hedera-mainnet
        _registerDVN("Horizen", 30329, 0x38179D3BFa6Ef1d69A8A7b0b671BA3D8836B2AE8); // hemi-mainnet
        _registerDVN("Horizen", 30265, 0x97841D4AB18E9A923322A002d5b8Eb42b31Ccdb5); // homeverse-mainnet
        _registerDVN("Horizen", 30367, 0xBB83Ecf372CbB6daa629ea9A9A53BEC6d601F229); // hyperliquid-mainnet
        _registerDVN("Horizen", 30339, 0x395B14700812cccC38b8e64F0a06ce2045FE9bA3); // ink-mainnet
        _registerDVN("Horizen", 30284, 0xDFC9455F8F86b45Fa3b1116967f740905de6Fe51); // iota-mainnet
        _registerDVN("Horizen", 30330, 0xca848BcB059e33Adb260d793EE360924B6Aa8E86); // islander-mainnet
        _registerDVN("Horizen", 30285, 0xFb02364E3F5e97d8327dC6e4326E93828a28657d); // joc-mainnet
        _registerDVN("Horizen", 30375, 0x84a410A8a912e333B957680998a76e526f98e207); // katana-mainnet
        _registerDVN("Horizen", 30177, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // kava-mainnet
        _registerDVN("Horizen", 30150, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // klaytn-mainnet
        _registerDVN("Horizen", 30373, 0x27bB790440376dB53c840326263801FAFd9F0EE6); // lens-mainnet
        _registerDVN("Horizen", 30309, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // lightlink-mainnet
        _registerDVN("Horizen", 30321, 0x6c5f923B63Fdd52fb9C45dAeFA8695fA6b55a935); // lisk-mainnet
        _registerDVN("Horizen", 30197, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // loot-mainnet
        _registerDVN("Horizen", 30311, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // lyra-mainnet
        _registerDVN("Horizen", 30217, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // manta-mainnet
        _registerDVN("Horizen", 30181, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // mantle-mainnet
        _registerDVN("Horizen", 30198, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // meritcircle-mainnet
        _registerDVN("Horizen", 30266, 0x439264FB87581a70Bb6D7bEFd16b636521B0Ad2D); // merlin-mainnet
        _registerDVN("Horizen", 30176, 0x3F10b9B75b05f103995eE8B8E2803AA6c7A9dcDf); // meter-mainnet
        _registerDVN("Horizen", 30151, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // metis-mainnet
        _registerDVN("Horizen", 30260, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // mode-mainnet
        _registerDVN("Horizen", 30390, 0xdCdd4628F858b45260C31D6ad076bD2C3D3c2f73); // monad-mainnet
        _registerDVN("Horizen", 30126, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // moonbeam-mainnet
        _registerDVN("Horizen", 30167, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // moonriver-mainnet
        _registerDVN("Horizen", 30322, 0x6c5f923B63Fdd52fb9C45dAeFA8695fA6b55a935); // morph-mainnet
        _registerDVN("Horizen", 30331, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // mp1-mainnet
        _registerDVN("Horizen", 30369, 0xf0809F6e760a5452Ee567975EdA7a28dA4a83D38); // nibiru-mainnet
        _registerDVN("Horizen", 30175, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // nova-mainnet
        _registerDVN("Horizen", 30155, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // okx-mainnet
        _registerDVN("Horizen", 30202, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // opbnb-mainnet
        _registerDVN("Horizen", 30392, 0xCee801c12814a7C5b8d792098f624fb3D7aD8651); // openledger-mainnet
        _registerDVN("Horizen", 30111, 0x9E930731cb4A6bf7eCc11F695A295c60bDd212eB); // optimism-mainnet
        _registerDVN("Horizen", 30213, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // orderly-mainnet
        _registerDVN("Horizen", 30302, 0x21cAF0BCE846AAA78C9f23C5A4eC5988EcBf9988); // peaq-mainnet
        _registerDVN("Horizen", 30383, 0xd4CE45957FBCb88b868ad2c759C7DB9BC2741e56); // plasma-mainnet
        _registerDVN("Horizen", 30370, 0x5E0744f8FBF26446c683BcF4cD405ad56bA76F8A); // plumephoenix-mainnet
        _registerDVN("Horizen", 30109, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // polygon-mainnet
        _registerDVN("Horizen", 30235, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // rarible-mainnet
        _registerDVN("Horizen", 30313, 0x6c5f923B63Fdd52fb9C45dAeFA8695fA6b55a935); // reya-mainnet
        _registerDVN("Horizen", 30333, 0x54dD79f5cE72b51FCBbcb170Dd01E32034323565); // rootstock-mainnet
        _registerDVN("Horizen", 30278, 0x5fddD320a1e29bB466Fa635661b125D51D976f92); // sanko-mainnet
        _registerDVN("Horizen", 30214, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // scroll-mainnet
        _registerDVN("Horizen", 30280, 0x87048402c32632B7c4d0A892d82bC1160E8B2393); // sei-mainnet
        _registerDVN("Horizen", 30230, 0xa59BA433ac34D2927232918Ef5B2eaAfcF130BA5); // shimmer-mainnet
        _registerDVN("Horizen", 30148, 0xabC9b1819cc4D9846550F928B985993cF6240439); // shrapnel-mainnet
        _registerDVN("Horizen", 30380, 0x5fddD320a1e29bB466Fa635661b125D51D976f92); // somnia-mainnet
        _registerDVN("Horizen", 30340, 0x8Fc629aa400D4D9c0B118F2685a49316552ABf27); // soneium-mainnet
        _registerDVN("Horizen", 30332, 0x54dD79f5cE72b51FCBbcb170Dd01E32034323565); // sonic-mainnet
        _registerDVN("Horizen", 30334, 0xCec9f0A49073ac4a1C439D06cb9448512389a64E); // sophon-mainnet
        _registerDVN("Horizen", 30341, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // space-mainnet
        _registerDVN("Horizen", 30396, 0x965A80Dc87cec5848310E612DeAD84B543AeF874); // stable-mainnet
        _registerDVN("Horizen", 30364, 0x73DDc92E39aEdA95FEb8D3E0008016d9F1268c76); // story-mainnet
        _registerDVN("Horizen", 30374, 0xb3d5Fd1f98510e90bd59BD702eD362622672b97f); // subtensorevm-mainnet
        _registerDVN("Horizen", 30327, 0x38179D3BFa6Ef1d69A8A7b0b671BA3D8836B2AE8); // superposition-mainnet
        _registerDVN("Horizen", 30335, 0xf4672690eF45b46EAa3b688fe2f0Fc09e9366b20); // swell-mainnet
        _registerDVN("Horizen", 30377, 0xB19A9370D404308040A9760678c8Ca28aFfbbb76); // tac-mainnet
        _registerDVN("Horizen", 30290, 0xBD237eF21319E2200487BDF30c188C6c34b16D3B); // taiko-mainnet
        _registerDVN("Horizen", 30199, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // telos-mainnet
        _registerDVN("Horizen", 30173, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // tenet-mainnet
        _registerDVN("Horizen", 30238, 0x0165C910Ea47964a23DC4FB7c7483F6f3ad462aE); // tiltyard-mainnet
        _registerDVN("Horizen", 30196, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // tomo-mainnet
        _registerDVN("Horizen", 30320, 0xC6a6324932B399D6A673B7Ed0af671F28033E046); // unichain-mainnet
        _registerDVN("Horizen", 30319, 0x95729Ea44326f8adD8A9b1d987279DBdC1DD3dFf); // worldchain-mainnet
        _registerDVN("Horizen", 30236, 0xE94aE34DfCC87A61836938641444080B98402c75); // xai-mainnet
        _registerDVN("Horizen", 30291, 0x0E5c792Ec122cBE89cE0085D7EFcdB151eae3376); // xchain-mainnet
        _registerDVN("Horizen", 30365, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // xdc-mainnet
        _registerDVN("Horizen", 30274, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // xlayer-mainnet
        _registerDVN("Horizen", 30216, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // xpla-mainnet
        _registerDVN("Horizen", 30303, 0xdCdd4628F858b45260C31D6ad076bD2C3D3c2f73); // zircuit-mainnet
        _registerDVN("Horizen", 30183, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // zkconsensys-mainnet
        _registerDVN("Horizen", 30301, 0x27bB790440376dB53c840326263801FAFd9F0EE6); // zklink-mainnet
        _registerDVN("Horizen", 30158, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // zkpolygon-mainnet
        _registerDVN("Horizen", 30165, 0x1253E268Bc04bB43CB96D2F7Ee858b8A1433Cf6D); // zksync-mainnet
        _registerDVN("Horizen", 30386, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // zkverify-mainnet
        _registerDVN("Horizen", 30195, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // zora-mainnet

        // Lagrange
        _registerDVN("Lagrange", 30110, 0x021e401C2a1A60618c5E6353A40524971Eba1E8D); // arbitrum-mainnet
        _registerDVN("Lagrange", 30184, 0xC50a49186aA80427aA3b0d3C2Cec19BA64222A29); // base-mainnet
        _registerDVN("Lagrange", 30101, 0x95729Ea44326f8adD8A9b1d987279DBdC1DD3dFf); // ethereum-mainnet
        _registerDVN("Lagrange", 30111, 0xA4281c1c88F0278FF696eDeb517052153190FC9E); // optimism-mainnet

        // LayerZero Labs
        _registerDVN("LayerZero Labs", 30324, 0xF4DA94b4EE9D8e209e3bf9f469221CE2731A7112); // abstract-mainnet
        _registerDVN("LayerZero Labs", 30372, 0x282b3386571f7f794450d5789911a9804FA346b4); // animechain-mainnet
        _registerDVN("LayerZero Labs", 30312, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // ape-mainnet
        _registerDVN("LayerZero Labs", 30384, 0x282b3386571f7f794450d5789911a9804FA346b4); // apexfusionnexus-mainnet
        _registerDVN("LayerZero Labs", 30110, 0x2f55C492897526677C5B68fb199ea31E2c126416); // arbitrum-mainnet
        _registerDVN("LayerZero Labs", 30210, 0xE1975c47779EdAaABa31F64934A33Affd3CE15c2); // astar-mainnet
        _registerDVN("LayerZero Labs", 30211, 0xD4a903930f2c9085586cda0b11D9681EECb20D2f); // aurora-mainnet
        _registerDVN("LayerZero Labs", 30106, 0x962F502A63F5FBeB44DC9ab932122648E8352959); // avalanche-mainnet
        _registerDVN("LayerZero Labs", 30363, 0x282b3386571f7f794450d5789911a9804FA346b4); // bahamut-mainnet
        _registerDVN("LayerZero Labs", 30184, 0x9e059a54699a285714207b43B055483E78FAac25); // base-mainnet
        _registerDVN("LayerZero Labs", 30234, 0xb21f945e8917c6Cd69FcFE66ac6703B90f7fe004); // bb1-mainnet
        _registerDVN("LayerZero Labs", 30362, 0x282b3386571f7f794450d5789911a9804FA346b4); // bera-mainnet
        _registerDVN("LayerZero Labs", 30317, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // bevm-mainnet
        _registerDVN("LayerZero Labs", 30314, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // bitlayer-mainnet
        _registerDVN("LayerZero Labs", 30243, 0xc097ab8CD7b053326DFe9fB3E3a31a0CCe3B526f); // blast-mainnet
        _registerDVN("LayerZero Labs", 30279, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // bob-mainnet
        _registerDVN("LayerZero Labs", 30376, 0x282b3386571f7f794450d5789911a9804FA346b4); // botanix-mainnet
        _registerDVN("LayerZero Labs", 30293, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // bouncebit-mainnet
        _registerDVN("LayerZero Labs", 30102, 0xfD6865c841c2d64565562fCc7e05e619A30615f0); // bsc-mainnet
        _registerDVN("LayerZero Labs", 30381, 0x15e51701F245F6D5bd0FEE87bCAf55B0841451B3); // camp-mainnet
        _registerDVN("LayerZero Labs", 30159, 0x1BAcC2205312534375c8d1801C27D28370656cFf); // canto-mainnet
        _registerDVN("LayerZero Labs", 30125, 0x75b073994560A5c03Cd970414d9170be0C6e5c36); // celo-mainnet
        _registerDVN("LayerZero Labs", 30323, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // codex-mainnet
        _registerDVN("LayerZero Labs", 30366, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // concrete-mainnet
        _registerDVN("LayerZero Labs", 30212, 0x8D183A062e99cad6f3723E6d836F9EA13886B173); // conflux-mainnet
        _registerDVN("LayerZero Labs", 30400, 0x282b3386571f7f794450d5789911a9804FA346b4); // converge-mainnet
        _registerDVN("LayerZero Labs", 30153, 0x3C5575898f59c097681d1Fc239c2c6Ad36B7b41c); // coredao-mainnet
        _registerDVN("LayerZero Labs", 30359, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // cronosevm-mainnet
        _registerDVN("LayerZero Labs", 30360, 0x07fD0e370B49919cA8dA0CE842B8177263c0E12c); // cronoszkevm-mainnet
        _registerDVN("LayerZero Labs", 30283, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // cyber-mainnet
        _registerDVN("LayerZero Labs", 30267, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // degen-mainnet
        _registerDVN("LayerZero Labs", 30118, 0xB98D764D25d53F803f05d451225612e4A9A3b712); // dexalot-mainnet
        _registerDVN("LayerZero Labs", 30115, 0x1F7E674143031e74bc48a0c570c174A07aA9C5d0); // dfk-mainnet
        _registerDVN("LayerZero Labs", 30385, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // dinari-mainnet
        _registerDVN("LayerZero Labs", 30393, 0x282b3386571f7f794450d5789911a9804FA346b4); // doma-mainnet
        _registerDVN("LayerZero Labs", 30149, 0x203DFa8CBcbe234821DA01a6e95Fcbf92dA065EA); // dos-mainnet
        _registerDVN("LayerZero Labs", 30328, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // edu-mainnet
        _registerDVN("LayerZero Labs", 30391, 0x282b3386571f7f794450d5789911a9804FA346b4); // ethereal-mainnet
        _registerDVN("LayerZero Labs", 30101, 0x589dEDbD617e0CBcB916A9223F4d1300c294236b); // ethereum-mainnet
        _registerDVN("LayerZero Labs", 30292, 0xc097ab8CD7b053326DFe9fB3E3a31a0CCe3B526f); // etherlink-mainnet
        _registerDVN("LayerZero Labs", 30112, 0xE60A3959Ca23a92BF5aAf992EF837cA7F828628a); // fantom-mainnet
        _registerDVN("LayerZero Labs", 30295, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // flare-mainnet
        _registerDVN("LayerZero Labs", 30336, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // flow-mainnet
        _registerDVN("LayerZero Labs", 30255, 0xcCE466a522984415bC91338c232d98869193D46e); // fraxtal-mainnet
        _registerDVN("LayerZero Labs", 30138, 0x795F8325aF292Ff6E58249361d1954893BE15Aff); // fuse-mainnet
        _registerDVN("LayerZero Labs", 30389, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // gatelayer-mainnet
        _registerDVN("LayerZero Labs", 30342, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // glue-mainnet
        _registerDVN("LayerZero Labs", 30145, 0x11bb2991882a86Dc3E38858d922559A385d506bA); // gnosis-mainnet
        _registerDVN("LayerZero Labs", 30361, 0x282b3386571f7f794450d5789911a9804FA346b4); // goat-mainnet
        _registerDVN("LayerZero Labs", 30294, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // gravity-mainnet
        _registerDVN("LayerZero Labs", 30371, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // gunz-mainnet
        _registerDVN("LayerZero Labs", 30116, 0x8363302080e711E0CAb978C081b9e69308d49808); // harmony-mainnet
        _registerDVN("LayerZero Labs", 30316, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // hedera-mainnet
        _registerDVN("LayerZero Labs", 30329, 0x282b3386571f7f794450d5789911a9804FA346b4); // hemi-mainnet
        _registerDVN("LayerZero Labs", 30265, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // homeverse-mainnet
        _registerDVN("LayerZero Labs", 30382, 0x7cacBe439EaD55fa1c22790330b12835c6884a91); // humanity-mainnet
        _registerDVN("LayerZero Labs", 30367, 0xc097ab8CD7b053326DFe9fB3E3a31a0CCe3B526f); // hyperliquid-mainnet
        _registerDVN("LayerZero Labs", 30394, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // injectiveevm-mainnet
        _registerDVN("LayerZero Labs", 30339, 0x174F2bA26f8ADeAfA82663bcf908288d5DbCa649); // ink-mainnet
        _registerDVN("LayerZero Labs", 30284, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // iota-mainnet
        _registerDVN("LayerZero Labs", 30330, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // islander-mainnet
        _registerDVN("LayerZero Labs", 30285, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // joc-mainnet
        _registerDVN("LayerZero Labs", 30375, 0x282b3386571f7f794450d5789911a9804FA346b4); // katana-mainnet
        _registerDVN("LayerZero Labs", 30177, 0x2D40A7B66F776345Cf763c8EBB83199Cd285e7a3); // kava-mainnet
        _registerDVN("LayerZero Labs", 30150, 0xc80233AD8251E668BecbC3B0415707fC7075501e); // klaytn-mainnet
        _registerDVN("LayerZero Labs", 30373, 0x07fD0e370B49919cA8dA0CE842B8177263c0E12c); // lens-mainnet
        _registerDVN("LayerZero Labs", 30309, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // lightlink-mainnet
        _registerDVN("LayerZero Labs", 30321, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // lisk-mainnet
        _registerDVN("LayerZero Labs", 30197, 0x4f8B7a7a346Da5c467085377796e91220d904c15); // loot-mainnet
        _registerDVN("LayerZero Labs", 30311, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // lyra-mainnet
        _registerDVN("LayerZero Labs", 30217, 0xA09dB5142654e3eB5Cf547D66833FAe7097B21C3); // manta-mainnet
        _registerDVN("LayerZero Labs", 30181, 0x28B6140ead70cb2Fb669705b3598ffB4BEaA060b); // mantle-mainnet
        _registerDVN("LayerZero Labs", 30198, 0x5E38c31C28d0F485d6dc3fFAbF8980bBCD882294); // meritcircle-mainnet
        _registerDVN("LayerZero Labs", 30266, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // merlin-mainnet
        _registerDVN("LayerZero Labs", 30176, 0xB792aFc44214B5f910216Bc904633dbD15b31680); // meter-mainnet
        _registerDVN("LayerZero Labs", 30151, 0x32d4F92437454829b3Fe7BEBfeCE5D0523DEb475); // metis-mainnet
        _registerDVN("LayerZero Labs", 30260, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // mode-mainnet
        _registerDVN("LayerZero Labs", 30390, 0x282b3386571f7f794450d5789911a9804FA346b4); // monad-mainnet
        _registerDVN("LayerZero Labs", 30126, 0x8B9b67b22ab2ed6Ee324C2fd43734dBd2dDDD045); // moonbeam-mainnet
        _registerDVN("LayerZero Labs", 30167, 0x2b3eBE6662Ad402317EE7Ef4e6B25c79a0f91015); // moonriver-mainnet
        _registerDVN("LayerZero Labs", 30322, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // morph-mainnet
        _registerDVN("LayerZero Labs", 30331, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // mp1-mainnet
        _registerDVN("LayerZero Labs", 30395, 0x282b3386571f7f794450d5789911a9804FA346b4); // nexera-mainnet
        _registerDVN("LayerZero Labs", 30369, 0x5727E81A40015961145330D91cC27b5E189fF3e1); // nibiru-mainnet
        _registerDVN("LayerZero Labs", 30175, 0xb7e97ad5661134185Fe608b2A31fe8cEf2147Ba9); // nova-mainnet
        _registerDVN("LayerZero Labs", 30388, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // og-mainnet
        _registerDVN("LayerZero Labs", 30155, 0x52EEA5c490fB89c7A0084B32FEAB854eefF07c82); // okx-mainnet
        _registerDVN("LayerZero Labs", 30202, 0x3eBb618B5c9d09DE770979D552b27D6357Aff73B); // opbnb-mainnet
        _registerDVN("LayerZero Labs", 30392, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // openledger-mainnet
        _registerDVN("LayerZero Labs", 30111, 0x6A02D83e8d433304bba74EF1c427913958187142); // optimism-mainnet
        _registerDVN("LayerZero Labs", 30213, 0xF53857dbc0D2c59D5666006EC200cbA2936B8c35); // orderly-mainnet
        _registerDVN("LayerZero Labs", 30302, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // peaq-mainnet
        _registerDVN("LayerZero Labs", 30383, 0x282b3386571f7f794450d5789911a9804FA346b4); // plasma-mainnet
        _registerDVN("LayerZero Labs", 30370, 0x4208D6E27538189bB48E603D6123A94b8Abe0A0b); // plumephoenix-mainnet
        _registerDVN("LayerZero Labs", 30109, 0x23DE2FE932d9043291f870324B74F820e11dc81A); // polygon-mainnet
        _registerDVN("LayerZero Labs", 30235, 0x0b5E5452d0c9DA1Bb5fB0664F48313e9667d7820); // rarible-mainnet
        _registerDVN("LayerZero Labs", 30313, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // reya-mainnet
        _registerDVN("LayerZero Labs", 30333, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // rootstock-mainnet
        _registerDVN("LayerZero Labs", 30278, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // sanko-mainnet
        _registerDVN("LayerZero Labs", 30214, 0xbe0d08a85EeBFCC6eDA0A843521f7CBB1180D2e2); // scroll-mainnet
        _registerDVN("LayerZero Labs", 30280, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // sei-mainnet
        _registerDVN("LayerZero Labs", 30230, 0x9Bdf3aE7E2e3D211811E5e782a808Ca0a75BF1Fc); // shimmer-mainnet
        _registerDVN("LayerZero Labs", 30379, 0x282b3386571f7f794450d5789911a9804FA346b4); // silicon-mainnet
        _registerDVN("LayerZero Labs", 30273, 0xce8358bc28dd8296Ce8cAF1CD2b44787abd65887); // skale-mainnet
        _registerDVN("LayerZero Labs", 30380, 0x282b3386571f7f794450d5789911a9804FA346b4); // somnia-mainnet
        _registerDVN("LayerZero Labs", 30340, 0xfDfA2330713A8e2EaC6e4f15918F11937fFA4dBE); // soneium-mainnet
        _registerDVN("LayerZero Labs", 30332, 0x282b3386571f7f794450d5789911a9804FA346b4); // sonic-mainnet
        _registerDVN("LayerZero Labs", 30334, 0x07fD0e370B49919cA8dA0CE842B8177263c0E12c); // sophon-mainnet
        _registerDVN("LayerZero Labs", 30341, 0x7cacBe439EaD55fa1c22790330b12835c6884a91); // space-mainnet
        _registerDVN("LayerZero Labs", 30396, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // stable-mainnet
        _registerDVN("LayerZero Labs", 30364, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // story-mainnet
        _registerDVN("LayerZero Labs", 30374, 0x282b3386571f7f794450d5789911a9804FA346b4); // subtensorevm-mainnet
        _registerDVN("LayerZero Labs", 30327, 0x282b3386571f7f794450d5789911a9804FA346b4); // superposition-mainnet
        _registerDVN("LayerZero Labs", 30335, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // swell-mainnet
        _registerDVN("LayerZero Labs", 30377, 0x282b3386571f7f794450d5789911a9804FA346b4); // tac-mainnet
        _registerDVN("LayerZero Labs", 30290, 0xc097ab8CD7b053326DFe9fB3E3a31a0CCe3B526f); // taiko-mainnet
        _registerDVN("LayerZero Labs", 30199, 0x3C5575898f59c097681d1Fc239c2c6Ad36B7b41c); // telos-mainnet
        _registerDVN("LayerZero Labs", 30173, 0x28A5536cA9F36c45A9d2AC8d2B62Fc46Fde024B6); // tenet-mainnet
        _registerDVN("LayerZero Labs", 30238, 0xCFc3f9dD0205b76FF04e20243f106465Dd829656); // tiltyard-mainnet
        _registerDVN("LayerZero Labs", 30196, 0x1aCe9DD1BC743aD036eF2D92Af42Ca70A1159df5); // tomo-mainnet
        _registerDVN("LayerZero Labs", 30320, 0x282b3386571f7f794450d5789911a9804FA346b4); // unichain-mainnet
        _registerDVN("LayerZero Labs", 30319, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // worldchain-mainnet
        _registerDVN("LayerZero Labs", 30236, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // xai-mainnet
        _registerDVN("LayerZero Labs", 30291, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // xchain-mainnet
        _registerDVN("LayerZero Labs", 30365, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // xdc-mainnet
        _registerDVN("LayerZero Labs", 30274, 0x9C061c9A4782294eeF65ef28Cb88233A987F4bdD); // xlayer-mainnet
        _registerDVN("LayerZero Labs", 30216, 0x2d24207F9C1F77B2E08F2C3aD430da18e355CF66); // xpla-mainnet
        _registerDVN("LayerZero Labs", 30303, 0x6788f52439ACA6BFF597d3eeC2DC9a44B8FEE842); // zircuit-mainnet
        _registerDVN("LayerZero Labs", 30183, 0x129Ee430Cb2Ff2708CCADDBDb408a88Fe4FFd480); // zkconsensys-mainnet
        _registerDVN("LayerZero Labs", 30301, 0x04830f6deCF08Dec9eD6C3fCAD215245B78A59e1); // zklink-mainnet
        _registerDVN("LayerZero Labs", 30158, 0x488863D609F3A673875a914fBeE7508a1DE45eC6); // zkpolygon-mainnet
        _registerDVN("LayerZero Labs", 30165, 0x620A9DF73D2F1015eA75aea1067227F9013f5C51); // zksync-mainnet
        _registerDVN("LayerZero Labs", 30386, 0x282b3386571f7f794450d5789911a9804FA346b4); // zkverify-mainnet
        _registerDVN("LayerZero Labs", 30195, 0xC1EC25A9e8a8DE5Aa346f635B33e5B74c4c081aF); // zora-mainnet

        // Luganodes
        _registerDVN("Luganodes", 30324, 0x022dA66B230da7EFdEEd802DFC77EE8dD258E2C8); // abstract-mainnet
        _registerDVN("Luganodes", 30110, 0x54dD79f5cE72b51FCBbcb170Dd01E32034323565); // arbitrum-mainnet
        _registerDVN("Luganodes", 30106, 0xE4193136B92bA91402313e95347c8e9FAD8d27d0); // avalanche-mainnet
        _registerDVN("Luganodes", 30184, 0xa0AF56164F02bDf9d75287ee77c568889F11d5f2); // base-mainnet
        _registerDVN("Luganodes", 30362, 0xFd4d23EB5CeA65f6CC7eEC8f3b394e55AED68299); // bera-mainnet
        _registerDVN("Luganodes", 30102, 0x2c7185f5B0976397d9eB5c19d639d4005e6708f0); // bsc-mainnet
        _registerDVN("Luganodes", 30125, 0x82F6Ad698f3116Ca1B71836A7f1303628FA855DB); // celo-mainnet
        _registerDVN("Luganodes", 30101, 0x58249a2Ec05c1978bF21DF1f5eC1847e42455CF4); // ethereum-mainnet
        _registerDVN("Luganodes", 30112, 0xa6F5DDBF0Bd4D03334523465439D301080574742); // fantom-mainnet
        _registerDVN("Luganodes", 30145, 0x7cC59B5062A8291804A21a2a793c6Ce9ea2f0Eb9); // gnosis-mainnet
        _registerDVN("Luganodes", 30367, 0x9e451905f65eF78D62b93DAc3513486da8429d0a); // hyperliquid-mainnet
        _registerDVN("Luganodes", 30339, 0x0ad6c9Eb13e373154bFB303561b979BAc5FA2302); // ink-mainnet
        _registerDVN("Luganodes", 30181, 0x315b0e76A510607bB0F706B17716F426D5b385b8); // mantle-mainnet
        _registerDVN("Luganodes", 30388, 0xE6655528dbB0f7d1407264aA878A5B5363B8752c); // og-mainnet
        _registerDVN("Luganodes", 30111, 0xd841A741Addcb6Dea735D3B8C9Faf96BA3f3d30D); // optimism-mainnet
        _registerDVN("Luganodes", 30109, 0xD1b5493e712081A6FBAb73116405590046668F6b); // polygon-mainnet
        _registerDVN("Luganodes", 30214, 0xf60C89799C85D8FaB79519f7666dcDe2A7C97CCA); // scroll-mainnet
        _registerDVN("Luganodes", 30280, 0x6E01Aa282f058873d28055e07d85f4197E8Db261); // sei-mainnet
        _registerDVN("Luganodes", 30380, 0x5488a4ca201421cF100dC1B90D1dE5B26b421f64); // somnia-mainnet
        _registerDVN("Luganodes", 30332, 0xC8B7744AFd77C3EEcf310383837A07584766A51a); // sonic-mainnet
        _registerDVN("Luganodes", 30377, 0x58249a2Ec05c1978bF21DF1f5eC1847e42455CF4); // tac-mainnet
        _registerDVN("Luganodes", 30320, 0xF02D0F9ACc2870e12C34aa3816dd86FaC1339f38); // unichain-mainnet
        _registerDVN("Luganodes", 30183, 0x08670E326968d18D4fe359080b8E3eeeA552E867); // zkconsensys-mainnet
        _registerDVN("Luganodes", 30195, 0x9FE36b305120556dbeefab58d58877D87b553DF5); // zora-mainnet

        // MIM
        _registerDVN("MIM", 30110, 0x9E930731cb4A6bf7eCc11F695A295c60bDd212eB); // arbitrum-mainnet
        _registerDVN("MIM", 30106, 0xF45742BbfaBCEe739eA2a2d0BA2dd140F1f2C6A3); // avalanche-mainnet
        _registerDVN("MIM", 30362, 0x73DDc92E39aEdA95FEb8D3E0008016d9F1268c76); // bera-mainnet
        _registerDVN("MIM", 30102, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // bsc-mainnet
        _registerDVN("MIM", 30101, 0x0Ae4e6a9a8B01EE22c6A49aF22B674A4E033A23D); // ethereum-mainnet
        _registerDVN("MIM", 30112, 0x1bab20E7FDc79257729CB596BEF85db76C44915E); // fantom-mainnet
        _registerDVN("MIM", 30367, 0x32B29d6B6cd4A851548A6E888Cc25E39E8a16d86); // hyperliquid-mainnet
        _registerDVN("MIM", 30369, 0x53Fa9f0bd34F3f0e80580d1c93168F56c9555cA4); // nibiru-mainnet
        _registerDVN("MIM", 30111, 0xD954bF7968eF68875c9100c9ec890f969504d120); // optimism-mainnet
        _registerDVN("MIM", 30109, 0x1bab20E7FDc79257729CB596BEF85db76C44915E); // polygon-mainnet

        // Mantle Bank
        _registerDVN("Mantle Bank", 30110, 0x50fF206140CadADA2d9d510F1A184Be9221d86cF); // arbitrum-mainnet
        _registerDVN("Mantle Bank", 30184, 0x761bC869351293c5572Ed5581E23e7D5D9C6D3d1); // base-mainnet
        _registerDVN("Mantle Bank", 30362, 0x88a8b858c7fCB3Fe0052c9b7bcC69183a9cebD76); // bera-mainnet
        _registerDVN("Mantle Bank", 30101, 0x868e6393AEa25E8c7e58BC5d4c90a5237C573ff6); // ethereum-mainnet
        _registerDVN("Mantle Bank", 30181, 0xB797053fBA3D41f5067C4BD74dc334328395C4d2); // mantle-mainnet

        // Mantle01
        _registerDVN("Mantle01", 30101, 0x7cC59B5062A8291804A21a2a793c6Ce9ea2f0Eb9); // ethereum-mainnet
        _registerDVN("Mantle01", 30181, 0x0D7cb4033e0C193F65B3639E61b6986A29Bf1DD4); // mantle-mainnet

        // Mantle02
        _registerDVN("Mantle02", 30101, 0xdd907f5aF01A829Cd65C99A132E8720d3e990D02); // ethereum-mainnet
        _registerDVN("Mantle02", 30181, 0x5a4c666E9C7aA86FD4fBFDFbfd04646DcC45C6c5); // mantle-mainnet

        // Mantle03
        _registerDVN("Mantle03", 30102, 0xEc65a0245c19A084650cB5B85FD1a2Bc7B0FD452); // bsc-mainnet
        _registerDVN("Mantle03", 30101, 0x554833698Ae0FB22ECC90B01222903fD62CA4B47); // ethereum-mainnet
        _registerDVN("Mantle03", 30367, 0xbBD228Fa47A8E80FbBfB778Abc56031Fa2C038ce); // hyperliquid-mainnet
        _registerDVN("Mantle03", 30181, 0x78203678D264063815Dac114eA810E9837Cd80f7); // mantle-mainnet

        // Muon
        _registerDVN("Muon", 30110, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // arbitrum-mainnet
        _registerDVN("Muon", 30106, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // avalanche-mainnet
        _registerDVN("Muon", 30184, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // base-mainnet
        _registerDVN("Muon", 30102, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // bsc-mainnet
        _registerDVN("Muon", 30101, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // ethereum-mainnet
        _registerDVN("Muon", 30112, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // fantom-mainnet
        _registerDVN("Muon", 30111, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // optimism-mainnet
        _registerDVN("Muon", 30109, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // polygon-mainnet
        _registerDVN("Muon", 30332, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // sonic-mainnet
        _registerDVN("Muon", 30183, 0xA3858e2A9860C935Fc9586a617e9b2A674C3e4c8); // zkconsensys-mainnet

        // Mysten Labs
        _registerDVN("Mysten Labs", 30101, 0x3D68029E411B181FEfA1a8BA60aaf27dbe68636C); // ethereum-mainnet

        // Nansen
        _registerDVN("Nansen", 30110, 0x01F9aAD7c53626bF8807d640D9Ddf852254D6f63); // arbitrum-mainnet
        _registerDVN("Nansen", 30106, 0xc816Afa2f1C4Ab615fE735270D1831fa7D067D15); // avalanche-mainnet
        _registerDVN("Nansen", 30184, 0x93aC538152E1BC4F093aE5666Ee9FD1d84f4f4bF); // base-mainnet
        _registerDVN("Nansen", 30362, 0x00A979a5D306E9c5F8Cf473659e75f8002E06fc8); // bera-mainnet
        _registerDVN("Nansen", 30102, 0x534C6b3e6805E9287ff1D49C349d5f7a01B9b7F5); // bsc-mainnet
        _registerDVN("Nansen", 30101, 0x3a4636E9AB975d28d3Af808b4e1c9fd936374E30); // ethereum-mainnet
        _registerDVN("Nansen", 30112, 0x57555Da46d20F39bC6795BCD6fF50cE425A0E5aF); // fantom-mainnet
        _registerDVN("Nansen", 30145, 0x13feb7234Ff60A97af04477d6421415766753Ba3); // gnosis-mainnet
        _registerDVN("Nansen", 30367, 0xcFe987eBFf7612B53D145DD70EE24D00E12d6A1F); // hyperliquid-mainnet
        _registerDVN("Nansen", 30339, 0x3a4636E9AB975d28d3Af808b4e1c9fd936374E30); // ink-mainnet
        _registerDVN("Nansen", 30181, 0x58620C352dd33EaaA2f6513877515453e20e8656); // mantle-mainnet
        _registerDVN("Nansen", 30111, 0x3b0531eB02Ab4aD72e7a531180beeF9493a00dD2); // optimism-mainnet
        _registerDVN("Nansen", 30302, 0xc2A0C36f5939A14966705c7Cec813163FaEEa1F0); // peaq-mainnet
        _registerDVN("Nansen", 30109, 0x0a8618F71dB88AB5D0CAF0610Ede19F0AB8817c5); // polygon-mainnet
        _registerDVN("Nansen", 30280, 0xf85F51c1d5b4de2446d99b104acFca7Ff63Bd3AD); // sei-mainnet
        _registerDVN("Nansen", 30332, 0x64D684840881b45869B0C72B17aa911A3FC4305e); // sonic-mainnet
        _registerDVN("Nansen", 30320, 0x144c6a7A17781e165f430b18f0680c5b3e3713E2); // unichain-mainnet

        // Nethermind
        _registerDVN("Nethermind", 30324, 0xc4A1F52fDA034A9A5E1B3b27D14451d15776Fef6); // abstract-mainnet
        _registerDVN("Nethermind", 30372, 0x9E0E95Ede70F680f74480b510FF9f45C70e3da80); // animechain-mainnet
        _registerDVN("Nethermind", 30312, 0x07653d28b0f53D4c54b70eb1f9025795B23a9D6e); // ape-mainnet
        _registerDVN("Nethermind", 30384, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // apexfusionnexus-mainnet
        _registerDVN("Nethermind", 30110, 0xa7b5189bcA84Cd304D8553977c7C614329750d99); // arbitrum-mainnet
        _registerDVN("Nethermind", 30210, 0xB19A9370D404308040A9760678c8Ca28aFfbbb76); // astar-mainnet
        _registerDVN("Nethermind", 30211, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // aurora-mainnet
        _registerDVN("Nethermind", 30106, 0xa59BA433ac34D2927232918Ef5B2eaAfcF130BA5); // avalanche-mainnet
        _registerDVN("Nethermind", 30184, 0xcd37CA043f8479064e10635020c65FfC005d36f6); // base-mainnet
        _registerDVN("Nethermind", 30234, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bb1-mainnet
        _registerDVN("Nethermind", 30362, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bera-mainnet
        _registerDVN("Nethermind", 30314, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bitlayer-mainnet
        _registerDVN("Nethermind", 30243, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // blast-mainnet
        _registerDVN("Nethermind", 30279, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // bob-mainnet
        _registerDVN("Nethermind", 30376, 0xA4281c1c88F0278FF696eDeb517052153190FC9E); // botanix-mainnet
        _registerDVN("Nethermind", 30102, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // bsc-mainnet
        _registerDVN("Nethermind", 30381, 0x2F29D3d12fc2d1961Ad8B5397c0f878003c35e20); // camp-mainnet
        _registerDVN("Nethermind", 30159, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // canto-mainnet
        _registerDVN("Nethermind", 30125, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // celo-mainnet
        _registerDVN("Nethermind", 30323, 0xabC9b1819cc4D9846550F928B985993cF6240439); // codex-mainnet
        _registerDVN("Nethermind", 30366, 0x047d9DBe4fC6B5c916F37237F547f9F42809935a); // concrete-mainnet
        _registerDVN("Nethermind", 30212, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // conflux-mainnet
        _registerDVN("Nethermind", 30153, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // coredao-mainnet
        _registerDVN("Nethermind", 30359, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // cronosevm-mainnet
        _registerDVN("Nethermind", 30360, 0x3A5a74f863ec48c1769C4Ee85f6C3d70f5655E2A); // cronoszkevm-mainnet
        _registerDVN("Nethermind", 30283, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // cyber-mainnet
        _registerDVN("Nethermind", 30267, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // degen-mainnet
        _registerDVN("Nethermind", 30118, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // dexalot-mainnet
        _registerDVN("Nethermind", 30115, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // dfk-mainnet
        _registerDVN("Nethermind", 30393, 0xabC9b1819cc4D9846550F928B985993cF6240439); // doma-mainnet
        _registerDVN("Nethermind", 30149, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // dos-mainnet
        _registerDVN("Nethermind", 30328, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // edu-mainnet
        _registerDVN("Nethermind", 30391, 0x6D4fc4bD9f9C29086e2Aa67d4C81F32D2E0F285c); // ethereal-mainnet
        _registerDVN("Nethermind", 30101, 0xa59BA433ac34D2927232918Ef5B2eaAfcF130BA5); // ethereum-mainnet
        _registerDVN("Nethermind", 30292, 0x7a23612F07d81F16B26cF0b5a4C3eca0E8668df2); // etherlink-mainnet
        _registerDVN("Nethermind", 30112, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // fantom-mainnet
        _registerDVN("Nethermind", 30295, 0x9bCd17A654bffAa6f8fEa38D19661a7210e22196); // flare-mainnet
        _registerDVN("Nethermind", 30336, 0x3C61aAd6D402D867c653F603558F4b8f91AbE952); // flow-mainnet
        _registerDVN("Nethermind", 30255, 0xa7b5189bcA84Cd304D8553977c7C614329750d99); // fraxtal-mainnet
        _registerDVN("Nethermind", 30138, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // fuse-mainnet
        _registerDVN("Nethermind", 30342, 0xaA3099F91912E07976c2DD1598DC740d81BD3FeA); // glue-mainnet
        _registerDVN("Nethermind", 30145, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // gnosis-mainnet
        _registerDVN("Nethermind", 30361, 0xe6CD8c2E46Ef396DF88048449e5B1C75172b40C3); // goat-mainnet
        _registerDVN("Nethermind", 30294, 0x4b92BC2A7d681bf5230472C80d92aCFE9A6b9435); // gravity-mainnet
        _registerDVN("Nethermind", 30371, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // gunz-mainnet
        _registerDVN("Nethermind", 30116, 0xD24972c11F91c1bB9eaEe97ec96bB9c33cF7af24); // harmony-mainnet
        _registerDVN("Nethermind", 30316, 0xeCc3Dc1Cc45B1934CE713F8fb0d3D3852C01a5c1); // hedera-mainnet
        _registerDVN("Nethermind", 30329, 0x07C05EaB7716AcB6f83ebF6268F8EECDA8892Ba1); // hemi-mainnet
        _registerDVN("Nethermind", 30367, 0x8E49eF1DfAe17e547CA0E7526FfDA81FbaCA810A); // hyperliquid-mainnet
        _registerDVN("Nethermind", 30394, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // injectiveevm-mainnet
        _registerDVN("Nethermind", 30339, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // ink-mainnet
        _registerDVN("Nethermind", 30284, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // iota-mainnet
        _registerDVN("Nethermind", 30330, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // islander-mainnet
        _registerDVN("Nethermind", 30285, 0x57eB450b257E6A5d65CDc9A7B95245139975eaCf); // joc-mainnet
        _registerDVN("Nethermind", 30375, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // katana-mainnet
        _registerDVN("Nethermind", 30177, 0x6a4C9096F162f0ab3C0517B0a40dc1CE44785e16); // kava-mainnet
        _registerDVN("Nethermind", 30150, 0x6a4C9096F162f0ab3C0517B0a40dc1CE44785e16); // klaytn-mainnet
        _registerDVN("Nethermind", 30373, 0x62aA89bAd332788021F6F4F4Fb196D5Fe59C27a6); // lens-mainnet
        _registerDVN("Nethermind", 30309, 0x18f76f0d8CCD176BbE59B3870fa486d1Fff87026); // lightlink-mainnet
        _registerDVN("Nethermind", 30321, 0x4b92BC2A7d681bf5230472C80d92aCFE9A6b9435); // lisk-mainnet
        _registerDVN("Nethermind", 30217, 0x247624e2143504730aeC22912ed41F092498bEf2); // manta-mainnet
        _registerDVN("Nethermind", 30181, 0xB19A9370D404308040A9760678c8Ca28aFfbbb76); // mantle-mainnet
        _registerDVN("Nethermind", 30266, 0xabC9b1819cc4D9846550F928B985993cF6240439); // merlin-mainnet
        _registerDVN("Nethermind", 30176, 0x08095eceD6c0b46D50eE45a6a59C0fD3De0b0855); // meter-mainnet
        _registerDVN("Nethermind", 30151, 0x6ABdb569Dc985504cCcB541ADE8445E5266e7388); // metis-mainnet
        _registerDVN("Nethermind", 30260, 0xcd37CA043f8479064e10635020c65FfC005d36f6); // mode-mainnet
        _registerDVN("Nethermind", 30390, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // monad-mainnet
        _registerDVN("Nethermind", 30126, 0x790d7B1E97a086eb0012393b65a5B32cE58a04Dc); // moonbeam-mainnet
        _registerDVN("Nethermind", 30167, 0xfE1cD27827E16b07E61A4AC96b521bDB35e00328); // moonriver-mainnet
        _registerDVN("Nethermind", 30322, 0xdf30C9f6A70cE65A152c5Bd09826525D7E97Ba49); // morph-mainnet
        _registerDVN("Nethermind", 30331, 0xE33de1A8cf9bcdC6b509C44EEF66f47c65dA6d47); // mp1-mainnet
        _registerDVN("Nethermind", 30395, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // nexera-mainnet
        _registerDVN("Nethermind", 30369, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // nibiru-mainnet
        _registerDVN("Nethermind", 30388, 0x95729Ea44326f8adD8A9b1d987279DBdC1DD3dFf); // og-mainnet
        _registerDVN("Nethermind", 30202, 0x6a4C9096F162f0ab3C0517B0a40dc1CE44785e16); // opbnb-mainnet
        _registerDVN("Nethermind", 30392, 0x97841D4AB18E9A923322A002d5b8Eb42b31Ccdb5); // openledger-mainnet
        _registerDVN("Nethermind", 30111, 0xa7b5189bcA84Cd304D8553977c7C614329750d99); // optimism-mainnet
        _registerDVN("Nethermind", 30213, 0x6a4C9096F162f0ab3C0517B0a40dc1CE44785e16); // orderly-mainnet
        _registerDVN("Nethermind", 30302, 0x725fAFe20B74FF6f88DAEA0C506190A8f1037635); // peaq-mainnet
        _registerDVN("Nethermind", 30383, 0xa51cE237FaFA3052D5d3308Df38A024724Bb1274); // plasma-mainnet
        _registerDVN("Nethermind", 30370, 0x882a1EE8891c7d22310dedf032eF9653785532B8); // plumephoenix-mainnet
        _registerDVN("Nethermind", 30109, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // polygon-mainnet
        _registerDVN("Nethermind", 30235, 0xB53648CA1aA054A80159c1175c03679fdC76bf88); // rarible-mainnet
        _registerDVN("Nethermind", 30333, 0x05AaEfDf9dB6E0f7d27FA3b6EE099EDB33dA029E); // rootstock-mainnet
        _registerDVN("Nethermind", 30214, 0x446755349101cB20c582C224462c3912d3584dCE); // scroll-mainnet
        _registerDVN("Nethermind", 30280, 0xD24972c11F91c1bB9eaEe97ec96bB9c33cF7af24); // sei-mainnet
        _registerDVN("Nethermind", 30230, 0x5fddD320a1e29bB466Fa635661b125D51D976f92); // shimmer-mainnet
        _registerDVN("Nethermind", 30379, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // silicon-mainnet
        _registerDVN("Nethermind", 30380, 0x5FA12ebC08e183C1F5d44678cF897edEfe68738B); // somnia-mainnet
        _registerDVN("Nethermind", 30340, 0x5Cc4E4d2cDf15795Dc5EA383b8768ec91A587719); // soneium-mainnet
        _registerDVN("Nethermind", 30332, 0x05AaEfDf9dB6E0f7d27FA3b6EE099EDB33dA029E); // sonic-mainnet
        _registerDVN("Nethermind", 30334, 0xa1A31D9ddf919e87a23A1416b0aa0b600D32435D); // sophon-mainnet
        _registerDVN("Nethermind", 30396, 0x9bCd17A654bffAa6f8fEa38D19661a7210e22196); // stable-mainnet
        _registerDVN("Nethermind", 30364, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // story-mainnet
        _registerDVN("Nethermind", 30374, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // subtensorevm-mainnet
        _registerDVN("Nethermind", 30327, 0x07C05EaB7716AcB6f83ebF6268F8EECDA8892Ba1); // superposition-mainnet
        _registerDVN("Nethermind", 30335, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // swell-mainnet
        _registerDVN("Nethermind", 30377, 0x97841D4AB18E9A923322A002d5b8Eb42b31Ccdb5); // tac-mainnet
        _registerDVN("Nethermind", 30290, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // taiko-mainnet
        _registerDVN("Nethermind", 30199, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // telos-mainnet
        _registerDVN("Nethermind", 30196, 0x790d7B1E97a086eb0012393b65a5B32cE58a04Dc); // tomo-mainnet
        _registerDVN("Nethermind", 30320, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // unichain-mainnet
        _registerDVN("Nethermind", 30319, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // worldchain-mainnet
        _registerDVN("Nethermind", 30236, 0xaCDe1f22EEAb249d3ca6Ba8805C8fEe9f52a16e7); // xai-mainnet
        _registerDVN("Nethermind", 30291, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // xchain-mainnet
        _registerDVN("Nethermind", 30365, 0x1294E3347ec64Fd63e1d0594Dc1294247cd237C7); // xdc-mainnet
        _registerDVN("Nethermind", 30274, 0x28af4dADbc5066e994986E8bb105240023dC44B6); // xlayer-mainnet
        _registerDVN("Nethermind", 30216, 0x809CDE2AfcF8627312E87a6a7bbFFaB3F8F347c7); // xpla-mainnet
        _registerDVN("Nethermind", 30303, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // zircuit-mainnet
        _registerDVN("Nethermind", 30183, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // zkconsensys-mainnet
        _registerDVN("Nethermind", 30158, 0x7A7dDC46882220a075934f40380d3A7e1e87d409); // zkpolygon-mainnet
        _registerDVN("Nethermind", 30165, 0xb183c2b91cf76cAd13602b32ADa2Fd273f19009C); // zksync-mainnet
        _registerDVN("Nethermind", 30386, 0x38179D3BFa6Ef1d69A8A7b0b671BA3D8836B2AE8); // zkverify-mainnet
        _registerDVN("Nethermind", 30195, 0xa7b5189bcA84Cd304D8553977c7C614329750d99); // zora-mainnet

        // Nodesguru
        _registerDVN("Nodesguru", 30110, 0xD954bF7968eF68875c9100c9ec890f969504d120); // arbitrum-mainnet
        _registerDVN("Nodesguru", 30106, 0xD251D8a85cDfC84518b9454EE6a8D017E503F56C); // avalanche-mainnet
        _registerDVN("Nodesguru", 30102, 0x1bab20E7FDc79257729CB596BEF85db76C44915E); // bsc-mainnet
        _registerDVN("Nodesguru", 30101, 0x9F45834F0C8042e36935781b944443e906886a87); // ethereum-mainnet
        _registerDVN("Nodesguru", 30112, 0x05AaEfDf9dB6E0f7d27FA3b6EE099EDB33dA029E); // fantom-mainnet
        _registerDVN("Nodesguru", 30294, 0x4D52f5bc932cf1A854381A85ad9ED79B8497c153); // gravity-mainnet
        _registerDVN("Nodesguru", 30111, 0xe6CD8c2E46Ef396DF88048449e5B1C75172b40C3); // optimism-mainnet
        _registerDVN("Nodesguru", 30302, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // peaq-mainnet
        _registerDVN("Nodesguru", 30109, 0xF7DDEE427507cdb6885E53CAAaa1973b1Fe29357); // polygon-mainnet
        _registerDVN("Nodesguru", 30301, 0x3A5a74f863ec48c1769C4Ee85f6C3d70f5655E2A); // zklink-mainnet

        // Nodit
        _registerDVN("Nodit", 30110, 0x4c41b4EDf85DEe828C2cFcc80019CB2BbCFb69a5); // arbitrum-mainnet
        _registerDVN("Nodit", 30106, 0x0F56cE0cA0595792Db727A21596edc2fd39be444); // avalanche-mainnet
        _registerDVN("Nodit", 30102, 0xEeCE50190806fA57016028d31D8631419882401c); // bsc-mainnet
        _registerDVN("Nodit", 30101, 0x0Cea5a94F8cd3330c4F84944bF4500F8daCD440C); // ethereum-mainnet
        _registerDVN("Nodit", 30111, 0x1288cDad593856D7672F82e4cC5fdFE1CF59646d); // optimism-mainnet
        _registerDVN("Nodit", 30109, 0x4c41b4EDf85DEe828C2cFcc80019CB2BbCFb69a5); // polygon-mainnet

        // Omnicat
        _registerDVN("Omnicat", 30110, 0xd1C70192CC0eb9a89e3D9032b9FAcab259A0a1e9); // arbitrum-mainnet
        _registerDVN("Omnicat", 30184, 0xe6CD8c2E46Ef396DF88048449e5B1C75172b40C3); // base-mainnet
        _registerDVN("Omnicat", 30243, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // blast-mainnet
        _registerDVN("Omnicat", 30102, 0xdfF3F73C260b3361d4F006B02972c6aF6C5c5417); // bsc-mainnet
        _registerDVN("Omnicat", 30159, 0x25e0e650a78e6304A3983Fc4b7Ffc6544b1bEea6); // canto-mainnet
        _registerDVN("Omnicat", 30101, 0xf10Ea2c0D43bC4973cfBCc94eBAfC39d1D4aF118); // ethereum-mainnet
        _registerDVN("Omnicat", 30109, 0xa2d10677441230C4AeD58030e4EA6Ba7Bfd80393); // polygon-mainnet

        // Omnix
        _registerDVN("Omnix", 30110, 0xabEa0b6B9237b589e676dC16f6D74Bf7612591f4); // arbitrum-mainnet
        _registerDVN("Omnix", 30106, 0x21cAF0BCE846AAA78C9f23C5A4eC5988EcBf9988); // avalanche-mainnet
        _registerDVN("Omnix", 30184, 0xeEdE111103535e473451311e26C3E6660b0F77e1); // base-mainnet
        _registerDVN("Omnix", 30102, 0x5a4c666E9C7aA86FD4fBFDFbfd04646DcC45C6c5); // bsc-mainnet
        _registerDVN("Omnix", 30101, 0xAf75bfD402f3d4EE84978179a6c87D16c4Bd1724); // ethereum-mainnet
        _registerDVN("Omnix", 30112, 0xE0F0FbBDBF9d398eCA0dd8c86d1F308D895b9Eb7); // fantom-mainnet
        _registerDVN("Omnix", 30367, 0x3E3A9bC9149Ddf1D3A3ea51c0A49Eb9fe6347043); // hyperliquid-mainnet
        _registerDVN("Omnix", 30111, 0x03d2414476a742Aba715BcC337583C820525E22a); // optimism-mainnet
        _registerDVN("Omnix", 30109, 0x06b85533967179eD5bC9C754b84aE7d02f7eD830); // polygon-mainnet
        _registerDVN("Omnix", 30278, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // sanko-mainnet

        // Ondo Staging
        _registerDVN("Ondo Staging", 30110, 0x2f2F1c6025E8Da125e2Afd73BA17d3cBDfE3d093); // arbitrum-mainnet
        _registerDVN("Ondo Staging", 30102, 0x089E70BC883Ad0b6551e513bF7A71Ffd2059f9F1); // bsc-mainnet
        _registerDVN("Ondo Staging", 30101, 0xF34D1B07c64c4F4d492aE3DdD0AaB0658A2975eb); // ethereum-mainnet
        _registerDVN("Ondo Staging", 30181, 0x377B51593a03B82543c1508fE7e75Aba6acDE008); // mantle-mainnet

        // P2P
        _registerDVN("P2P", 30324, 0x52B7E1958F6Ad3E195DC30578dA5Fa7ac5827A2A); // abstract-mainnet
        _registerDVN("P2P", 30110, 0xB3Ce0A5D132Cd9Bf965aBa435E650c55Edce0062); // arbitrum-mainnet
        _registerDVN("P2P", 30106, 0xE94aE34DfCC87A61836938641444080B98402c75); // avalanche-mainnet
        _registerDVN("P2P", 30184, 0x5b6735c66d97479cCD18294fc96B3084EcB2fa3f); // base-mainnet
        _registerDVN("P2P", 30362, 0x3b247F1B48F055EbF2DB593672B98C9597E3081E); // bera-mainnet
        _registerDVN("P2P", 30102, 0x439264FB87581a70Bb6D7bEFd16b636521B0Ad2D); // bsc-mainnet
        _registerDVN("P2P", 30125, 0x7E65BDd15C8Db8995F80aBf0D6593b57dc8BE437); // celo-mainnet
        _registerDVN("P2P", 30101, 0x06559EE34D85a88317Bf0bfE307444116c631b67); // ethereum-mainnet
        _registerDVN("P2P", 30112, 0x439264FB87581a70Bb6D7bEFd16b636521B0Ad2D); // fantom-mainnet
        _registerDVN("P2P", 30145, 0xf10Ea2c0D43bC4973cfBCc94eBAfC39d1D4aF118); // gnosis-mainnet
        _registerDVN("P2P", 30367, 0xC7423626016bc40375458bc0277F28681EC91C8e); // hyperliquid-mainnet
        _registerDVN("P2P", 30339, 0x68b6Fb5e728dB92A09BA810595915aE3d7399e40); // ink-mainnet
        _registerDVN("P2P", 30150, 0xF7a1963e52b1471d01e320d547b72b05F443C9e6); // klaytn-mainnet
        _registerDVN("P2P", 30181, 0x2206ceb6809bD39f8707ED5eE618f8CFA57E40F2); // mantle-mainnet
        _registerDVN("P2P", 30111, 0x539008c98B17803A273eDf98aBA2d4414Ee3f4D7); // optimism-mainnet
        _registerDVN("P2P", 30302, 0x795720d981C1f4D4d4381682225572c431284592); // peaq-mainnet
        _registerDVN("P2P", 30383, 0xfD429433af17c5C75E4c8BC84b8F6dCD1b2C051A); // plasma-mainnet
        _registerDVN("P2P", 30109, 0x9EEee79F5dBC4D99354b5CB547c138Af432F937b); // polygon-mainnet
        _registerDVN("P2P", 30214, 0xC6a6324932B399D6A673B7Ed0af671F28033E046); // scroll-mainnet
        _registerDVN("P2P", 30280, 0xA83A87a0bDce466edfBB6794404E1D7F556B8F20); // sei-mainnet
        _registerDVN("P2P", 30380, 0xE57aF13D6676F7a37d37AB603aaeA6D63B1dEe8E); // somnia-mainnet
        _registerDVN("P2P", 30340, 0xf85D19E8884EB985A7f77BA385409ec7aD2923A5); // soneium-mainnet
        _registerDVN("P2P", 30332, 0x45A7305c65AAd28384F20a80F87a5183772E4F70); // sonic-mainnet
        _registerDVN("P2P", 30374, 0x6c5f923B63Fdd52fb9C45dAeFA8695fA6b55a935); // subtensorevm-mainnet
        _registerDVN("P2P", 30377, 0x965A80Dc87cec5848310E612DeAD84B543AeF874); // tac-mainnet
        _registerDVN("P2P", 30320, 0xab82E9b24004b954985528dAc14D1B020722a3c8); // unichain-mainnet
        _registerDVN("P2P", 30183, 0x0b239476A771834D846Cb505817baC3C391c338A); // zkconsensys-mainnet
        _registerDVN("P2P", 30195, 0xD1b5493e712081A6FBAb73116405590046668F6b); // zora-mainnet

        // POPS
        _registerDVN("POPS", 30110, 0x8fa9eEf18c2A1459024f0B44714e5aCc1Ce7f5e8); // arbitrum-mainnet
        _registerDVN("POPS", 30106, 0x2b8CBEa81315130A4C422e875063362640ddFeB0); // avalanche-mainnet
        _registerDVN("POPS", 30184, 0xA9d11632eC5f9502D39afF28d66415F6CCa37544); // base-mainnet
        _registerDVN("POPS", 30362, 0x4055fAd06ded1F57A1b4D07455665a9Bbc33C700); // bera-mainnet
        _registerDVN("POPS", 30102, 0x33E5fcC13D7439cC62d54c41AA966197145b3Cd7); // bsc-mainnet
        _registerDVN("POPS", 30101, 0x94AAfe0A92A8300f0A2100A7f3DE47d6845747A9); // ethereum-mainnet
        _registerDVN("POPS", 30112, 0x78203678D264063815Dac114eA810E9837Cd80f7); // fantom-mainnet
        _registerDVN("POPS", 30145, 0x790d7B1E97a086eb0012393b65a5B32cE58a04Dc); // gnosis-mainnet
        _registerDVN("POPS", 30126, 0x7fe673201724925B5c477d4E1A4Bd3E954688cF5); // moonbeam-mainnet
        _registerDVN("POPS", 30111, 0xe552485d02EDd3067FE7FCbD4dd56BB1D3A998D2); // optimism-mainnet
        _registerDVN("POPS", 30109, 0xa75ABcC0FAB6aE09c8FD808bEc7bE7E88fe31D6B); // polygon-mainnet
        _registerDVN("POPS", 30214, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // scroll-mainnet

        // Paxos
        _registerDVN("Paxos", 30110, 0x8E5f5825602Bc5db725974Bb9e60677d4adC5fbe); // arbitrum-mainnet
        _registerDVN("Paxos", 30101, 0xb0B2EF168F52F6d1e42f461e11117295eF992daf); // ethereum-mainnet
        _registerDVN("Paxos", 30339, 0x1C5C9C9b50885319BD3cB7e67294136CD436BeE3); // ink-mainnet
        _registerDVN("Paxos", 30274, 0x8bEFB8cd9529e539B095251Ea3a058e710225D30); // xlayer-mainnet

        // Planetarium
        _registerDVN("Planetarium", 30110, 0xe6CD8c2E46Ef396DF88048449e5B1C75172b40C3); // arbitrum-mainnet
        _registerDVN("Planetarium", 30106, 0x2AC038606fff3FB00317B8F0CcFB4081694aCDD0); // avalanche-mainnet
        _registerDVN("Planetarium", 30102, 0x05AaEfDf9dB6E0f7d27FA3b6EE099EDB33dA029E); // bsc-mainnet
        _registerDVN("Planetarium", 30101, 0x972ED7bd3d42D9C0bea3632992Ebf7e97186ea4A); // ethereum-mainnet
        _registerDVN("Planetarium", 30112, 0xF7DDEE427507cdb6885E53CAAaa1973b1Fe29357); // fantom-mainnet
        _registerDVN("Planetarium", 30111, 0x021e401C2a1A60618c5E6353A40524971Eba1E8D); // optimism-mainnet
        _registerDVN("Planetarium", 30109, 0x2AC038606fff3FB00317B8F0CcFB4081694aCDD0); // polygon-mainnet

        // Polyhedra
        _registerDVN("Polyhedra", 30110, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // arbitrum-mainnet
        _registerDVN("Polyhedra", 30106, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // avalanche-mainnet
        _registerDVN("Polyhedra", 30184, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // base-mainnet
        _registerDVN("Polyhedra", 30243, 0x0ff4cc28826356503BB79c00637bec0eE006f237); // blast-mainnet
        _registerDVN("Polyhedra", 30102, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // bsc-mainnet
        _registerDVN("Polyhedra", 30125, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // celo-mainnet
        _registerDVN("Polyhedra", 30153, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // coredao-mainnet
        _registerDVN("Polyhedra", 30283, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // cyber-mainnet
        _registerDVN("Polyhedra", 30101, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // ethereum-mainnet
        _registerDVN("Polyhedra", 30112, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // fantom-mainnet
        _registerDVN("Polyhedra", 30295, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // flare-mainnet
        _registerDVN("Polyhedra", 30145, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // gnosis-mainnet
        _registerDVN("Polyhedra", 30150, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // klaytn-mainnet
        _registerDVN("Polyhedra", 30217, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // manta-mainnet
        _registerDVN("Polyhedra", 30181, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // mantle-mainnet
        _registerDVN("Polyhedra", 30266, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // merlin-mainnet
        _registerDVN("Polyhedra", 30151, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // metis-mainnet
        _registerDVN("Polyhedra", 30260, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // mode-mainnet
        _registerDVN("Polyhedra", 30126, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // moonbeam-mainnet
        _registerDVN("Polyhedra", 30175, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // nova-mainnet
        _registerDVN("Polyhedra", 30202, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // opbnb-mainnet
        _registerDVN("Polyhedra", 30111, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // optimism-mainnet
        _registerDVN("Polyhedra", 30109, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // polygon-mainnet
        _registerDVN("Polyhedra", 30214, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // scroll-mainnet
        _registerDVN("Polyhedra", 30280, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // sei-mainnet
        _registerDVN("Polyhedra", 30274, 0x8ddF05F9A5c488b4973897E278B58895bF87Cb24); // xlayer-mainnet
        _registerDVN("Polyhedra", 30183, 0xE014fe8c4d5C23EDB7AC4011F226e869ac7Ef5CC); // zkconsensys-mainnet

        // Predicate
        _registerDVN("Predicate", 30101, 0x006E1248BE5e40B4A4E7099397719Df7aB872De7); // ethereum-mainnet
        _registerDVN("Predicate", 30370, 0xD7bB44516b476ca805FB9d6fc5b508ef3Ee9448D); // plumephoenix-mainnet

        // Quantoz
        _registerDVN("Quantoz", 30101, 0x4066b6e7BfD761B579902E7e8D03F4feb9B9536E); // ethereum-mainnet
        _registerDVN("Quantoz", 30109, 0xA773b91c42FC8e77C37d7396B2814b8FA485b2c3); // polygon-mainnet

        // Republic
        _registerDVN("Republic", 30106, 0x1feB08B1A53A9710AfcE82D380B8c2833C69a37e); // avalanche-mainnet
        _registerDVN("Republic", 30102, 0xF7DDEE427507cdb6885E53CAAaa1973b1Fe29357); // bsc-mainnet
        _registerDVN("Republic", 30101, 0xA1Bc1B9af01A0ec78883AA5DC7DECDCe897E1E76); // ethereum-mainnet
        _registerDVN("Republic", 30109, 0x547bF6889B1095b7CC6e525A1F8E8Fdb26134a38); // polygon-mainnet

        // Restake
        _registerDVN("Restake", 30110, 0x969A0bdd86A230345AD87A6a381DE5ED9E6cda85); // arbitrum-mainnet
        _registerDVN("Restake", 30106, 0x377B51593a03B82543c1508fE7e75Aba6acDE008); // avalanche-mainnet
        _registerDVN("Restake", 30102, 0x4D52f5bc932cf1A854381A85ad9ED79B8497c153); // bsc-mainnet
        _registerDVN("Restake", 30101, 0xE4193136B92bA91402313e95347c8e9FAD8d27d0); // ethereum-mainnet
        _registerDVN("Restake", 30112, 0x4D52f5bc932cf1A854381A85ad9ED79B8497c153); // fantom-mainnet
        _registerDVN("Restake", 30111, 0xcced05c3667877B545285B25f19F794436A1c481); // optimism-mainnet
        _registerDVN("Restake", 30109, 0x2afa3787cd95fee5D5753cd717EF228eb259f4ea); // polygon-mainnet

        // Shrapnel
        _registerDVN("Shrapnel", 30110, 0x7B8a0fD9D6ae5011d5cBD3E85Ed6D5510F98c9Bf); // arbitrum-mainnet
        _registerDVN("Shrapnel", 30106, 0x6A110d94e1baA6984A3d904bab37Ae49b90E6B4f); // avalanche-mainnet
        _registerDVN("Shrapnel", 30102, 0xb4FA7f1C67E5Ec99B556Ec92CbDdBCdd384106F2); // bsc-mainnet
        _registerDVN("Shrapnel", 30101, 0xCe97511db880571A7C31821eB026Ef12fCaC892e); // ethereum-mainnet
        _registerDVN("Shrapnel", 30112, 0xb4FA7f1C67E5Ec99B556Ec92CbDdBCdd384106F2); // fantom-mainnet
        _registerDVN("Shrapnel", 30111, 0xd36246C322Ee102A2203bCA9cafb84c179D306F6); // optimism-mainnet
        _registerDVN("Shrapnel", 30109, 0x54dD79f5cE72b51FCBbcb170Dd01E32034323565); // polygon-mainnet

        // Stablelab
        _registerDVN("Stablelab", 30110, 0xcd37CA043f8479064e10635020c65FfC005d36f6); // arbitrum-mainnet
        _registerDVN("Stablelab", 30106, 0x5fddD320a1e29bB466Fa635661b125D51D976f92); // avalanche-mainnet
        _registerDVN("Stablelab", 30102, 0xabC9b1819cc4D9846550F928B985993cF6240439); // bsc-mainnet
        _registerDVN("Stablelab", 30101, 0x5fddD320a1e29bB466Fa635661b125D51D976f92); // ethereum-mainnet
        _registerDVN("Stablelab", 30112, 0xabC9b1819cc4D9846550F928B985993cF6240439); // fantom-mainnet
        _registerDVN("Stablelab", 30111, 0xcd37CA043f8479064e10635020c65FfC005d36f6); // optimism-mainnet
        _registerDVN("Stablelab", 30109, 0xabC9b1819cc4D9846550F928B985993cF6240439); // polygon-mainnet

        // Stakingcabin
        _registerDVN("Stakingcabin", 30110, 0xb0646Fb9028364aC1f04477271375EF32A8A5e62); // arbitrum-mainnet
        _registerDVN("Stakingcabin", 30106, 0xb6323Aa5A3FC07d93A3cf0F1044447F2a88B7dAb); // avalanche-mainnet
        _registerDVN("Stakingcabin", 30102, 0xcCF6ee53aA0B7c7f190D2a7B273e7b04CCE14D21); // bsc-mainnet
        _registerDVN("Stakingcabin", 30283, 0x2206ceb6809bD39f8707ED5eE618f8CFA57E40F2); // cyber-mainnet
        _registerDVN("Stakingcabin", 30101, 0xCd0ca0619fc8dB4d47B19A1f04105312952E5F6D); // ethereum-mainnet
        _registerDVN("Stakingcabin", 30112, 0x193204535913df3A3b350fcd2e1C97D047AbB409); // fantom-mainnet
        _registerDVN("Stakingcabin", 30295, 0xCe97511db880571A7C31821eB026Ef12fCaC892e); // flare-mainnet
        _registerDVN("Stakingcabin", 30294, 0x0D155ec1Dfc983E919C318964fD16078408E99CC); // gravity-mainnet
        _registerDVN("Stakingcabin", 30265, 0x1294E3347ec64Fd63e1d0594Dc1294247cd237C7); // homeverse-mainnet
        _registerDVN("Stakingcabin", 30285, 0x34730f2570E6cff8B1C91FaaBF37D0DD917c4367); // joc-mainnet
        _registerDVN("Stakingcabin", 30266, 0xfd4AD9CDBd06FD4D8cA521307BF63a20239208ef); // merlin-mainnet
        _registerDVN("Stakingcabin", 30111, 0x56D675bFd1574fF170723689223c3A93DeE5fA78); // optimism-mainnet
        _registerDVN("Stakingcabin", 30302, 0x2EdfE0220A74d9609c79711a65E3A2F2A85Dc83b); // peaq-mainnet
        _registerDVN("Stakingcabin", 30109, 0xcd19d26710CACf8241583769f353EA7395159007); // polygon-mainnet
        _registerDVN("Stakingcabin", 30278, 0x1253CA32712171b5D28115A1346F2B22BB9a41D5); // sanko-mainnet
        _registerDVN("Stakingcabin", 30280, 0x93d2d7AADC9F2Cf5EbC88e9703E06dB09b8Fd85B); // sei-mainnet
        _registerDVN("Stakingcabin", 30290, 0x2c7185f5B0976397d9eB5c19d639d4005e6708f0); // taiko-mainnet
        _registerDVN("Stakingcabin", 30303, 0x92ef4381a03372985985E70fb15E9F081E2e8D14); // zircuit-mainnet

        // Stargate
        _registerDVN("Stargate", 30324, 0xCec9f0A49073ac4a1C439D06cb9448512389a64E); // abstract-mainnet
        _registerDVN("Stargate", 30312, 0x794C0b0071D4A926c443468f027912e693678151); // ape-mainnet
        _registerDVN("Stargate", 30384, 0x313328609a9C38459CaE56625FFf7F2AD6dcde3b); // apexfusionnexus-mainnet
        _registerDVN("Stargate", 30110, 0x5756a74e8e18D8392605bA667171962B2b2826B5); // arbitrum-mainnet
        _registerDVN("Stargate", 30211, 0xE11c808bC6099Abc9bE566C9017aa2Ab0f131d35); // aurora-mainnet
        _registerDVN("Stargate", 30106, 0x252B234545e154543Ad2784c7111Eb90406be836); // avalanche-mainnet
        _registerDVN("Stargate", 30184, 0xcdF31d62140204C08853b547E64707110fBC6680); // base-mainnet
        _registerDVN("Stargate", 30362, 0x6E70FCdc42D3d63748B7d8883399Dcb16BBB5c8c); // bera-mainnet
        _registerDVN("Stargate", 30376, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // botanix-mainnet
        _registerDVN("Stargate", 30102, 0xac8de74CE0A44A5e73BBc709fe800406F58431e0); // bsc-mainnet
        _registerDVN("Stargate", 30381, 0x64a344a15e4DE73F393E345E6Bfe937F34ee1f90); // camp-mainnet
        _registerDVN("Stargate", 30323, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // codex-mainnet
        _registerDVN("Stargate", 30153, 0xe6CD8c2E46Ef396DF88048449e5B1C75172b40C3); // coredao-mainnet
        _registerDVN("Stargate", 30359, 0x2Ae36A544b904F2f2960F6Fd1a6084b4b11ba334); // cronosevm-mainnet
        _registerDVN("Stargate", 30360, 0x0D1bc4Efd08940eB109Ef3040c1386d09B6334E0); // cronoszkevm-mainnet
        _registerDVN("Stargate", 30267, 0x80442151791BbDd89117719e508115EBc1Ce2D93); // degen-mainnet
        _registerDVN("Stargate", 30393, 0xa6F5DDBF0Bd4D03334523465439D301080574742); // doma-mainnet
        _registerDVN("Stargate", 30328, 0x97F930a15172F38B7e947778889424e37b5DF316); // edu-mainnet
        _registerDVN("Stargate", 30101, 0x8FafAE7Dd957044088b3d0F67359C327c6200d18); // ethereum-mainnet
        _registerDVN("Stargate", 30292, 0x31F748a368a893Bdb5aBB67ec95F232507601A73); // etherlink-mainnet
        _registerDVN("Stargate", 30295, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // flare-mainnet
        _registerDVN("Stargate", 30336, 0xd1C70192CC0eb9a89e3D9032b9FAcab259A0a1e9); // flow-mainnet
        _registerDVN("Stargate", 30138, 0x9F45834F0C8042e36935781b944443e906886a87); // fuse-mainnet
        _registerDVN("Stargate", 30342, 0xd1C70192CC0eb9a89e3D9032b9FAcab259A0a1e9); // glue-mainnet
        _registerDVN("Stargate", 30145, 0xFCeA5cEF8b1ae3A454577C9444CDD95c1284B0cF); // gnosis-mainnet
        _registerDVN("Stargate", 30361, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // goat-mainnet
        _registerDVN("Stargate", 30294, 0x70BF42C69173d6e33b834f59630DAC592C70b369); // gravity-mainnet
        _registerDVN("Stargate", 30316, 0x178D9517FC35633afDA67b8c236e568997a3Ae03); // hedera-mainnet
        _registerDVN("Stargate", 30329, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // hemi-mainnet
        _registerDVN("Stargate", 30339, 0xE900e073BADaFdC6F72541F34E6b701bde835487); // ink-mainnet
        _registerDVN("Stargate", 30284, 0xf18A7d86917653725aFB7C215E47a24F9D784718); // iota-mainnet
        _registerDVN("Stargate", 30330, 0x9EEee79F5dBC4D99354b5CB547c138Af432F937b); // islander-mainnet
        _registerDVN("Stargate", 30177, 0x9CbAF815eD62Ef45C59E9F2Cb05106bAbb4d31d3); // kava-mainnet
        _registerDVN("Stargate", 30150, 0x17720E3F361dCc2f70871a2ce3ac51b0Eaa5c2E4); // klaytn-mainnet
        _registerDVN("Stargate", 30309, 0x0E95cf21aD9376A26997c97f326C5A0a267bB8FF); // lightlink-mainnet
        _registerDVN("Stargate", 30321, 0x3fe00587a7b2432421d739A68bb890ceE55Bc18F); // lisk-mainnet
        _registerDVN("Stargate", 30217, 0xca848BcB059e33Adb260d793EE360924B6Aa8E86); // manta-mainnet
        _registerDVN("Stargate", 30181, 0xfe809470016196573D64A8D17a745bebEA4ecC41); // mantle-mainnet
        _registerDVN("Stargate", 30151, 0x61A1B61A1087be03ABeDC04900Cfcc1C14187237); // metis-mainnet
        _registerDVN("Stargate", 30260, 0x06559EE34D85a88317Bf0bfE307444116c631b67); // mode-mainnet
        _registerDVN("Stargate", 30369, 0x06D99Ffd7c09Ea72e962a06B6e311e513d7c3d20); // nibiru-mainnet
        _registerDVN("Stargate", 30388, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // og-mainnet
        _registerDVN("Stargate", 30111, 0xfe6507F094155caBB4784403Cd784C2DF04122dd); // optimism-mainnet
        _registerDVN("Stargate", 30213, 0xD074B6bbCBEC2f2B4c4265DE3D95e521f82bF669); // orderly-mainnet
        _registerDVN("Stargate", 30302, 0x18f76f0d8CCD176BbE59B3870fa486d1Fff87026); // peaq-mainnet
        _registerDVN("Stargate", 30383, 0xabC9b1819cc4D9846550F928B985993cF6240439); // plasma-mainnet
        _registerDVN("Stargate", 30370, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // plumephoenix-mainnet
        _registerDVN("Stargate", 30109, 0xC79F0B1bcb7cDAE9f9BA547dcFc57cBfcd2993A5); // polygon-mainnet
        _registerDVN("Stargate", 30235, 0x2fa870cEE4da57De84d1dB36759d4716AD7E5038); // rarible-mainnet
        _registerDVN("Stargate", 30333, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // rootstock-mainnet
        _registerDVN("Stargate", 30214, 0xb87591D8B0b93faE8b631A073577c40e8Dd46A62); // scroll-mainnet
        _registerDVN("Stargate", 30280, 0xBd00C87850416db0995EF8030b104F875E1bdD15); // sei-mainnet
        _registerDVN("Stargate", 30380, 0xA83A87a0bDce466edfBB6794404E1D7F556B8F20); // somnia-mainnet
        _registerDVN("Stargate", 30340, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // soneium-mainnet
        _registerDVN("Stargate", 30332, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // sonic-mainnet
        _registerDVN("Stargate", 30334, 0x7cC1A4A700AAb8FbA8160a4e09B04a9A68C6D914); // sophon-mainnet
        _registerDVN("Stargate", 30364, 0xA80AA110f05C9C6140018aAE0C4E08A70f43350d); // story-mainnet
        _registerDVN("Stargate", 30327, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // superposition-mainnet
        _registerDVN("Stargate", 30335, 0x7976b969A8E9560C483229FfBB855E8440898c9D); // swell-mainnet
        _registerDVN("Stargate", 30290, 0x37473676FF697f2Eba29C8A3105309AbF00bA013); // taiko-mainnet
        _registerDVN("Stargate", 30199, 0xA80AA110f05C9C6140018aAE0C4E08A70f43350d); // telos-mainnet
        _registerDVN("Stargate", 30320, 0x9885110b909E88bb94f7f767A68ec2558B2AfA73); // unichain-mainnet
        _registerDVN("Stargate", 30319, 0x7cEc38c06a2FEC9Fd525B1925544110204CbB5f6); // worldchain-mainnet
        _registerDVN("Stargate", 30291, 0x56053A8f4db677e5774F8Ee5BdD9D2dC270075f3); // xchain-mainnet
        _registerDVN("Stargate", 30365, 0x4FE90e0f2A99e464d6E97B161d72101CD03C20fe); // xdc-mainnet
        _registerDVN("Stargate", 30183, 0xEf269BBaDB81DE86E4b3278fa1DAe1723545268b); // zkconsensys-mainnet
        _registerDVN("Stargate", 30165, 0x62aA89bAd332788021F6F4F4Fb196D5Fe59C27a6); // zksync-mainnet
        _registerDVN("Stargate", 30195, 0x376839ad96f4f0CDfFe10AAF987aBaD3AF0A8901); // zora-mainnet

        // Superduper
        _registerDVN("Superduper", 30110, 0x539008c98B17803A273eDf98aBA2d4414Ee3f4D7); // arbitrum-mainnet
        _registerDVN("Superduper", 30106, 0x0E95cf21aD9376A26997c97f326C5A0a267bB8FF); // avalanche-mainnet
        _registerDVN("Superduper", 30102, 0xBD40c9047980500C46B8aed4462e2f889299FEbE); // bsc-mainnet
        _registerDVN("Superduper", 30101, 0x92ef4381a03372985985E70fb15E9F081E2e8D14); // ethereum-mainnet
        _registerDVN("Superduper", 30112, 0xBD40c9047980500C46B8aed4462e2f889299FEbE); // fantom-mainnet
        _registerDVN("Superduper", 30111, 0xdf30C9f6A70cE65A152c5Bd09826525D7E97Ba49); // optimism-mainnet
        _registerDVN("Superduper", 30109, 0x8Fc629aa400D4D9c0B118F2685a49316552ABf27); // polygon-mainnet

        // Superform
        _registerDVN("Superform", 30110, 0x5496d03d9065B08e5677E1c5D1107110Bb05d445); // arbitrum-mainnet
        _registerDVN("Superform", 30106, 0x8fb0B7D74B557e4b45EF89648BAc197EAb2E4325); // avalanche-mainnet
        _registerDVN("Superform", 30184, 0xEb62f578497Bdc351dD650853a751135212fAF49); // base-mainnet
        _registerDVN("Superform", 30243, 0x0E95cf21aD9376A26997c97f326C5A0a267bB8FF); // blast-mainnet
        _registerDVN("Superform", 30102, 0xF4c489AfD83625F510947e63ff8F90dfEE0aE46C); // bsc-mainnet
        _registerDVN("Superform", 30101, 0x7518f30bd5867b5fA86702556245Dead173afE46); // ethereum-mainnet
        _registerDVN("Superform", 30112, 0x2EdfE0220A74d9609c79711a65E3A2F2A85Dc83b); // fantom-mainnet
        _registerDVN("Superform", 30111, 0xb0B2EF168F52F6d1e42f461e11117295eF992daf); // optimism-mainnet
        _registerDVN("Superform", 30109, 0x1E4CE74ccf5498B19900649D9196e64BAb592451); // polygon-mainnet
        _registerDVN("Superform", 30183, 0x7A205ED4e3d7f9d0777594501705D8CD405c3B05); // zkconsensys-mainnet

        // Switchboard
        _registerDVN("Switchboard", 30110, 0xcced05c3667877B545285B25f19F794436A1c481); // arbitrum-mainnet
        _registerDVN("Switchboard", 30106, 0x92ef4381a03372985985E70fb15E9F081E2e8D14); // avalanche-mainnet
        _registerDVN("Switchboard", 30102, 0xf0809F6e760a5452Ee567975EdA7a28dA4a83D38); // bsc-mainnet
        _registerDVN("Switchboard", 30101, 0x276e6B1138d2d49C0Cda86658765d12Ef84550c1); // ethereum-mainnet
        _registerDVN("Switchboard", 30112, 0xf0809F6e760a5452Ee567975EdA7a28dA4a83D38); // fantom-mainnet
        _registerDVN("Switchboard", 30111, 0x313328609a9C38459CaE56625FFf7F2AD6dcde3b); // optimism-mainnet
        _registerDVN("Switchboard", 30109, 0xC6D46F63578635E4a7140cdf4D0eea0fd7bB50eC); // polygon-mainnet

        // TSS
        _registerDVN("TSS", 30312, 0xA2Eb037Ee6AABa1547fCa8804392EB8EF9c33976); // ape-mainnet
        _registerDVN("TSS", 30110, 0xA0Cc33Dd6f4819D473226257792AFe230EC3c67f); // arbitrum-mainnet
        _registerDVN("TSS", 30210, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // astar-mainnet
        _registerDVN("TSS", 30211, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // aurora-mainnet
        _registerDVN("TSS", 30106, 0x5a54fe5234E811466D5366846283323c954310B2); // avalanche-mainnet
        _registerDVN("TSS", 30184, 0xAaB5A48CFC03Efa9cC34A2C1aAcCCB84b4b770e4); // base-mainnet
        _registerDVN("TSS", 30234, 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9); // bb1-mainnet
        _registerDVN("TSS", 30362, 0x306B9a8953B9462F8b826e6768a93C8EA7454965); // bera-mainnet
        _registerDVN("TSS", 30317, 0x6c47E59BC0600942146BF37668fc92b369C85ab7); // bevm-mainnet
        _registerDVN("TSS", 30243, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // blast-mainnet
        _registerDVN("TSS", 30279, 0xbB2753C1B940363d278c81D6402fA89E79Ab4ebc); // bob-mainnet
        _registerDVN("TSS", 30102, 0x5a54fe5234E811466D5366846283323c954310B2); // bsc-mainnet
        _registerDVN("TSS", 30159, 0x377530cdA84DFb2673bF4d145DCF0C4D7fdcB5b6); // canto-mainnet
        _registerDVN("TSS", 30125, 0x071c3F1bc3046c693c3ABBC03a87ca9A30e43bE2); // celo-mainnet
        _registerDVN("TSS", 30212, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // conflux-mainnet
        _registerDVN("TSS", 30153, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // coredao-mainnet
        _registerDVN("TSS", 30267, 0x41Bdb4aa4A63a5b2Efc531858d3118392B1A1C3d); // degen-mainnet
        _registerDVN("TSS", 30118, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // dexalot-mainnet
        _registerDVN("TSS", 30115, 0x88bD5f18a13C22C41Cf5c8cBA12eB371C4bD18D9); // dfk-mainnet
        _registerDVN("TSS", 30149, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // dos-mainnet
        _registerDVN("TSS", 30101, 0x5a54fe5234E811466D5366846283323c954310B2); // ethereum-mainnet
        _registerDVN("TSS", 30292, 0xb87396e0d0d8B12169319803B56dB763Cd738E63); // etherlink-mainnet
        _registerDVN("TSS", 30112, 0xA0Cc33Dd6f4819D473226257792AFe230EC3c67f); // fantom-mainnet
        _registerDVN("TSS", 30255, 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9); // fraxtal-mainnet
        _registerDVN("TSS", 30138, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // fuse-mainnet
        _registerDVN("TSS", 30145, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // gnosis-mainnet
        _registerDVN("TSS", 30361, 0x00961ae3250C2c0dB37a476C0ebA2353Ce800Dae); // goat-mainnet
        _registerDVN("TSS", 30116, 0x3E2EF091D7606E4CA3B8d84bcAf23da0FfA11053); // harmony-mainnet
        _registerDVN("TSS", 30265, 0xcbD35a9b849342AD34a71e072D9947D4AFb4E164); // homeverse-mainnet
        _registerDVN("TSS", 30367, 0xacFC61640598C25bdB273291D889816B2218CD9B); // hyperliquid-mainnet
        _registerDVN("TSS", 30339, 0xf772581dcf3300914D6222C4e6FcF0ed5EF93142); // ink-mainnet
        _registerDVN("TSS", 30284, 0x59dAE6516D2fA7F21195adC0Cda14d819D21031C); // iota-mainnet
        _registerDVN("TSS", 30177, 0xAaB5A48CFC03Efa9cC34A2C1aAcCCB84b4b770e4); // kava-mainnet
        _registerDVN("TSS", 30150, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // klaytn-mainnet
        _registerDVN("TSS", 30197, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // loot-mainnet
        _registerDVN("TSS", 30217, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // manta-mainnet
        _registerDVN("TSS", 30181, 0xAaB5A48CFC03Efa9cC34A2C1aAcCCB84b4b770e4); // mantle-mainnet
        _registerDVN("TSS", 30198, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // meritcircle-mainnet
        _registerDVN("TSS", 30266, 0x41Bdb4aa4A63a5b2Efc531858d3118392B1A1C3d); // merlin-mainnet
        _registerDVN("TSS", 30176, 0x51A6E62D12F2260E697039Ff53bCB102053f5ab7); // meter-mainnet
        _registerDVN("TSS", 30151, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // metis-mainnet
        _registerDVN("TSS", 30260, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // mode-mainnet
        _registerDVN("TSS", 30126, 0xdEeF80c12d49e5DA8e01B05636E2d0C776F6b78D); // moonbeam-mainnet
        _registerDVN("TSS", 30167, 0x84070061032F3e7ea4E068f447FB7cDfC98d57Fe); // moonriver-mainnet
        _registerDVN("TSS", 30322, 0xb250B0b2A9891a035080615Ac30f040d2b7E7FAB); // morph-mainnet
        _registerDVN("TSS", 30175, 0x37aaaf95887624a363effB7762D489E3C05c2a02); // nova-mainnet
        _registerDVN("TSS", 30155, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // okx-mainnet
        _registerDVN("TSS", 30202, 0xA658742d33ebd2ce2F0bdFf73515Aa797Fd161D9); // opbnb-mainnet
        _registerDVN("TSS", 30111, 0xA0Cc33Dd6f4819D473226257792AFe230EC3c67f); // optimism-mainnet
        _registerDVN("TSS", 30213, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // orderly-mainnet
        _registerDVN("TSS", 30302, 0x4b80F7d25c451D204b1C93D9bdf2aB3B04f3EA4a); // peaq-mainnet
        _registerDVN("TSS", 30109, 0x5a54fe5234E811466D5366846283323c954310B2); // polygon-mainnet
        _registerDVN("TSS", 30235, 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9); // rarible-mainnet
        _registerDVN("TSS", 30313, 0x4b80F7d25c451D204b1C93D9bdf2aB3B04f3EA4a); // reya-mainnet
        _registerDVN("TSS", 30278, 0xbB2753C1B940363d278c81D6402fA89E79Ab4ebc); // sanko-mainnet
        _registerDVN("TSS", 30214, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // scroll-mainnet
        _registerDVN("TSS", 30280, 0xd5C9DFDE96aA0731b3224f8bacf00Cd456188542); // sei-mainnet
        _registerDVN("TSS", 30230, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // shimmer-mainnet
        _registerDVN("TSS", 30148, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // shrapnel-mainnet
        _registerDVN("TSS", 30340, 0xf80cB5F7467B67cBEC77DcE6a13C89f210b554c0); // soneium-mainnet
        _registerDVN("TSS", 30332, 0x01BBb6319c596e70342a0cFD1193CfebE10BBB1D); // sonic-mainnet
        _registerDVN("TSS", 30364, 0xe25741bda30bb79a66ADf656E7f2D3f0C4fb3191); // story-mainnet
        _registerDVN("TSS", 30327, 0xaeA4FB2C28252C8e5f195178820E8791Aa4A4e41); // superposition-mainnet
        _registerDVN("TSS", 30335, 0x275448a4BF72Ab5A560e8A535AAC0c85B99bC896); // swell-mainnet
        _registerDVN("TSS", 30290, 0x0EC3Aa6352A0BFA3352523938260e42c212fa8E7); // taiko-mainnet
        _registerDVN("TSS", 30199, 0x4514FC667a944752ee8A29F544c1B20b1A315f25); // telos-mainnet
        _registerDVN("TSS", 30173, 0x282b3386571f7f794450d5789911a9804FA346b4); // tenet-mainnet
        _registerDVN("TSS", 30238, 0xfDfA2330713A8e2EaC6e4f15918F11937fFA4dBE); // tiltyard-mainnet
        _registerDVN("TSS", 30196, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // tomo-mainnet
        _registerDVN("TSS", 30320, 0x306B9a8953B9462F8b826e6768a93C8EA7454965); // unichain-mainnet
        _registerDVN("TSS", 30236, 0x4b80F7d25c451D204b1C93D9bdf2aB3B04f3EA4a); // xai-mainnet
        _registerDVN("TSS", 30274, 0x4b80F7d25c451D204b1C93D9bdf2aB3B04f3EA4a); // xlayer-mainnet
        _registerDVN("TSS", 30216, 0xC39161c743D0307EB9BCc9FEF03eeb9Dc4802de7); // xpla-mainnet
        _registerDVN("TSS", 30303, 0x4b80F7d25c451D204b1C93D9bdf2aB3B04f3EA4a); // zircuit-mainnet
        _registerDVN("TSS", 30183, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // zkconsensys-mainnet
        _registerDVN("TSS", 30301, 0x8b82a8DE13aF4bdAC68134494d83A7Aacc113165); // zklink-mainnet
        _registerDVN("TSS", 30158, 0xA6Bf2bE6c60175601BF88217c75dD4b14ABB5FBb); // zkpolygon-mainnet
        _registerDVN("TSS", 30165, 0xCB7aD38D45ab5bcF5880B0fa851263C29582c18a); // zksync-mainnet
        _registerDVN("TSS", 30195, 0xcb566e3B6934Fa77258d68ea18E931fa75e1aaAa); // zora-mainnet

        // Tapioca
        _registerDVN("Tapioca", 30110, 0xabC9b1819cc4D9846550F928B985993cF6240439); // arbitrum-mainnet
        _registerDVN("Tapioca", 30106, 0xD24972c11F91c1bB9eaEe97ec96bB9c33cF7af24); // avalanche-mainnet
        _registerDVN("Tapioca", 30101, 0xD24972c11F91c1bB9eaEe97ec96bB9c33cF7af24); // ethereum-mainnet
        _registerDVN("Tapioca", 30111, 0xabC9b1819cc4D9846550F928B985993cF6240439); // optimism-mainnet

        // USDT0
        _registerDVN("USDT0", 30110, 0xa8b8575fA41c305953F519C7D288cd7741727C7b); // arbitrum-mainnet
        _registerDVN("USDT0", 30106, 0x375C76c6E8ec55A358e6A8c72fe233d0D4a96bEE); // avalanche-mainnet
        _registerDVN("USDT0", 30362, 0xd01ae6905d48315f7bE10C7330aeCF8360Ef5b12); // bera-mainnet
        _registerDVN("USDT0", 30102, 0x72F697797aC173F09eDa73Dd9C11a141376d2b57); // bsc-mainnet
        _registerDVN("USDT0", 30125, 0x18f76f0d8CCD176BbE59B3870fa486d1Fff87026); // celo-mainnet
        _registerDVN("USDT0", 30212, 0x38179D3BFa6Ef1d69A8A7b0b671BA3D8836B2AE8); // conflux-mainnet
        _registerDVN("USDT0", 30101, 0x3b0531eB02Ab4aD72e7a531180beeF9493a00dD2); // ethereum-mainnet
        _registerDVN("USDT0", 30295, 0x7cEc38c06a2FEC9Fd525B1925544110204CbB5f6); // flare-mainnet
        _registerDVN("USDT0", 30316, 0x9d5D4983C4ed9253E920Aa82bE9436F1FbeAe0c0); // hedera-mainnet
        _registerDVN("USDT0", 30367, 0xaE016a939935D6fe6185900d4c7C7C9b27366caC); // hyperliquid-mainnet
        _registerDVN("USDT0", 30339, 0xDF44a1594d3D516f7CDFb4DC275a79a5F6e3Db1d); // ink-mainnet
        _registerDVN("USDT0", 30181, 0xb832B06aB57da86AfBD6a1AF9857703D548fb37d); // mantle-mainnet
        _registerDVN("USDT0", 30390, 0x2DCbD79F38D6871232422db88EC29e8D97135Ac7); // monad-mainnet
        _registerDVN("USDT0", 30331, 0x91f93749e44C7b6510dcD236aeADE39dFc901D49); // mp1-mainnet
        _registerDVN("USDT0", 30111, 0x947Bb89919d1E5996d6C46223626AC2E97BcF8A3); // optimism-mainnet
        _registerDVN("USDT0", 30383, 0xdCdd4628F858b45260C31D6ad076bD2C3D3c2f73); // plasma-mainnet
        _registerDVN("USDT0", 30109, 0xdD3d71FF05D9206C69c667D22b3a0970009780e4); // polygon-mainnet
        _registerDVN("USDT0", 30333, 0xBABbb709b3CefE563f2aB14898a53301686D48b9); // rootstock-mainnet
        _registerDVN("USDT0", 30280, 0x8EfB6b7dC61C6B6638714747d5E6B81a3512b5C3); // sei-mainnet
        _registerDVN("USDT0", 30396, 0x8D77D35604A9f37f488E41D1d916b2A0088F82Dd); // stable-mainnet
        _registerDVN("USDT0", 30320, 0x208894346d2995A26493f8C2a5b04fB1ee41A899); // unichain-mainnet
        _registerDVN("USDT0", 30274, 0x6De0D56e2d695dB9E2B4FBEcA3D81372c59848BB); // xlayer-mainnet

        // Ubisoft
        _registerDVN("Ubisoft", 30324, 0x62aA89bAd332788021F6F4F4Fb196D5Fe59C27a6); // abstract-mainnet
        _registerDVN("Ubisoft", 30110, 0x77aAF86B4466A67869667BaBe02c6Ebe7E7791d6); // arbitrum-mainnet
        _registerDVN("Ubisoft", 30106, 0xc86D023C11c5e8aA731F40b65158590624d242aF); // avalanche-mainnet
        _registerDVN("Ubisoft", 30184, 0x505E855810cb243b2f23e95fdbb8F28d22F87a8C); // base-mainnet
        _registerDVN("Ubisoft", 30101, 0xcc9da5B157eD87e24A9cF562E6A7C05D3C3deCD3); // ethereum-mainnet
        _registerDVN("Ubisoft", 30265, 0x60aF1C48AbD2F6013e06269292a77B3285e984b9); // homeverse-mainnet
        _registerDVN("Ubisoft", 30111, 0x51ACCFB59e4BDA0F8324934f9789e9Ea19fBEff4); // optimism-mainnet
        _registerDVN("Ubisoft", 30216, 0xa51cE237FaFA3052D5d3308Df38A024724Bb1274); // xpla-mainnet
        _registerDVN("Ubisoft", 30158, 0xD074B6bbCBEC2f2B4c4265DE3D95e521f82bF669); // zkpolygon-mainnet
        _registerDVN("Ubisoft", 30165, 0x02A7Bf7298D17C12181578fF474c17C922aAd75A); // zksync-mainnet

        // WBTC Canary
        _registerDVN("WBTC Canary", 30110, 0xeCB3ac94976F11a53ae74Af1f36FB89E247FAEEF); // arbitrum-mainnet
        _registerDVN("WBTC Canary", 30106, 0x6995acD4AE604f8f334F5309A75232544F78E0c9); // avalanche-mainnet
        _registerDVN("WBTC Canary", 30184, 0x4873d56816F45eF341a8819d7039E4746Ed77C21); // base-mainnet
        _registerDVN("WBTC Canary", 30362, 0x575d0De08426223896d9Cd4bBaF4C02C9a7Dc8c6); // bera-mainnet
        _registerDVN("WBTC Canary", 30279, 0x8bAFe0299CB4D3fF75d3f7045554474Bf414FD11); // bob-mainnet
        _registerDVN("WBTC Canary", 30102, 0xd29dCf66E264aA7d6833BdaC6b9279791a7c246B); // bsc-mainnet
        _registerDVN("WBTC Canary", 30101, 0x89CA15937e1033AF26Fe4C5e976216E8C8179408); // ethereum-mainnet
        _registerDVN("WBTC Canary", 30336, 0xbbDc8C15936e5ce33FFBcAF1Aba2A8F17e31eFB5); // flow-mainnet
        _registerDVN("WBTC Canary", 30316, 0x8EfB6b7dC61C6B6638714747d5E6B81a3512b5C3); // hedera-mainnet
        _registerDVN("WBTC Canary", 30382, 0xDd7B5E1dB4AaFd5C8EC3b764eFB8ed265Aa5445B); // humanity-mainnet
        _registerDVN("WBTC Canary", 30390, 0x6398E91001Cc1682bBA103E6B2489Fa5675a5a64); // monad-mainnet
        _registerDVN("WBTC Canary", 30388, 0xE40D78243074711E21cA5290eE190062BdCe09B5); // og-mainnet
        _registerDVN("WBTC Canary", 30202, 0xeFdC8b6E2757118dE5990260D2A4eF39d31f9e49); // opbnb-mainnet
        _registerDVN("WBTC Canary", 30111, 0x6F798D30577c91E8F9291e82e633Dbe4dCe16b93); // optimism-mainnet
        _registerDVN("WBTC Canary", 30383, 0x3b65E87E2A4690f14cae0483014259DeD8215adc); // plasma-mainnet
        _registerDVN("WBTC Canary", 30280, 0xf2e89Ed7E342c708BA8CD79b293AD9244f5FCcb3); // sei-mainnet
        _registerDVN("WBTC Canary", 30340, 0x1176B42A5c76b41e0895705af028ff8A75c08156); // soneium-mainnet
        _registerDVN("WBTC Canary", 30332, 0x87a4d47918e83Df0fcF6040dBdC358119f7deb2a); // sonic-mainnet
        _registerDVN("WBTC Canary", 30335, 0x4d09C99ED8788b191144D6Cdd129014FDe70326f); // swell-mainnet
        _registerDVN("WBTC Canary", 30199, 0xd2750419b4a663c8Ff8f7B6067885D82f299aCe9); // telos-mainnet
        _registerDVN("WBTC Canary", 30320, 0x148aE5e1df44Cf8b6D258430Eab79b28d0da4Aa6); // unichain-mainnet

        // Wyoming
        _registerDVN("Wyoming", 30110, 0xcB1b1d524D013a32E976A5963bd541C388Ec0517); // arbitrum-mainnet
        _registerDVN("Wyoming", 30106, 0xDa4428ff0f15B9D92C39aE08c4fc2F1216662c2f); // avalanche-mainnet
        _registerDVN("Wyoming", 30184, 0xF80285eFb7518d5c79F4e98E3bAA59dA5eE79621); // base-mainnet
        _registerDVN("Wyoming", 30101, 0x6C70Db9CE65fA37499C1f1A150A6440fC9c7273A); // ethereum-mainnet
        _registerDVN("Wyoming", 30111, 0x94eC5934Daa761d7597B76fD0fecf8385De143be); // optimism-mainnet
        _registerDVN("Wyoming", 30109, 0xf6cB110b0334825797B9b733060229C68e5D8bef); // polygon-mainnet

        // Zeeve
        _registerDVN("Zeeve", 30110, 0x7151c89f6ba70d6aB845289E4ec706530FfAF3CB); // arbitrum-mainnet
        _registerDVN("Zeeve", 30106, 0x65c41255c7f49A4B728676A0Ede4a1329Ff6911A); // avalanche-mainnet
        _registerDVN("Zeeve", 30184, 0x6b34E842175C701F488E2Dd335A98caAEc49593F); // base-mainnet
        _registerDVN("Zeeve", 30102, 0x92453Ce02159F774f1c846a4A0693008ED020F59); // bsc-mainnet
        _registerDVN("Zeeve", 30101, 0x02041731CB8CBAe90838BB8632c359fC0c2b0775); // ethereum-mainnet
        _registerDVN("Zeeve", 30292, 0x1e023Ed98a1236FB30054bA1317bB82c3C37785F); // etherlink-mainnet
        _registerDVN("Zeeve", 30111, 0x4873d56816F45eF341a8819d7039E4746Ed77C21); // optimism-mainnet
    }
    
    /// @notice Register chain name to EID mappings
    function _registerChainMappings() private {
        _chainNameToEid["abstract-mainnet"] = 30324;
        _eidToChainName[30324] = "abstract-mainnet";
        _chainNameToEid["animechain-mainnet"] = 30372;
        _eidToChainName[30372] = "animechain-mainnet";
        _chainNameToEid["ape-mainnet"] = 30312;
        _eidToChainName[30312] = "ape-mainnet";
        _chainNameToEid["apexfusionnexus-mainnet"] = 30384;
        _eidToChainName[30384] = "apexfusionnexus-mainnet";
        _chainNameToEid["arbitrum-mainnet"] = 30110;
        _eidToChainName[30110] = "arbitrum-mainnet";
        _chainNameToEid["astar-mainnet"] = 30210;
        _eidToChainName[30210] = "astar-mainnet";
        _chainNameToEid["aurora-mainnet"] = 30211;
        _eidToChainName[30211] = "aurora-mainnet";
        _chainNameToEid["avalanche-mainnet"] = 30106;
        _eidToChainName[30106] = "avalanche-mainnet";
        _chainNameToEid["bahamut-mainnet"] = 30363;
        _eidToChainName[30363] = "bahamut-mainnet";
        _chainNameToEid["base-mainnet"] = 30184;
        _eidToChainName[30184] = "base-mainnet";
        _chainNameToEid["bb1-mainnet"] = 30234;
        _eidToChainName[30234] = "bb1-mainnet";
        _chainNameToEid["bera-mainnet"] = 30362;
        _eidToChainName[30362] = "bera-mainnet";
        _chainNameToEid["bevm-mainnet"] = 30317;
        _eidToChainName[30317] = "bevm-mainnet";
        _chainNameToEid["bitlayer-mainnet"] = 30314;
        _eidToChainName[30314] = "bitlayer-mainnet";
        _chainNameToEid["blast-mainnet"] = 30243;
        _eidToChainName[30243] = "blast-mainnet";
        _chainNameToEid["bob-mainnet"] = 30279;
        _eidToChainName[30279] = "bob-mainnet";
        _chainNameToEid["botanix-mainnet"] = 30376;
        _eidToChainName[30376] = "botanix-mainnet";
        _chainNameToEid["bouncebit-mainnet"] = 30293;
        _eidToChainName[30293] = "bouncebit-mainnet";
        _chainNameToEid["bsc-mainnet"] = 30102;
        _eidToChainName[30102] = "bsc-mainnet";
        _chainNameToEid["camp-mainnet"] = 30381;
        _eidToChainName[30381] = "camp-mainnet";
        _chainNameToEid["canto-mainnet"] = 30159;
        _eidToChainName[30159] = "canto-mainnet";
        _chainNameToEid["celo-mainnet"] = 30125;
        _eidToChainName[30125] = "celo-mainnet";
        _chainNameToEid["codex-mainnet"] = 30323;
        _eidToChainName[30323] = "codex-mainnet";
        _chainNameToEid["concrete-mainnet"] = 30366;
        _eidToChainName[30366] = "concrete-mainnet";
        _chainNameToEid["conflux-mainnet"] = 30212;
        _eidToChainName[30212] = "conflux-mainnet";
        _chainNameToEid["converge-mainnet"] = 30400;
        _eidToChainName[30400] = "converge-mainnet";
        _chainNameToEid["coredao-mainnet"] = 30153;
        _eidToChainName[30153] = "coredao-mainnet";
        _chainNameToEid["cronosevm-mainnet"] = 30359;
        _eidToChainName[30359] = "cronosevm-mainnet";
        _chainNameToEid["cronoszkevm-mainnet"] = 30360;
        _eidToChainName[30360] = "cronoszkevm-mainnet";
        _chainNameToEid["cyber-mainnet"] = 30283;
        _eidToChainName[30283] = "cyber-mainnet";
        _chainNameToEid["degen-mainnet"] = 30267;
        _eidToChainName[30267] = "degen-mainnet";
        _chainNameToEid["dexalot-mainnet"] = 30118;
        _eidToChainName[30118] = "dexalot-mainnet";
        _chainNameToEid["dfk-mainnet"] = 30115;
        _eidToChainName[30115] = "dfk-mainnet";
        _chainNameToEid["dinari-mainnet"] = 30385;
        _eidToChainName[30385] = "dinari-mainnet";
        _chainNameToEid["dm2verse-mainnet"] = 30315;
        _eidToChainName[30315] = "dm2verse-mainnet";
        _chainNameToEid["doma-mainnet"] = 30393;
        _eidToChainName[30393] = "doma-mainnet";
        _chainNameToEid["dos-mainnet"] = 30149;
        _eidToChainName[30149] = "dos-mainnet";
        _chainNameToEid["ebi-mainnet"] = 30282;
        _eidToChainName[30282] = "ebi-mainnet";
        _chainNameToEid["edu-mainnet"] = 30328;
        _eidToChainName[30328] = "edu-mainnet";
        _chainNameToEid["eon-mainnet"] = 30215;
        _eidToChainName[30215] = "eon-mainnet";
        _chainNameToEid["ethereal-mainnet"] = 30391;
        _eidToChainName[30391] = "ethereal-mainnet";
        _chainNameToEid["ethereum-mainnet"] = 30101;
        _eidToChainName[30101] = "ethereum-mainnet";
        _chainNameToEid["etherlink-mainnet"] = 30292;
        _eidToChainName[30292] = "etherlink-mainnet";
        _chainNameToEid["fantom-mainnet"] = 30112;
        _eidToChainName[30112] = "fantom-mainnet";
        _chainNameToEid["flare-mainnet"] = 30295;
        _eidToChainName[30295] = "flare-mainnet";
        _chainNameToEid["flow-mainnet"] = 30336;
        _eidToChainName[30336] = "flow-mainnet";
        _chainNameToEid["fraxtal-mainnet"] = 30255;
        _eidToChainName[30255] = "fraxtal-mainnet";
        _chainNameToEid["fuse-mainnet"] = 30138;
        _eidToChainName[30138] = "fuse-mainnet";
        _chainNameToEid["gatelayer-mainnet"] = 30389;
        _eidToChainName[30389] = "gatelayer-mainnet";
        _chainNameToEid["glue-mainnet"] = 30342;
        _eidToChainName[30342] = "glue-mainnet";
        _chainNameToEid["gnosis-mainnet"] = 30145;
        _eidToChainName[30145] = "gnosis-mainnet";
        _chainNameToEid["goat-mainnet"] = 30361;
        _eidToChainName[30361] = "goat-mainnet";
        _chainNameToEid["gravity-mainnet"] = 30294;
        _eidToChainName[30294] = "gravity-mainnet";
        _chainNameToEid["gunz-mainnet"] = 30371;
        _eidToChainName[30371] = "gunz-mainnet";
        _chainNameToEid["harmony-mainnet"] = 30116;
        _eidToChainName[30116] = "harmony-mainnet";
        _chainNameToEid["hedera-mainnet"] = 30316;
        _eidToChainName[30316] = "hedera-mainnet";
        _chainNameToEid["hemi-mainnet"] = 30329;
        _eidToChainName[30329] = "hemi-mainnet";
        _chainNameToEid["homeverse-mainnet"] = 30265;
        _eidToChainName[30265] = "homeverse-mainnet";
        _chainNameToEid["hubble-mainnet"] = 30182;
        _eidToChainName[30182] = "hubble-mainnet";
        _chainNameToEid["humanity-mainnet"] = 30382;
        _eidToChainName[30382] = "humanity-mainnet";
        _chainNameToEid["hyperliquid-mainnet"] = 30367;
        _eidToChainName[30367] = "hyperliquid-mainnet";
        _chainNameToEid["injectiveevm-mainnet"] = 30394;
        _eidToChainName[30394] = "injectiveevm-mainnet";
        _chainNameToEid["ink-mainnet"] = 30339;
        _eidToChainName[30339] = "ink-mainnet";
        _chainNameToEid["iota-mainnet"] = 30284;
        _eidToChainName[30284] = "iota-mainnet";
        _chainNameToEid["islander-mainnet"] = 30330;
        _eidToChainName[30330] = "islander-mainnet";
        _chainNameToEid["joc-mainnet"] = 30285;
        _eidToChainName[30285] = "joc-mainnet";
        _chainNameToEid["katana-mainnet"] = 30375;
        _eidToChainName[30375] = "katana-mainnet";
        _chainNameToEid["kava-mainnet"] = 30177;
        _eidToChainName[30177] = "kava-mainnet";
        _chainNameToEid["klaytn-mainnet"] = 30150;
        _eidToChainName[30150] = "klaytn-mainnet";
        _chainNameToEid["lens-mainnet"] = 30373;
        _eidToChainName[30373] = "lens-mainnet";
        _chainNameToEid["lightlink-mainnet"] = 30309;
        _eidToChainName[30309] = "lightlink-mainnet";
        _chainNameToEid["lisk-mainnet"] = 30321;
        _eidToChainName[30321] = "lisk-mainnet";
        _chainNameToEid["loot-mainnet"] = 30197;
        _eidToChainName[30197] = "loot-mainnet";
        _chainNameToEid["lyra-mainnet"] = 30311;
        _eidToChainName[30311] = "lyra-mainnet";
        _chainNameToEid["manta-mainnet"] = 30217;
        _eidToChainName[30217] = "manta-mainnet";
        _chainNameToEid["mantle-mainnet"] = 30181;
        _eidToChainName[30181] = "mantle-mainnet";
        _chainNameToEid["masa-mainnet"] = 30263;
        _eidToChainName[30263] = "masa-mainnet";
        _chainNameToEid["meritcircle-mainnet"] = 30198;
        _eidToChainName[30198] = "meritcircle-mainnet";
        _chainNameToEid["merlin-mainnet"] = 30266;
        _eidToChainName[30266] = "merlin-mainnet";
        _chainNameToEid["meter-mainnet"] = 30176;
        _eidToChainName[30176] = "meter-mainnet";
        _chainNameToEid["metis-mainnet"] = 30151;
        _eidToChainName[30151] = "metis-mainnet";
        _chainNameToEid["mode-mainnet"] = 30260;
        _eidToChainName[30260] = "mode-mainnet";
        _chainNameToEid["monad-mainnet"] = 30390;
        _eidToChainName[30390] = "monad-mainnet";
        _chainNameToEid["moonbeam-mainnet"] = 30126;
        _eidToChainName[30126] = "moonbeam-mainnet";
        _chainNameToEid["moonriver-mainnet"] = 30167;
        _eidToChainName[30167] = "moonriver-mainnet";
        _chainNameToEid["morph-mainnet"] = 30322;
        _eidToChainName[30322] = "morph-mainnet";
        _chainNameToEid["mp1-mainnet"] = 30331;
        _eidToChainName[30331] = "mp1-mainnet";
        _chainNameToEid["nexera-mainnet"] = 30395;
        _eidToChainName[30395] = "nexera-mainnet";
        _chainNameToEid["nibiru-mainnet"] = 30369;
        _eidToChainName[30369] = "nibiru-mainnet";
        _chainNameToEid["nova-mainnet"] = 30175;
        _eidToChainName[30175] = "nova-mainnet";
        _chainNameToEid["og-mainnet"] = 30388;
        _eidToChainName[30388] = "og-mainnet";
        _chainNameToEid["okx-mainnet"] = 30155;
        _eidToChainName[30155] = "okx-mainnet";
        _chainNameToEid["opbnb-mainnet"] = 30202;
        _eidToChainName[30202] = "opbnb-mainnet";
        _chainNameToEid["openledger-mainnet"] = 30392;
        _eidToChainName[30392] = "openledger-mainnet";
        _chainNameToEid["optimism-mainnet"] = 30111;
        _eidToChainName[30111] = "optimism-mainnet";
        _chainNameToEid["orderly-mainnet"] = 30213;
        _eidToChainName[30213] = "orderly-mainnet";
        _chainNameToEid["peaq-mainnet"] = 30302;
        _eidToChainName[30302] = "peaq-mainnet";
        _chainNameToEid["pgn-mainnet"] = 30218;
        _eidToChainName[30218] = "pgn-mainnet";
        _chainNameToEid["plasma-mainnet"] = 30383;
        _eidToChainName[30383] = "plasma-mainnet";
        _chainNameToEid["plume-mainnet"] = 30318;
        _eidToChainName[30318] = "plume-mainnet";
        _chainNameToEid["plumephoenix-mainnet"] = 30370;
        _eidToChainName[30370] = "plumephoenix-mainnet";
        _chainNameToEid["polygon-mainnet"] = 30109;
        _eidToChainName[30109] = "polygon-mainnet";
        _chainNameToEid["rarible-mainnet"] = 30235;
        _eidToChainName[30235] = "rarible-mainnet";
        _chainNameToEid["real-mainnet"] = 30237;
        _eidToChainName[30237] = "real-mainnet";
        _chainNameToEid["reya-mainnet"] = 30313;
        _eidToChainName[30313] = "reya-mainnet";
        _chainNameToEid["rootstock-mainnet"] = 30333;
        _eidToChainName[30333] = "rootstock-mainnet";
        _chainNameToEid["sanko-mainnet"] = 30278;
        _eidToChainName[30278] = "sanko-mainnet";
        _chainNameToEid["scroll-mainnet"] = 30214;
        _eidToChainName[30214] = "scroll-mainnet";
        _chainNameToEid["sei-mainnet"] = 30280;
        _eidToChainName[30280] = "sei-mainnet";
        _chainNameToEid["shimmer-mainnet"] = 30230;
        _eidToChainName[30230] = "shimmer-mainnet";
        _chainNameToEid["shrapnel-mainnet"] = 30148;
        _eidToChainName[30148] = "shrapnel-mainnet";
        _chainNameToEid["silicon-mainnet"] = 30379;
        _eidToChainName[30379] = "silicon-mainnet";
        _chainNameToEid["skale-mainnet"] = 30273;
        _eidToChainName[30273] = "skale-mainnet";
        _chainNameToEid["somnia-mainnet"] = 30380;
        _eidToChainName[30380] = "somnia-mainnet";
        _chainNameToEid["soneium-mainnet"] = 30340;
        _eidToChainName[30340] = "soneium-mainnet";
        _chainNameToEid["sonic-mainnet"] = 30332;
        _eidToChainName[30332] = "sonic-mainnet";
        _chainNameToEid["sophon-mainnet"] = 30334;
        _eidToChainName[30334] = "sophon-mainnet";
        _chainNameToEid["space-mainnet"] = 30341;
        _eidToChainName[30341] = "space-mainnet";
        _chainNameToEid["stable-mainnet"] = 30396;
        _eidToChainName[30396] = "stable-mainnet";
        _chainNameToEid["story-mainnet"] = 30364;
        _eidToChainName[30364] = "story-mainnet";
        _chainNameToEid["subtensorevm-mainnet"] = 30374;
        _eidToChainName[30374] = "subtensorevm-mainnet";
        _chainNameToEid["superposition-mainnet"] = 30327;
        _eidToChainName[30327] = "superposition-mainnet";
        _chainNameToEid["swell-mainnet"] = 30335;
        _eidToChainName[30335] = "swell-mainnet";
        _chainNameToEid["tac-mainnet"] = 30377;
        _eidToChainName[30377] = "tac-mainnet";
        _chainNameToEid["taiko-mainnet"] = 30290;
        _eidToChainName[30290] = "taiko-mainnet";
        _chainNameToEid["telos-mainnet"] = 30199;
        _eidToChainName[30199] = "telos-mainnet";
        _chainNameToEid["tenet-mainnet"] = 30173;
        _eidToChainName[30173] = "tenet-mainnet";
        _chainNameToEid["tiltyard-mainnet"] = 30238;
        _eidToChainName[30238] = "tiltyard-mainnet";
        _chainNameToEid["tomo-mainnet"] = 30196;
        _eidToChainName[30196] = "tomo-mainnet";
        _chainNameToEid["unichain-mainnet"] = 30320;
        _eidToChainName[30320] = "unichain-mainnet";
        _chainNameToEid["worldchain-mainnet"] = 30319;
        _eidToChainName[30319] = "worldchain-mainnet";
        _chainNameToEid["xai-mainnet"] = 30236;
        _eidToChainName[30236] = "xai-mainnet";
        _chainNameToEid["xchain-mainnet"] = 30291;
        _eidToChainName[30291] = "xchain-mainnet";
        _chainNameToEid["xdc-mainnet"] = 30365;
        _eidToChainName[30365] = "xdc-mainnet";
        _chainNameToEid["xlayer-mainnet"] = 30274;
        _eidToChainName[30274] = "xlayer-mainnet";
        _chainNameToEid["xpla-mainnet"] = 30216;
        _eidToChainName[30216] = "xpla-mainnet";
        _chainNameToEid["zircuit-mainnet"] = 30303;
        _eidToChainName[30303] = "zircuit-mainnet";
        _chainNameToEid["zkatana-mainnet"] = 30257;
        _eidToChainName[30257] = "zkatana-mainnet";
        _chainNameToEid["zkconsensys-mainnet"] = 30183;
        _eidToChainName[30183] = "zkconsensys-mainnet";
        _chainNameToEid["zklink-mainnet"] = 30301;
        _eidToChainName[30301] = "zklink-mainnet";
        _chainNameToEid["zkpolygon-mainnet"] = 30158;
        _eidToChainName[30158] = "zkpolygon-mainnet";
        _chainNameToEid["zksync-mainnet"] = 30165;
        _eidToChainName[30165] = "zksync-mainnet";
        _chainNameToEid["zkverify-mainnet"] = 30386;
        _eidToChainName[30386] = "zkverify-mainnet";
        _chainNameToEid["zora-mainnet"] = 30195;
        _eidToChainName[30195] = "zora-mainnet";
    }
    
    /// @notice Register a single DVN
    function _registerDVN(string memory dvnName, uint32 eid, address dvnAddress) private {
        _dvnAddresses[dvnName][eid] = dvnAddress;
        
        // Track unique DVN names
        if (!_dvnNameExists[dvnName]) {
            _dvnNameExists[dvnName] = true;
            _dvnNames.push(dvnName);
        }
        
        // Track DVNs per chain
        _dvnsByChain[eid].push(dvnName);
    }
    
    function getDVNAddress(string memory dvnName, uint32 eid) public view returns (address dvnAddress) {
        dvnAddress = _dvnAddresses[dvnName][eid];
        require(dvnAddress != address(0), string.concat("DVN not found: ", dvnName, " on chain ", _toString(eid)));
    }
    
    function getDVNAddressByChainName(string memory dvnName, string memory chainName) public view returns (address dvnAddress) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0, string.concat("Unknown chain: ", chainName));
        return getDVNAddress(dvnName, eid);
    }
    
    function dvnExists(string memory dvnName, uint32 eid) public view returns (bool exists) {
        return _dvnAddresses[dvnName][eid] != address(0);
    }
    
    function getAvailableDVNs() public view returns (string[] memory dvnNames) {
        return _dvnNames;
    }
    
    function getDVNsForChain(uint32 eid) public view returns (string[] memory names, address[] memory addresses) {
        names = _dvnsByChain[eid];
        addresses = new address[](names.length);
        
        for (uint256 i = 0; i < names.length; i++) {
            addresses[i] = _dvnAddresses[names[i]][eid];
        }
    }
    
    function getDVNsForChainByName(string memory chainName) public view returns (string[] memory names, address[] memory addresses) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0, string.concat("Unknown chain: ", chainName));
        return getDVNsForChain(eid);
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