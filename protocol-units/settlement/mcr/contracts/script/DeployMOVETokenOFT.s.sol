// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Script.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenOFT} from "../src/token/MOVETokenOFT.sol";
import {CREATE3Factory, ICREATE3Factory} from "./helpers/Create3/CREATE3Factory.sol";

// OpenZeppelin
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

// Safe
import {Enum} from "@safe-smart-account/contracts/common/Enum.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";

// LayerZero
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {SetConfigParam} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";
import {ExecutorConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/SendLibBase.sol";
import {UlnConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/uln/UlnBase.sol";
import {EnforcedOptionParam} from "@layerzerolabs/oapp-evm/contracts/oapp/interfaces/IOAppOptionsType3.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";

// LayerZero Address Book
import {LZProtocol} from "../layerzero_book/LZProtocol.sol";
import {LZWorkers} from "../layerzero_book/LZWorkers.sol";

interface IProxyFactory {
    function createProxyWithNonce(address masterCopy, bytes memory data, uint256 nonce)
        external
        returns (address proxy);
}

/**
 * @title DeployMOVETokenOFT
 * @notice Deployment script for MOVE token on EVM chain with full LayerZero configuration
 * @dev Deploys implementation, proxy via CREATE3, configures LayerZero DVNs, and sets peer
 */
contract DeployMOVETokenOFT is Script {
    // Deployment addresses
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    address constant ZERO = address(0x0);
    address constant MASTER_COPY_ADDRESS_130 = 0xd9Db270c1B5E3Bd161E8c8503c55cEABeE709552;
    address constant MASTER_COPY_ADDRESS_141 = 0x41675C099F32341bf84BFc5382aF534df5C7461a;

    // Expected multisig addresses
    address constant EXPECTED_MULTISIG_LABS_OPS = 0x5cD71FFf9947486fD6F5c193a722191c22728887;
    address constant EXPECTED_MULTISIG_DEPLOYER = 0x7aE744e3b2816F660054EAbd1a1C4935DA34Ae28;
    address constant EXPECTED_MULTISIG_EXECUTOR = 0x443513664Eab95280360Dc1c33FDe1ad4ED5C7bB;
    address constant EXPECTED_MOVE_TOKEN_PROXY = 0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;

    // LayerZero endpoint and libraries
    address public lzEndpoint;
    address public receiveUln302;
    address public sendUln302;
    address public lzExecutor;

    // Data Verification Networks (DVNs)
    address public p2pDVN;
    address public horizenDVN;
    address public lzDVN;

    // LayerZero config types
    uint32 public constant EXECUTOR_CONFIG_TYPE = 1;
    uint32 public constant ULN_CONFIG_TYPE = 2;
    uint32 public constant RECEIVE_CONFIG_TYPE = 2;

    // LayerZero parameters
    uint32 public movementEid = 30325;
    uint32 public hyperevmEid = 30367;
    uint32 public ethereumEid = 30101;
    uint32 public baseEid = 30184;
    uint32 public avalancheEid = 30106;
    bool public defaultValues = false;
    bytes32 public movementOapp = 0x7e4fd97ef92302eea9b10f74be1d96fb1f1511cf7ed28867b0144ca89c6ebc3c;

    // Factory contracts
    IProxyFactory public proxyFactory = IProxyFactory(0xa6B71E26C5e0845f74c812102Ca7114b6a896AB2);
    CREATE3Factory public create3 = CREATE3Factory(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);

    // Deployment parameters
    bytes32 public salt = 0x6c0000000000000000000000018eddf77afc0a5c6d05a564a44fe37b068922c3;

    // Deployment state
    MOVETokenOFT public moveTokenImplementation;
    address public moveTokenProxy;
    address public multisigLabsOps;
    address public multisigExecutor;
    address public multisigDeployer;
    address public timelock;

    function run() external {

        // Load LayerZero configuration from layerzero_book/LZProtocol.sol
        LZProtocol lzProtocol = new LZProtocol();
        uint32 eid = lzProtocol.getEidByChainId(block.chainid);
        LZProtocol.ProtocolAddresses memory addresses = lzProtocol.getProtocolAddresses(eid);

        // Set LayerZero addresses from protocol
        lzEndpoint = addresses.endpointV2;
        sendUln302 = addresses.sendUln302;
        receiveUln302 = addresses.receiveUln302;
        lzExecutor = addresses.executor;

        // Load DVN addresses from LZWorkers
        LZWorkers lzWorkers = new LZWorkers();
        p2pDVN = lzWorkers.getDVNAddress("P2P", eid);
        horizenDVN = lzWorkers.getDVNAddress("Horizen", eid);
        lzDVN = lzWorkers.getDVNAddress("LayerZero Labs", eid);

        // Assert that all addresses are not zero
        require(lzEndpoint != address(0), "LayerZero endpoint cannot be zero address");
        require(sendUln302 != address(0), "Send ULN 302 cannot be zero address");
        require(receiveUln302 != address(0), "Receive ULN 302 cannot be zero address");
        require(lzExecutor != address(0), "LayerZero executor cannot be zero address");
        require(p2pDVN != address(0), "P2P DVN cannot be zero address");
        require(horizenDVN != address(0), "Horizen DVN cannot be zero address");
        require(lzDVN != address(0), "LayerZero Labs DVN cannot be zero address");

        vm.startBroadcast();

        // Step 1: Deploy multisigs
        console.log("=== Step 1: Deploying Multisigs ===");
        deployMultisigs();

        timelock = deployTimelock(multisigLabsOps, multisigExecutor);

        // Step 2: Deploy MOVE token implementation and proxy
        console.log("\n=== Step 2: Deploying MOVE Token ===");
        deployMoveToken();

        // Step 3: Configure LayerZero
        console.log("\n=== Step 3: Configuring LayerZero ===");
        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000013880));

        // Movement
        configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), movementEid, movementOapp, 15, 250000, options);
        // HyperEVM
        configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), hyperevmEid, bytes32(uint256(uint160(EXPECTED_MOVE_TOKEN_PROXY))), 15, 15, options);
        // Base
        configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), baseEid, bytes32(uint256(uint160(EXPECTED_MOVE_TOKEN_PROXY))), 15, 15, options);
        // Ethereum
        configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), ethereumEid, bytes32(uint256(uint160(EXPECTED_MOVE_TOKEN_PROXY))), 15, 15, options);
        vm.stopBroadcast();

        // Log final deployment info
        console.log("\n=== Deployment Complete ===");
        console.log("Multisig Labs Ops:", multisigLabsOps);
        console.log("Multisig Executor:", multisigExecutor);
        console.log("Multisig Deployer:", multisigDeployer);
        console.log("MOVE Token Implementation:", address(moveTokenImplementation));
        console.log("MOVE Token Proxy:", moveTokenProxy);
    }

    /**
     * @dev Deploys Safe multisigs for Labs Ops and Deployer
     */
    function deployMultisigs() internal {
        bytes memory labsOpsInitData =
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000fd0732dc9e303f09fcef3a7388ad10a83459ec9900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000070000000000000000000000008bc3c3e914ace9a0e73a3e39a669148435de8bbf0000000000000000000000006890e3e256dbc8e0684f0a6ce04c05e2a6658a0700000000000000000000000082153abfc66883acf303534b34f5e27e0ff0db72000000000000000000000000944907f1fb2afa7abea02b929df496d09567c64000000000000000000000000002f57fe96ba5c135e9a8bf82626ca49423128cca000000000000000000000000aff3deeb13bd2b480751189808c16e9809eebcce000000000000000000000000648dc72081ae39814080d3ac45ad08ff4bb864170000000000000000000000000000000000000000000000000000000000000000";
        bytes memory deployerInitData =
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000b2105464215716e1445367bea5668f581ef7d0630000000000000000000000003eb69ef2dbedd5d58aa5e074131cd22d5e87ff530000000000000000000000000000000000000000000000000000000000000000";
        bytes memory executorInitData = 
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000180000000000000000000000000fd0732dc9e303f09fcef3a7388ad10a83459ec990000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000450ba32bf877280403d556a4a40aa03be9349a4d0000000000000000000000001da2ed7e5828352bf0e188179cd9d0e15f84006b000000000000000000000000721b92ab5c162e95ad74571ff1f7695608576de70000000000000000000000000000000000000000000000000000000000000000";
        
        multisigDeployer = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS_130, deployerInitData, 0);
        require(multisigDeployer == EXPECTED_MULTISIG_DEPLOYER, "Multisig Deployer address mismatch");

        multisigLabsOps = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS_141, labsOpsInitData, 0);
        require(multisigLabsOps == EXPECTED_MULTISIG_LABS_OPS, "Multisig Labs Ops address mismatch");

        multisigExecutor = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS_141, executorInitData, 0);
        require(multisigExecutor == EXPECTED_MULTISIG_EXECUTOR, "Multisig Executor address mismatch");
    }

    /**
     * @dev Deploys MOVE token implementation and proxy via CREATE3
     */
    function deployMoveToken() internal {
        // Deploy implementation
        moveTokenImplementation = new MOVETokenOFT(lzEndpoint);
        console.log("Deployed Implementation:", address(moveTokenImplementation));
        require(timelock != address(0), "Timelock must be deployed before deploying proxy");
        // Prepare CREATE3 deployment bytecode
        bytes memory create3Bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(
                address(moveTokenImplementation),
                address(timelock),
                abi.encodeWithSignature("initialize(address,address)", EXPECTED_MULTISIG_LABS_OPS, DEPLOYER_ADDRESS)
            )
        );

        // Deploy proxy using CREATE3 via Safe multisig
        bytes memory bytecode = abi.encodeWithSignature("deploy(bytes32,bytes)", salt, create3Bytecode);
        bytes32 digest = Safe(payable(multisigDeployer)).getTransactionHash(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), 0
        );

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(digest);
        bytes memory signature = abi.encodePacked(r, s, v);
        require(signature.length >= 65, "Invalid signature length");

        moveTokenProxy = create3.getDeployed(multisigDeployer, salt);

        Safe(payable(multisigDeployer)).execTransaction(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), signature
        );

        require(moveTokenProxy == EXPECTED_MOVE_TOKEN_PROXY, "MOVE Token proxy address mismatch");

        // Verify deployment
        MOVETokenOFT move = MOVETokenOFT(moveTokenProxy);

        // Verify basic token properties
        require(move.decimals() == 8, "Decimals verification failed");
        require(move.sharedDecimals() == 8, "Decimals verification failed");
        require(keccak256(bytes(move.name())) == keccak256(bytes("Movement")), "Name verification failed");
        require(keccak256(bytes(move.symbol())) == keccak256(bytes("MOVE")), "Symbol verification failed");
        require(move.owner() == DEPLOYER_ADDRESS, "Owner verification failed");
        require(address(move.endpoint()) == lzEndpoint, "Endpoint verification failed");
        require(move.totalSupply() == 0, "Total supply should be 0");
        require(move.DOMAIN_SEPARATOR() != bytes32(0), "Domain separator should not be zero");

        console.log("Basic token properties verified");

        // Verify EIP-712 domain
        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            bytes32 domainSalt,
            uint256[] memory extensions
        ) = move.eip712Domain();

        require(fields == hex"0f", "EIP-712 fields verification failed");
        require(keccak256(bytes(name)) == keccak256(bytes("Movement")), "EIP-712 name verification failed");
        require(keccak256(bytes(version)) == keccak256(bytes("1")), "EIP-712 version verification failed");
        require(chainId == block.chainid, "EIP-712 chainId verification failed");
        require(verifyingContract == address(move), "EIP-712 verifying contract verification failed");
        require(domainSalt == bytes32(0), "EIP-712 salt verification failed");
        require(extensions.length == 0, "EIP-712 extensions verification failed");

        console.log("EIP-712 domain verified");

        console.log("All token verifications passed");
    }

    function deployTimelock(address proposer, address executor) internal returns (address) {
        require(proposer != address(0), "Proposer cannot be zero address");
        require(executor != address(0), "Executor cannot be zero address");
        return address(new TimelockController(172800, _arrayfy(proposer), _arrayfy(executor), ZERO));
    }

    /**
     * @dev Configures LayerZero DVN, executor, and enforced options
     */
    function configureLZ(MOVETokenOFT move, uint32 eid, bytes32 oapp, uint64 sendConfirmations, uint64 receiveConfirmations, bytes memory options) internal {

        // Configure libraries
        console.log("Setting LayerZero libraries...");
        setLibraries(move, eid, sendUln302, receiveUln302);

        // Configure DVNs and executor
        console.log("Configuring DVNs and executor...");
        uint256 dvnCount = 3;
        address[] memory dvns = new address[](dvnCount);
        dvns[0] = p2pDVN;
        dvns[1] = horizenDVN;
        dvns[2] = lzDVN;

        address[] memory emptyArray = new address[](0);


        UlnConfig memory sendUlnConfig = UlnConfig({
            confirmations: defaultValues ? uint64(0) : uint64(sendConfirmations),
            requiredDVNCount: defaultValues ? uint8(0) : uint8(dvnCount),
            optionalDVNCount: uint8(0),
            optionalDVNThreshold: uint8(0),
            requiredDVNs: defaultValues ? emptyArray : _sortDVNs(dvns),
            optionalDVNs: emptyArray
        });
        UlnConfig memory receiveUlnConfig = UlnConfig({
            confirmations: defaultValues ? uint64(0) : uint64(receiveConfirmations),
            requiredDVNCount: defaultValues ? uint8(0) : uint8(dvnCount),
            optionalDVNCount: uint8(0),
            optionalDVNThreshold: uint8(0),
            requiredDVNs: defaultValues ? emptyArray : _sortDVNs(dvns),
            optionalDVNs: emptyArray
        });
        ExecutorConfig memory executorConfig = ExecutorConfig({maxMessageSize: 0, executor: lzExecutor});

        setConfigs(address(move), eid, sendUln302, receiveUln302, sendUlnConfig, receiveUlnConfig, executorConfig);
        console.log("DVN and executor configuration complete");

        console.log("Setting Enforced Options");

        setEnforcedOptions(move, eid, options);

        console.log("Setting Peer");
        setPeer(move, eid, oapp);

        console.log("LayerZero configuration complete");
    }

    function setEnforcedOptions(MOVETokenOFT move, uint32 eid, bytes memory options) internal {
        // Set enforced options
        console.log("=== ENFORCED OPTIONS CONFIG ===");
        EnforcedOptionParam[] memory enforcedParams = new EnforcedOptionParam[](2);
        enforcedParams[0] = EnforcedOptionParam({eid: eid, msgType: uint16(1), options: options});
        enforcedParams[1] = EnforcedOptionParam({eid: eid, msgType: uint16(2), options: options});

        console.log("Enforced Option Params [");
        for (uint256 i = 0; i < enforcedParams.length; i++) {
            console.log("  eid:", vm.toString(enforcedParams[i].eid));
            console.log("  msgType:", vm.toString(enforcedParams[i].msgType));
            console.log("  options:", vm.toString(enforcedParams[i].options));
        }
        console.log("]");

        move.setEnforcedOptions(enforcedParams);
    }

    /**
     * @dev Sets the peer OApp address on the Movement network
     */
    function setPeer(MOVETokenOFT move, uint32 eid, bytes32 oapp) internal {
        console.log("=== PEER CONFIG ===");
        console.log("EID:", vm.toString(eid));
        console.log("Peer Address:", vm.toString(oapp));

        move.setPeer(eid, oapp);
    }


    /**
     * @dev Sets LayerZero configuration parameters for send and receive libraries
     * @param contractAddress Address of the contract to configure
     * @param remoteEid Remote endpoint ID to configure for
     * @param sendLibraryAddress Address of the send library
     * @param receiveLibraryAddress Address of the receive library
     * @param sendUlnConfig ULN configuration with DVN settings
     * @param receiveUlnConfig ULN configuration with DVN settings
     * @param executorConfig Executor configuration settings
     */
    function setConfigs(
        address contractAddress,
        uint32 remoteEid,
        address sendLibraryAddress,
        address receiveLibraryAddress,
        UlnConfig memory sendUlnConfig,
        UlnConfig memory receiveUlnConfig,
        ExecutorConfig memory executorConfig
    ) internal {
        // Configure send library with executor and ULN configs
        SetConfigParam[] memory sendConfigParams = new SetConfigParam[](2);
        sendConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: EXECUTOR_CONFIG_TYPE, config: abi.encode(executorConfig)});
        sendConfigParams[1] =
            SetConfigParam({eid: remoteEid, configType: ULN_CONFIG_TYPE, config: abi.encode(sendUlnConfig)});

        // Configure receive library with ULN config
        SetConfigParam[] memory receiveConfigParams = new SetConfigParam[](1);
        receiveConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: RECEIVE_CONFIG_TYPE, config: abi.encode(receiveUlnConfig)});

        console.log("=== ENDPOINT ===");
        console.log(lzEndpoint);
        
        console.log("=== SEND LIBRARY CONFIG ===");
        console.log("Send Library Address:", sendLibraryAddress);
        console.log("Send Config Param [");
        for (uint256 i = 0; i < sendConfigParams.length; i++) {
            console.log(vm.toString(sendConfigParams[i].eid));
            console.log(vm.toString(sendConfigParams[i].configType));
            console.log("\"", vm.toString(sendConfigParams[i].config),"\"");
        }
        console.log("]");

        console.log("=== RECEIVE LIBRARY CONFIG ===");
        console.log("Receive Library Address:", receiveLibraryAddress);
        console.log("Receive Config Param [");
        for (uint256 i = 0; i < receiveConfigParams.length; i++) {
            console.log(vm.toString(receiveConfigParams[i].eid));
            console.log(vm.toString(receiveConfigParams[i].configType));
            console.log("\"", vm.toString(receiveConfigParams[i].config),"\"");
            console.log(",");
        }
        console.log("]");

        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, sendLibraryAddress, sendConfigParams);
        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, receiveLibraryAddress, receiveConfigParams);
    }

    /**
     * @dev Sets the send and receive libraries for LayerZero messaging
     * @param _eid Endpoint ID to configure for
     * @param _sendLib Address of the send library
     * @param _receiveLib Address of the receive library
     */
    function setLibraries(MOVETokenOFT move, uint32 _eid, address _sendLib, address _receiveLib) internal {
        console.log("=== LIBRARY CONFIG ===");
        console.log("OApp Address:", address(move));
        console.log("EID:", vm.toString(_eid));
        console.log("Send Library:", _sendLib);
        console.log("Receive Library:", _receiveLib);

        ILayerZeroEndpointV2(lzEndpoint).setSendLibrary(address(move), _eid, _sendLib);
        ILayerZeroEndpointV2(lzEndpoint).setReceiveLibrary(address(move), _eid, _receiveLib, 0);
    }

    function _sortDVNs(address[] memory dvns) internal pure returns (address[] memory sortedDVNs) {
        // Simple bubble sort for demonstration; optimize as needed
        uint256 n = dvns.length;
        sortedDVNs = dvns;
        for (uint256 i = 0; i < n; i++) {
            for (uint256 j = 0; j < n - i - 1; j++) {
                if (sortedDVNs[j] > sortedDVNs[j + 1]) {
                    (sortedDVNs[j], sortedDVNs[j + 1]) = (sortedDVNs[j + 1], sortedDVNs[j]);
                }
            }
        }
    }

    function _arrayfy(address addr) internal pure returns (address[] memory arr) {
        arr = new address[](1);
        arr[0] = addr;
    }
}
