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
contract DefaultConfig is Script {
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
    uint64 public confirmations = 0;
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

        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000013880));

        // Movement
        configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), movementEid, movementOapp, options);
        // HyperEVM
        // configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), hyperevmEid, hyperevmOapp, options);
        // Ethereum
        // configureLZ(MOVETokenOFT(EXPECTED_MOVE_TOKEN_PROXY), ethereumEid, ethereumOapp, options);

        vm.stopBroadcast();
    }

    /**
     * @dev Configures LayerZero DVN, executor, and enforced options
     */
    function configureLZ(MOVETokenOFT move, uint32 eid, bytes32 oapp, bytes memory options) internal {

        // Configure libraries
        console.log("Setting LayerZero libraries...");
        // setLibraries(move, eid, sendUln302, receiveUln302);

        // Configure DVNs and executor
        console.log("Configuring DVNs and executor...");

        address[] memory emptyArray = new address[](0);
        UlnConfig memory ulnConfig = UlnConfig({
            confirmations: uint64(confirmations),
            requiredDVNCount: uint8(0),
            optionalDVNCount: uint8(0),
            optionalDVNThreshold: uint8(0),
            requiredDVNs: emptyArray,
            optionalDVNs: emptyArray
        });
        ExecutorConfig memory executorConfig = ExecutorConfig({maxMessageSize: 0, executor: lzExecutor});

        setConfigs(address(move), eid, sendUln302, receiveUln302, ulnConfig, executorConfig);
        console.log("DVN and executor configuration complete");

        console.log("LayerZero configuration complete");
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
     * @param _eid Endpoint ID to configure for
     * @param _sendLib Address of the send library
     * @param _receiveLib Address of the receive library
     */
    function setLibraries(MOVETokenOFT move, uint32 _eid, address _sendLib, address _receiveLib) internal {
        ILayerZeroEndpointV2(lzEndpoint).setSendLibrary(address(move), _eid, _sendLib);
        ILayerZeroEndpointV2(lzEndpoint).setReceiveLibrary(address(move), _eid, _receiveLib, 0);

         // Verify send library configuration
        address receivedSendLib = move.endpoint().getSendLibrary(address(move), _eid);
        require(receivedSendLib == sendUln302, "Send library verification failed");
        console.log("Send library verified:", receivedSendLib);

        // Verify receive library configuration
        (address receivedReceiveLib,) = move.endpoint().getReceiveLibrary(address(move), _eid);
        require(receivedReceiveLib == receiveUln302, "Receive library verification failed");
        console.log("Receive library verified:", receivedReceiveLib);
    }

    function deployTimelock(address proposer, address executor) internal returns (address) {
        require(proposer != address(0), "Proposer cannot be zero address");
        require(executor != address(0), "Executor cannot be zero address");
        return address(new TimelockController(172800, _arrayfy(proposer), _arrayfy(executor), ZERO));
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
