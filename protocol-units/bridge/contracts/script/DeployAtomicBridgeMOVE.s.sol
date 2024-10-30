// To Deploy 
// forge script AtomicBridgeInitiatorMOVEDeployer --fork-url https://holesky.infura.io/v3/YOUR_INFURA_PROJECT_ID --broadcast --verify --etherscan-api-key YOUR_ETHERSCAN_API_KEY
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "forge-std/Script.sol";
import {AtomicBridgeCounterpartyMOVE} from "../src/AtomicBridgeCounterpartyMOVE.sol";
import {AtomicBridgeInitiatorMOVE} from "../src/AtomicBridgeInitiatorMOVE.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract AtomicBridgeInitiatorMOVEDeployer is Script {
    TransparentUpgradeableProxy public atomicBridgeInitiatorProxy;
    TransparentUpgradeableProxy public atomicBridgeCounterpartyProxy;
    TimelockController public timelock;
    string public atomicBridgeInitiatorSignature = "initialize(address,address,uint256,uint256)";
    string public atomicBridgeCounterpartySignature = "initialize(address,address,uint256)";
    address public proxyAdminInitiator;
    address public proxyAdminCounterparty;


    // TODO: all params are hardcoded for testnet deployment for now
    // Parameters
    address public moveTokenAddress = 0xC36ba8B8fD9EcbF36288b9B9B0ae9FC3E0645227; 
    address public ownerAddress = 0x5b97cdf756f6363A88706c376464180E008Bd88b; 
    address public relayerAddress = 0x5b97cdf756f6363A88706c376464180E008Bd88b; 
    uint256 public timeLockInitiatorDuration = 2 days; // 48 hours in seconds
    uint256 public timeLockCounterpartyDuration = 1 days; // 24 hours in seconds (half that of the initiators)
    uint256 public minDelay = 2 days; // 2-day delay for governance timelock

    // Safe addresses (replace these with actual safe addresses)
    address public movementLabsSafe = 0x493516F6dB02c9b7f649E650c5de244646022Aa0; 
    address public movementFoundationSafe = 0x00db70A9e12537495C359581b7b3Bc3a69379A00;

    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant REFUNDER_ROLE = keccak256("REFUNDER_ROLE");
    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER_ROLE");

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

        // Deploy AtomicBridgeInitiatorMOVE contract
        _deployAtomicBridgeInitiator();
        _deployAtomicBridgeCounterparty();

        AtomicBridgeInitiatorMOVE(address(atomicBridgeInitiatorProxy)).setCounterpartyAddress(address(atomicBridgeCounterpartyProxy));
        AtomicBridgeCounterpartyMOVE(address(atomicBridgeCounterpartyProxy)).setInitiatorAddress(address(atomicBridgeInitiatorProxy));

        vm.stopBroadcast();
    }

    function _deployAtomicBridgeInitiator() internal {
        console.log("AtomicBridgeInitiatorMOVE: deploying");

        // Instantiate the implementation contract
        AtomicBridgeInitiatorMOVE atomicBridgeImplementation = new AtomicBridgeInitiatorMOVE();

        // Deploy the TransparentUpgradeableProxy
        atomicBridgeInitiatorProxy = new TransparentUpgradeableProxy(
            address(atomicBridgeImplementation),
            address(timelock), // Admin is the timelock
            abi.encodeWithSignature(
                atomicBridgeSignature,
                moveTokenAddress,  // MOVE token address
                ownerAddress,      // Owner of the contract
                timeLockInitiatorDuration  // Timelock duration (24 hours)
            )
        );

        Vm.Log[] memory logs = vm.getRecordedLogs();
        proxyAdminInitiator = logs[logs.length - 2].emitter;
        console.log("proxy admin initiator:", proxyAdminInitiator);

        console.log("AtomicBridgeInitiatorMOVE deployed at proxy address:", address(atomicBridgeProxy));
        console.log("Implementation address:", address(atomicBridgeImplementation));
    }

    function _deployAtomicBridgeCounterparty() internal {
        console.log("AtomicBridgeCounterpartyMOVE: deploying");

        // Instantiate the implementation contract
        AtomicBridgeCounterpartyMOVE atomicBridgeCounterpartyImplementation = new AtomicBridgeCounterpartyMOVE();
        
         vm.recordLogs();
        // Deploy the TransparentUpgradeableProxy
        atomicBridgeCounterpartyProxy = new TransparentUpgradeableProxy(
            address(atomicBridgeCounterpartyImplementation),
            address(timelock), // Admin is the timelock
            abi.encodeWithSignature(
                atomicBridgeCounterpartySignature,
                atomicBridgeInitiatorAddress,  // AtomicBridgeInitiatorMOVE address
                ownerAddress,                  // Owner of the contract
                relayerAddress,                // relayer of the contract
                timeLockCounterpartyDuration   // Timelock duration (48 hours)
            )
        );
        Vm.Log[] memory logs = vm.getRecordedLogs();
        proxyAdminCounterparty = logs[logs.length - 2].emitter;
        console.log("proxy admin counterparty:", proxyAdminCounterparty);

        console.log("AtomicBridgeCounterpartyMOVE deployed at proxy address:", address(atomicBridgeCounterpartyProxy));
        console.log("Implementation address:", address(atomicBridgeCounterpartyImplementation));
    }

    function _upgradeAtomicBridgeInitiator() internal {
        console.log("AtomicBridgeInitiatorMOVE: upgrading");
        AtomicBridgeInitiatorMOVE newBridgeImplementation = new AtomicBridgeInitiatorMOVE();
        require(proxyAdminInitiator != address(0), "Proxy admin not set");
        timelock.schedule(
            proxyAdminInitiator,
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(atomicBridgeProxy),
                address(newBridgeImplementation),
                abi.encodeWithSignature(
                    atomicBridgeSignature,
                    moveTokenAddress, 
                    ownerAddress, 
                    timeLockDuration
                )
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + minDelay
        );
    }

    function _upgradeAtomicBridgeCounterparty() internal {
        console.log("AtomicBridgeCounterpartyMOVE: upgrading");
        AtomicBridgeCounterpartyMOVE newCounterpartyImplementation = new AtomicBridgeCounterpartyMOVE();
        require(proxyAdminCounterparty != address(0), "Proxy admin not set");
        timelock.schedule(
            proxyAdminCounterparty,
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(atomicBridgeCounterpartyProxy),
                address(newCounterpartyImplementation),
                abi.encodeWithSignature(
                    atomicBridgeCounterpartySignature,
                    atomicBridgeInitiatorAddress, 
                    ownerAddress, 
                    timeLockDuration
                )
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + minDelay
        );
    }
}
