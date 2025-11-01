// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Script.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquid} from "../src/token/MOVETokenHyperliquid.sol";
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

interface IProxyFactory {
    function createProxyWithNonce(address masterCopy, bytes memory data, uint256 nonce)
        external
        returns (address proxy);
}

/**
 * @title DeployMOVETokenHyperliquid
 * @notice Deployment script for MOVE token on Hyperliquid with full LayerZero configuration
 * @dev Deploys implementation, proxy via CREATE3, configures LayerZero DVNs, and sets peer
 */
contract DeployMOVETokenHyperliquidTestnet is Script {
    // Deployment addresses
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    address constant ZERO = address(0x0);
    address constant MASTER_COPY_ADDRESS = 0xd9Db270c1B5E3Bd161E8c8503c55cEABeE709552;

    // Expected multisig addresses
    address constant EXPECTED_MULTISIG_LABS_OPS = 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2;
    address constant EXPECTED_MULTISIG_DEPLOYER = 0x7aE744e3b2816F660054EAbd1a1C4935DA34Ae28;
    address constant EXPECTED_MOVE_TOKEN_PROXY = 0x431a8F39106dC468DfF6aE772c8F121a0443bFA0;

    // LayerZero endpoint and libraries (Hyperliquid Mainnet)
    address public lzEndpoint = 0xf9e1815F151024bDE4B7C10BAC10e8Ba9F6b53E1;
    address public receiveUln302 = 0x012f6eaE2A0Bf5916f48b5F37C62Bcfb7C1ffdA1;
    address public sendUln302 = 0x43E505ba192aaC7BABdC1A796c87844171011684;
    address public lzExecutor = 0x72e34F44Eb09058bdDaf1aeEebDEC062f1844b00;

    // Data Verification Networks (DVNs)
    address public p2pDVN = 0x4c90F152707c6EAB6cd801E326D25b0591E449a2;
    address public lzDVN = 0x91E698871030D0e1b6c9268C20bB57E2720618Dd;

    // LayerZero config types
    uint32 public constant EXECUTOR_CONFIG_TYPE = 1;
    uint32 public constant ULN_CONFIG_TYPE = 2;
    uint32 public constant RECEIVE_CONFIG_TYPE = 2;

    // LayerZero parameters
    uint32 public movementEid = 40325;
    uint64 public confirmations = 0;
    bytes32 public movementOapp = 0xa7c56d1854c385d73b0389e6e4fe4341e7f3f5c2c6e5a81a64cff902a6b7b0f3;

    // Factory contracts
    IProxyFactory public proxyFactory = IProxyFactory(0xa6B71E26C5e0845f74c812102Ca7114b6a896AB2);
    CREATE3Factory public create3 = CREATE3Factory(0xC31CfE110880F86DcED15c067F4A2242F6d72580);

    // Deployment parameters
    bytes32 public salt = 0x6c0000000000000000000000018eddf77afc0a5c6d05a564a44fe37b068922c3;

    // Deployment state
    MOVETokenHyperliquid public moveTokenImplementation;
    address public moveTokenProxy;
    address public multisigLabsOps;
    address public multisigDeployer;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        require(vm.addr(deployerPrivateKey) == DEPLOYER_ADDRESS, "Private key does not match deployer address");
        multisigLabsOps = EXPECTED_MULTISIG_LABS_OPS;
        multisigDeployer = EXPECTED_MULTISIG_DEPLOYER;
        vm.startBroadcast(deployerPrivateKey);

        // Initialize CREATE3 factory
        // create3 = new CREATE3Factory();

        // Step 1: Deploy multisigs
        // console.log("=== Step 1: Deploying Multisigs ===");
        // deployMultisigs();

        // Step 2: Deploy MOVE token implementation and proxy
        console.log("\n=== Step 2: Deploying MOVE Token ===");
        deployMoveToken();

        // Step 3: Configure LayerZero
        console.log("\n=== Step 3: Configuring LayerZero ===");
        configureLZ();

        // Step 4: Set peer
        console.log("\n=== Step 4: Setting Peer ===");
        setPeer();

        vm.stopBroadcast();

        // Log final deployment info
        console.log("\n=== Deployment Complete ===");
        console.log("Multisig Labs Ops:", multisigLabsOps);
        console.log("Multisig Deployer:", multisigDeployer);
        console.log("MOVE Token Implementation:", address(moveTokenImplementation));
        console.log("MOVE Token Proxy:", moveTokenProxy);
    }

    /**
     * @dev Deploys Safe multisigs for Labs Ops and Deployer
     */
    function deployMultisigs() internal {
        bytes memory firstInitData =
            hex"b63e800d00000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000000000000b2105464215716e1445367bea5668f581ef7d06300000000000000000000000049f86aee2c2187870ece0e64570d0048eaf4c75100000000000000000000000012cbb2c9f072e955b6b95ad46213aaa984a4434d000000000000000000000000aff3deeb13bd2b480751189808c16e9809eebcce0000000000000000000000000eed12ca165a962cd12420dfb38407637bca42670000000000000000000000000000000000000000000000000000000000000000";
        bytes memory secondInitData =
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000b2105464215716e1445367bea5668f581ef7d0630000000000000000000000003eb69ef2dbedd5d58aa5e074131cd22d5e87ff530000000000000000000000000000000000000000000000000000000000000000";

        // multisigLabsOps = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, firstInitData, 0);
        // multisigDeployer = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, secondInitData, 0);

        require(multisigLabsOps == EXPECTED_MULTISIG_LABS_OPS, "Multisig Labs Ops address mismatch");
        require(multisigDeployer == EXPECTED_MULTISIG_DEPLOYER, "Multisig Deployer address mismatch");

        console.log("Deployed Multisig Labs Ops:", multisigLabsOps);
        console.log("Deployed Multisig Deployer:", multisigDeployer);
    }

    /**
     * @dev Deploys MOVE token implementation and proxy via CREATE3
     */
    function deployMoveToken() internal {
        // Deploy implementation
        moveTokenImplementation = new MOVETokenHyperliquid(lzEndpoint);
        console.log("Deployed Implementation:", address(moveTokenImplementation));

        // Prepare CREATE3 deployment bytecode
        bytes memory create3Bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(
                address(moveTokenImplementation),
                address(multisigLabsOps),
                abi.encodeWithSignature("initialize(address)", DEPLOYER_ADDRESS)
            )
        );

        // Deploy proxy using CREATE3 via Safe multisig
        bytes memory bytecode = abi.encodeWithSignature("deploy(bytes32,bytes)", salt, create3Bytecode);
        bytes32 digest = Safe(payable(multisigDeployer)).getTransactionHash(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), 0
        );

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(vm.envUint("PRIVATE_KEY"), digest);
        bytes memory signature = abi.encodePacked(r, s, v);
        require(signature.length >= 65, "Invalid signature length");

        moveTokenProxy = create3.getDeployed(multisigDeployer, salt);

        Safe(payable(multisigDeployer)).execTransaction(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), signature
        );

        require(moveTokenProxy == EXPECTED_MOVE_TOKEN_PROXY, "MOVE Token proxy address mismatch");

        // Verify deployment
        MOVETokenHyperliquid move = MOVETokenHyperliquid(moveTokenProxy);

        // Verify basic token properties
        require(move.decimals() == 8, "Decimals verification failed");
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

        // Verify finalizer storage slot
        address finalizer = address(uint160(uint256(vm.load(address(move), keccak256("HyperCore deployer")))));
        require(finalizer == DEPLOYER_ADDRESS, "Finalizer verification failed");
        console.log("Finalizer verified");

        console.log("All token verifications passed");
    }

    /**
     * @dev Configures LayerZero DVN, executor, and enforced options
     */
    function configureLZ() internal {
        MOVETokenHyperliquid move = MOVETokenHyperliquid(moveTokenProxy);

        // Configure libraries
        console.log("Setting LayerZero libraries...");
        setLibraries(moveTokenProxy, movementEid, sendUln302, receiveUln302);

        // Verify send library configuration
        address receivedSendLib = move.endpoint().getSendLibrary(address(move), movementEid);
        require(receivedSendLib == sendUln302, "Send library verification failed");
        console.log("Send library verified:", receivedSendLib);

        // Verify receive library configuration
        (address receivedReceiveLib,) = move.endpoint().getReceiveLibrary(address(move), movementEid);
        require(receivedReceiveLib == receiveUln302, "Receive library verification failed");
        console.log("Receive library verified:", receivedReceiveLib);

        // Configure DVNs and executor
        console.log("Configuring DVNs and executor...");
        address[] memory dvnArray = new address[](2);
        dvnArray[1] = lzDVN;
        dvnArray[0] = p2pDVN;

        address[] memory emptyArray = new address[](0);
        UlnConfig memory ulnConfig = UlnConfig({
            confirmations: uint64(confirmations),
            requiredDVNCount: uint8(dvnArray.length),
            optionalDVNCount: uint8(0),
            optionalDVNThreshold: uint8(0),
            requiredDVNs: dvnArray,
            optionalDVNs: emptyArray
        });
        ExecutorConfig memory executorConfig = ExecutorConfig({maxMessageSize: 0, executor: lzExecutor});

        setConfigs(moveTokenProxy, movementEid, sendUln302, receiveUln302, ulnConfig, executorConfig);
        console.log("DVN and executor configuration complete");

        // Set enforced options
        console.log("Setting enforced options...");
        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000013880));
        console.logBytes(options);
        EnforcedOptionParam[] memory enforcedParams = new EnforcedOptionParam[](2);
        enforcedParams[0] = EnforcedOptionParam({eid: movementEid, msgType: uint16(1), options: options});
        enforcedParams[1] = EnforcedOptionParam({eid: movementEid, msgType: uint16(2), options: options});
        move.setEnforcedOptions(enforcedParams);

        // Verify enforced options are set for both message types
        bytes memory verifyOptions1 = move.enforcedOptions(movementEid, uint16(1));
        bytes memory verifyOptions2 = move.enforcedOptions(movementEid, uint16(2));
        require(keccak256(verifyOptions1) == keccak256(options), "Enforced options verification failed for msgType 1");
        require(keccak256(verifyOptions2) == keccak256(options), "Enforced options verification failed for msgType 2");
        console.log("Enforced options verified for both message types");

        console.log("LayerZero configuration complete");
    }

    /**
     * @dev Sets the peer OApp address on the Movement network
     */
    function setPeer() internal {
        MOVETokenHyperliquid move = MOVETokenHyperliquid(moveTokenProxy);
        move.setPeer(movementEid, movementOapp);

        // Verify peer is set correctly
        bytes32 verifyPeer = move.peers(movementEid);
        require(verifyPeer == movementOapp, "Peer verification failed");
        console.log("Peer verified for Movement EID:", movementEid);
    }

    /**
     * @dev Sets LayerZero configuration parameters for send and receive libraries
     * @param contractAddress Address of the contract to configure
     * @param remoteEid Remote endpoint ID to configure for
     * @param sendLibraryAddress Address of the send library
     * @param receiveLibraryAddress Address of the receive library
     * @param ulnConfig ULN configuration with DVN settings
     * @param executorConfig Executor configuration settings
     */
    function setConfigs(
        address contractAddress,
        uint32 remoteEid,
        address sendLibraryAddress,
        address receiveLibraryAddress,
        UlnConfig memory ulnConfig,
        ExecutorConfig memory executorConfig
    ) internal {
        // Configure send library with executor and ULN configs
        SetConfigParam[] memory sendConfigParams = new SetConfigParam[](2);
        sendConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: EXECUTOR_CONFIG_TYPE, config: abi.encode(executorConfig)});
        sendConfigParams[1] =
            SetConfigParam({eid: remoteEid, configType: ULN_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        // Configure receive library with ULN config
        SetConfigParam[] memory receiveConfigParams = new SetConfigParam[](1);
        receiveConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: RECEIVE_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, sendLibraryAddress, sendConfigParams);
        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, receiveLibraryAddress, receiveConfigParams);
    }

    /**
     * @dev Sets the send and receive libraries for LayerZero messaging
     * @param _oapp Address of the OmniApp to configure
     * @param _eid Endpoint ID to configure for
     * @param _sendLib Address of the send library
     * @param _receiveLib Address of the receive library
     */
    function setLibraries(address _oapp, uint32 _eid, address _sendLib, address _receiveLib) internal {
        ILayerZeroEndpointV2(lzEndpoint).setSendLibrary(_oapp, _eid, _sendLib);
        ILayerZeroEndpointV2(lzEndpoint).setReceiveLibrary(_oapp, _eid, _receiveLib, 0);
    }
}
