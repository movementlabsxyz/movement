// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeCounterparty} from "../src/AtomicBridgeCounterparty.sol";
import {AtomicBridgeInitiator} from "../src/AtomicBridgeInitiator.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IWETH9} from "../src/IWETH9.sol";

contract AtomicBridgeCounterpartyTest is Test {
    AtomicBridgeCounterparty public atomicBridgeCounterpartyImplementation;
    AtomicBridgeCounterparty public atomicBridgeCounterparty;
    AtomicBridgeInitiator public atomicBridgeInitiatorImplementation;
    AtomicBridgeInitiator public atomicBridgeInitiator;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    IWETH9 public weth;

    address public deployer = address(0x1);
    address public recipient = address(0x2);
    address public otherUser = address(0x3);
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    bytes32 public initiator = keccak256(abi.encodePacked(deployer));
    bytes32 public bridgeTransferId =
        keccak256(abi.encodePacked(block.number, initiator, recipient, amount, hashLock));

    uint256 public constant COUNTERPARTY_TIME_LOCK_DURATION = 24 * 60 * 60; // 24 hours

    function setUp() public {
        // Sepolia WETH9 address
        address wethAddress = 0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14;
        weth = IWETH9(wethAddress);

        // Time lock durations
        uint256 initiatorTimeLockDuration = 48 * 60 * 60; // 48 hours for the initiator
        uint256 counterpartyTimeLockDuration = 24 * 60 * 60; // 24 hours for the counterparty

        // Deploy the AtomicBridgeInitiator contract with a 48-hour time lock and initial pool balance
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiator();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256)", 
                wethAddress, 
                deployer, 
                initiatorTimeLockDuration, 
                0 ether
            )
        );

        atomicBridgeInitiator = AtomicBridgeInitiator(address(proxy));

        // Deploy the AtomicBridgeCounterparty contract with a 24-hour time lock
        atomicBridgeCounterpartyImplementation = new AtomicBridgeCounterparty();
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeCounterpartyImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256)", 
                address(atomicBridgeInitiator), 
                deployer, 
                counterpartyTimeLockDuration // Set 24-hour time lock for the counterparty
            )
        );

        atomicBridgeCounterparty = AtomicBridgeCounterparty(address(proxy));

        // Set the counterparty contract in the AtomicBridgeInitiator contract
        vm.startPrank(deployer);
        atomicBridgeInitiator.setCounterpartyAddress(address(atomicBridgeCounterparty));
        vm.stopPrank();
    }
}

