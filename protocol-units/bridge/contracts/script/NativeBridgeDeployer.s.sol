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
    TimelockController public timelock;
    TransparentUpgradeableProxy public nativeBridgeProxy;
    string public nativeBridgeSignature = "initialize(address,address,address,address,address)";
    address public proxyAdmin;

    // TODO: all params are hardcoded for testnet deployment for now
    // Parameters
    address public moveTokenAddress = 0xC36ba8B8fD9EcbF36288b9B9B0ae9FC3E0645227;
    address public adminAddress = 0x5A368EDEbF574162B84f8ECFE48e9De4f520E087; // Replace with your .env PRIVATE_KEY address for testing
    address public relayerAddress = 0x5A368EDEbF574162B84f8ECFE48e9De4f520E087;
    address public maintainerAddress = address(0x0);
    address public timelockAddress = 0xC5B4Ca6E12144dE0e8e666F738A289476bebBc02; // mainnet: 0xA649f6335828f070dDDd7A8c4F5bef2b6FF7Bd51

    // Safe addresses (replace these with actual safe addresses)
    address public movementLabsSafe = 0x493516F6dB02c9b7f649E650c5de244646022Aa0; // mainnet: 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2
    address public movementFoundationSafe = 0x00db70A9e12537495C359581b7b3Bc3a69379A00; // mainnet: 0x074C155f09cE5fC3B65b4a9Bbb01739459C7AD63

    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;
    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER_ROLE");

    uint256 public minDelay = 2 days;

    function run() external {
        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);
        timelock = TimelockController(payable(timelockAddress));
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);

        proposers[0] = movementLabsSafe;
        executors[0] = movementFoundationSafe;

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
            timelockAddress, // Admin is the timelock
            abi.encodeWithSignature(
                nativeBridgeSignature,
                moveTokenAddress, // MOVE token address
                adminAddress, // Owner of the contract
                relayerAddress, // Owner of the contract
                maintainerAddress, // Owner of the contract
                movementLabsSafe // Insurance fund
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
                    nativeBridgeSignature, moveTokenAddress, adminAddress, adminAddress, adminAddress
                )
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + minDelay
        );
    }
}
