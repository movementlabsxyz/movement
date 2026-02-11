// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Script.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquidV2} from "../src/token/MOVETokenHyperliquidV2.sol";
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

interface IProxyFactory {
    function createProxyWithNonce(address masterCopy, bytes memory data, uint256 nonce)
        external
        returns (address proxy);
}

/**
 * @title DeployMOVETokenHyperliquidV2
 * @notice Deployment script for MOVE token on Hyperliquid with full LayerZero configuration
 * @dev Deploys implementation, proxy via CREATE3, configures LayerZero DVNs, and sets peer
 */
contract UpgradeMOVETokenHyperliquidV2 is Script {
    // Deployment addresses
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    address constant ZERO = address(0x0);

    // LayerZero endpoint and libraries (Hyperliquid Mainnet)
    address public lzEndpoint = 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9;
    address public receiveUln302 = 0x7cacBe439EaD55fa1c22790330b12835c6884a91;
    address public sendUln302 = 0xfd76d9CB0Bac839725aB79127E7411fe71b1e3CA;
    address public lzExecutor = 0x41Bdb4aa4A63a5b2Efc531858d3118392B1A1C3d;

    // Deployment state
    MOVETokenHyperliquidV2 public moveTokenImplementation;
    address public moveTokenProxy = 0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;
    MOVETokenHyperliquidV2 public move = MOVETokenHyperliquidV2(payable(moveTokenProxy));

     // LayerZero parameters
    uint32 public movementEid = 30325;
    uint64 public confirmations = 0;
    bytes32 public movementOapp = 0x7e4fd97ef92302eea9b10f74be1d96fb1f1511cf7ed28867b0144ca89c6ebc3c;
    

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        require(vm.addr(deployerPrivateKey) == DEPLOYER_ADDRESS, "Private key does not match deployer address");

        vm.startBroadcast(deployerPrivateKey);

        // Deploy new MOVE implementation
        moveTokenImplementation = new MOVETokenHyperliquidV2(lzEndpoint);

         // Verify basic token properties
        require(move.decimals() == 8, "Decimals verification failed");
        require(move.sharedDecimals() == 6, "Decimals verification failed");
        require(keccak256(bytes(move.name())) == keccak256(bytes("Movement")), "Name verification failed");
        require(keccak256(bytes(move.symbol())) == keccak256(bytes("MOVE")), "Symbol verification failed");
        require(move.owner() == DEPLOYER_ADDRESS, "Owner verification failed");
        require(address(move.endpoint()) == lzEndpoint, "Endpoint verification failed");
        require(move.totalSupply() == 0, "Total supply should be 0");
        require(move.DOMAIN_SEPARATOR() != bytes32(0), "Domain separator should not be zero");
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

        // Verify finalizer storage slot
        address finalizer = address(uint160(uint256(vm.load(address(move), keccak256("HyperCore deployer")))));
        require(finalizer == DEPLOYER_ADDRESS, "Finalizer verification failed");

         // Verify send library configuration
        address receivedSendLib = move.endpoint().getSendLibrary(address(move), movementEid);
        require(receivedSendLib == sendUln302, "Send library verification failed");
        console.log("Send library verified:", receivedSendLib);

        // Verify receive library configuration
        (address receivedReceiveLib,) = move.endpoint().getReceiveLibrary(address(move), movementEid);
        require(receivedReceiveLib == receiveUln302, "Receive library verification failed");
        console.log("Receive library verified:", receivedReceiveLib);
        console.log("Finalizer verified");

        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000013880));
        // Verify enforced options are set for both message types
        bytes memory verifyOptions1 = move.enforcedOptions(movementEid, uint16(1));
        bytes memory verifyOptions2 = move.enforcedOptions(movementEid, uint16(2));
        require(keccak256(verifyOptions1) == keccak256(options), "Enforced options verification failed for msgType 1");
        require(keccak256(verifyOptions2) == keccak256(options), "Enforced options verification failed for msgType 2");
        console.log("Enforced options verified for both message types");

        // Verify peer is set correctly
        bytes32 verifyPeer = move.peers(movementEid);
        require(verifyPeer == movementOapp, "Peer verification failed");

        vm.stopBroadcast();
    }
}
