// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiatorMOVE, IAtomicBridgeInitiatorMOVE, OwnableUpgradeable} from "../src/AtomicBridgeInitiatorMOVE.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";  
import {console} from "forge-std/console.sol";
import {RateLimiter} from "../src/RateLimiter.sol";

contract AtomicBridgeInitiatorMOVETest is Test {
    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorImplementation;
    MockMOVEToken public moveToken;   
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorMOVE;
    RateLimiter public rateLimiter;

    address public originator = address(1);
    bytes32 public recipient = keccak256(abi.encodePacked(address(2)));
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public constant timeLockDuration = 48 * 60 * 60; // 48 hours in seconds
    uint256 public constant riskPeriod = 24 * 60 * 60; // 24 hours in seconds
    uint256 public constant securityFund = 10 ether; // Example security fund

    function setUp() public {
        // Deploy the MOVEToken contract and mint some tokens to the deployer
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this));

        // Deploy the RateLimiter contract with owner, riskPeriod, and securityFund
        rateLimiter = new RateLimiter();
        rateLimiter.initialize(address(this), riskPeriod, securityFund);

        // Deploy the AtomicBridgeInitiatorMOVE contract with RateLimiter integration
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiatorMOVE();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256,address)", 
                address(moveToken), 
                address(this), 
                timeLockDuration,
                0 ether,
                address(rateLimiter)
            )
        );

        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(address(proxy));

        // Set up the originator with initial MOVE balance
        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.timestamp, block.prevrandao))));
        moveToken.transfer(originator, 10 ether); // Fund originator for testing
    }


    function testRateLimitExceeded() public {
        uint256 moveAmount = 6 ether; // Set to exceed the rate limit based on security fund and risk period

        // Transfer MOVE tokens to originator and approve the bridge contract
        vm.startPrank(originator);
        moveToken.approve(address(atomicBridgeInitiatorMOVE), moveAmount);

        // First transfer should succeed if under rate limit
        bytes32 bridgeTransferId1 = atomicBridgeInitiatorMOVE.initiateBridgeTransfer(moveAmount / 2, recipient, hashLock);
        
        // Second transfer should trigger the rate limit if it exceeds the allowed amount
        vm.expectRevert("RATE_LIMIT_EXCEEDED");
        atomicBridgeInitiatorMOVE.initiateBridgeTransfer(moveAmount, recipient, hashLock);

        vm.stopPrank();
    }

    function testWithinRateLimit() public {
        uint256 moveAmount = 2 ether; // Within rate limit

        // Transfer MOVE tokens to originator and approve the bridge contract
        vm.startPrank(originator);
        moveToken.approve(address(atomicBridgeInitiatorMOVE), moveAmount);

        // First transfer within limit
        bytes32 bridgeTransferId1 = atomicBridgeInitiatorMOVE.initiateBridgeTransfer(moveAmount, recipient, hashLock);

        // Verify transfer details
        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiatorMOVE.MessageState transferState
        ) = atomicBridgeInitiatorMOVE.bridgeTransfers(bridgeTransferId1);

        assertEq(transferAmount, moveAmount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiatorMOVE.MessageState.INITIALIZED));

        vm.stopPrank();
    }
}

