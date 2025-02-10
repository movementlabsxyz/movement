// To Deploy
// forge script NativeBridgeDeployer --fork-url https://holesky.infura.io/v3/YOUR_INFURA_PROJECT_ID --broadcast --verify --etherscan-api-key YOUR_ETHERSCAN_API_KEY
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "forge-std/Script.sol";
import {NativeBridge} from "../src/NativeBridge.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Vm} from "forge-std/Vm.sol";

contract NativeBridgeDeployer is Script {
    TransparentUpgradeableProxy public nativeBridgeProxy;
    TimelockController public timelock;
    string public nativeBridgeSignature = "initialize(address,address,address,address,uint256)";
    address public proxyAdmin;

    // TODO: all params are hardcoded for testnet deployment for now
    // Parameters
    address public moveTokenAddress = 0xC36ba8B8fD9EcbF36288b9B9B0ae9FC3E0645227;
    address public ownerAddress = 0x5b97cdf756f6363A88706c376464180E008Bd88b; // Replace with your .env PRIVATE_KEY address for testing

    // Safe addresses (replace these with actual safe addresses)
    address public movementLabsSafe = 0x493516F6dB02c9b7f649E650c5de244646022Aa0;
    address public movementFoundationSafe = 0x00db70A9e12537495C359581b7b3Bc3a69379A00;

    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;
    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER_ROLE");

    uint256 public minDelay = 2 days;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);

        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);

        proposers[0] = movementLabsSafe;
        executors[0] = movementFoundationSafe;

        // Deploy TimelockController
        timelock = new TimelockController(minDelay, proposers, executors, address(0));
        console.log("Timelock deployed at:", address(timelock));

        // Deploy NativeBridge contract
        _deployNativeBridge();

        vm.stopBroadcast();
    }

    function _deployNativeBridge() internal {
        console.log("NativeBridge: deploying");

        // Instantiate the implementation contract
        NativeBridge nativeBridgeImplementation = new NativeBridge();
        
        vm.recordLogs();
        // Deploy the TransparentUpgradeableProxy
        nativeBridgeProxy = new TransparentUpgradeableProxy(
            address(nativeBridgeImplementation),
            address(timelock), // Admin is the timelock
            abi.encodeWithSignature(
                nativeBridgeSignature,
                moveTokenAddress, // MOVE token address
                ownerAddress, // Owner of the contract
                ownerAddress, // Owner of the contract
                ownerAddress  // Owner of the contract
            )
        );

        Vm.Log[] memory logs = vm.getRecordedLogs();
        proxyAdmin = logs[logs.length - 2].emitter;
        console.log("proxy admin:", proxyAdmin);

        console.log("nativeBridgeProxy deployed at proxy address:", address(nativeBridgeProxy));
        console.log("Implementation address:", address(nativeBridgeImplementation));
    }

    function _upgradeAtomicBridge() internal {
        console.log("NativeBridge: upgrading");
        NativeBridge newBridgeImplementation = new NativeBridge();
        require(proxyAdmin != address(0), "Proxy admin not set");
        timelock.schedule(
            proxyAdmin,
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(nativeBridgeProxy),
                address(newBridgeImplementation),
                abi.encodeWithSignature(
                    nativeBridgeSignature, moveTokenAddress, ownerAddress, ownerAddress, ownerAddress
                )
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + minDelay
        );
    }
}
