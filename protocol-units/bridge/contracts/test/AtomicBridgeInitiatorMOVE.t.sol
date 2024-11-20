// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiatorMOVE, IAtomicBridgeInitiatorMOVE, OwnableUpgradeable} from "../src/AtomicBridgeInitiatorMOVE.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";  

contract AtomicBridgeInitiatorMOVETest is Test {
    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorImplementation;
    MockMOVEToken public moveToken;   
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorMOVE;

    address public originator = address(1);
    bytes32 public recipient = keccak256(abi.encodePacked(address(2)));
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public constant timeLockDuration = 48 * 60 * 60; // 48 hours in seconds

    function setUp() public {
        // Deploy the MOVEToken contract and mint some tokens to the deployer
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this)); // Contract will hold initial MOVE tokens

        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.timestamp, block.prevrandao))));

        // Deploy the AtomicBridgeInitiatorMOVE contract
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiatorMOVE();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256)", 
                address(moveToken), 
                address(this), 
                timeLockDuration,
                0 ether
            )
        );

        atomicBridgeInitiatorMOVE = AtomicBridgeInitiatorMOVE(address(proxy));
    }

    function testInitiateBridgeTransferWithMove() public {
        uint256 moveAmount = 100 * 10**8;

        // Transfer moveAmount tokens to the originator and check initial balance
        moveToken.transfer(originator, moveAmount); 
        uint256 initialBalance = moveToken.balanceOf(originator);

        vm.startPrank(originator);
        moveToken.approve(address(atomicBridgeInitiatorMOVE), moveAmount);

        // Initiate the bridge transfer
        bytes32 bridgeTransferId = atomicBridgeInitiatorMOVE.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            hashLock 
        );

        // Verify the bridge transfer details
        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiatorMOVE.MessageState transferState
        ) = atomicBridgeInitiatorMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(transferAmount, moveAmount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiatorMOVE.MessageState.INITIALIZED));

        // Check the originator's MOVE balance after initiating the transfer
        uint256 finalBalance = moveToken.balanceOf(originator);
        assertEq(finalBalance, initialBalance - moveAmount);

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 secret = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(secret));
        uint256 moveAmount = 100 * 10**8; // 100 MOVEToken

        // Transfer moveAmount tokens to the originator and check initial balance
        moveToken.transfer(originator, moveAmount); 
        uint256 initialBalance = moveToken.balanceOf(originator);

        vm.startPrank(originator);
        moveToken.approve(address(atomicBridgeInitiatorMOVE), moveAmount);

        // Initiate the bridge transfer
        bytes32 bridgeTransferId = atomicBridgeInitiatorMOVE.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            testHashLock 
        );

        vm.stopPrank();

        atomicBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, secret);

        // Verify the bridge transfer details after completion
        (
            uint256 completedAmount,
            address completedOriginator,
            bytes32 completedRecipient,
            bytes32 completedHashLock,
            uint256 completedTimeLock,
            AtomicBridgeInitiatorMOVE.MessageState completedState
        ) = atomicBridgeInitiatorMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(completedAmount, moveAmount);
        assertEq(completedOriginator, originator);
        assertEq(completedRecipient, recipient);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.timestamp);
        assertEq(uint8(completedState), uint8(AtomicBridgeInitiatorMOVE.MessageState.COMPLETED));

        // Ensure no changes to the originator's balance after the transfer is completed
        uint256 finalBalance = moveToken.balanceOf(originator);
        assertEq(finalBalance, initialBalance - moveAmount);
    }

    function testRefundBridgeTransfer() public {
        uint256 moveAmount = 100 * 10**8; // 100 MOVEToken

        // Transfer moveAmount tokens to the originator and check initial balance
        moveToken.transfer(originator, moveAmount);
        uint256 initialBalance = moveToken.balanceOf(originator);

        vm.startPrank(originator);
        moveToken.approve(address(atomicBridgeInitiatorMOVE), moveAmount);

        // Initiate the bridge transfer
        bytes32 bridgeTransferId = atomicBridgeInitiatorMOVE.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            hashLock 
        );
        vm.stopPrank();

        // Advance time and block height to ensure the time lock has expired
        vm.warp(block.timestamp + timeLockDuration + 1);

        // Test that a non-owner cannot call refund
        vm.startPrank(originator);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, originator));
        atomicBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId);
        vm.stopPrank();

        // Owner refunds the transfer
        vm.expectEmit();
        emit IAtomicBridgeInitiatorMOVE.BridgeTransferRefunded(bridgeTransferId);
        atomicBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId);

        // Verify that the originator receives the refund and the balance is restored
        uint256 finalBalance = moveToken.balanceOf(originator);
        assertEq(finalBalance, initialBalance, "MOVE balance mismatch");
    }
}

