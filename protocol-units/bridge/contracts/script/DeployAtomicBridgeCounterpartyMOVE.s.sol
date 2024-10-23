// To Deploy
// forge script AtomicBridgeCounterpartyMOVEDeployer --fork-url https://holesky.infura.io/v3/YOUR_INFURA_PROJECT_ID --broadcast --verify --etherscan-api-key YOUR_ETHERSCAN_API_KEY
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "forge-std/Script.sol";
import {AtomicBridgeCounterpartyMOVE} from "../src/AtomicBridgeCounterpartyMOVE.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract AtomicBridgeCounterpartyMOVEDeployer is Script {
    TransparentUpgradeableProxy public atomicBridgeCounterpartyProxy;
    TimelockController public timelock;
    string public atomicBridgeCounterpartySignature = "initialize(address,address,uint256)";
    address public moveAdmin;

    address public atomicBridgeInitiatorAddress = address(0x5FbDB2315678afecb367f032d93F642f64180aa3);     
    address public ownerAddress = address(0x5b97cdf756f6363A88706c376464180E008Bd88b); 
    uint256 public timeLockDuration = 86400; // 24 hours in seconds (half that of the initiators)
    uint256 public minDelay = 2 days; // 2-day delay for governance timelock

    address public movementLabsSafe = address(0x493516F6dB02c9b7f649E650c5de244646022Aa0); 
    address public movementFoundationSafe = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);

    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

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

        // Deploy AtomicBridgeCounterpartyMOVE contract
        _deployAtomicBridgeCounterparty();

        vm.stopBroadcast();
    }

    function _deployAtomicBridgeCounterparty() internal {
        console.log("AtomicBridgeCounterpartyMOVE: deploying");

        // Instantiate the implementation contract
        AtomicBridgeCounterpartyMOVE atomicBridgeCounterpartyImplementation = new AtomicBridgeCounterpartyMOVE();

        // Deploy the TransparentUpgradeableProxy
        atomicBridgeCounterpartyProxy = new TransparentUpgradeableProxy(
            address(atomicBridgeCounterpartyImplementation),
            address(timelock), // Admin is the timelock
            abi.encodeWithSignature(
                atomicBridgeCounterpartySignature,
                atomicBridgeInitiatorAddress,  // AtomicBridgeInitiatorMOVE address
                ownerAddress,                  // Owner of the contract
                timeLockDuration               // Timelock duration (48 hours)
            )
        );

        console.log("AtomicBridgeCounterpartyMOVE deployed at proxy address:", address(atomicBridgeCounterpartyProxy));
        console.log("Implementation address:", address(atomicBridgeCounterpartyImplementation));
    }

    function _upgradeAtomicBridgeCounterparty() internal {
        console.log("AtomicBridgeCounterpartyMOVE: upgrading");
        AtomicBridgeCounterpartyMOVE newCounterpartyImplementation = new AtomicBridgeCounterpartyMOVE();

        timelock.schedule(
            address(moveAdmin),
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
