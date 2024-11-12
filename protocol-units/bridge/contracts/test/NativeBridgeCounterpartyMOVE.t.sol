// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {NativeBridgeCounterpartyMOVE} from "../src/NativeBridgeCounterpartyMOVE.sol";
import {NativeBridgeInitiatorMOVE} from "../src/NativeBridgeInitiatorMOVE.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";

contract NativeBridgeCounterpartyMOVETest is Test {
    NativeBridgeCounterpartyMOVE public nativeBridgeCounterpartyMOVEImplementation;
    NativeBridgeCounterpartyMOVE public nativeBridgeCounterpartyMOVE;
    NativeBridgeInitiatorMOVE public nativeBridgeInitiatorMOVEImplementation;
    NativeBridgeInitiatorMOVE public nativeBridgeInitiatorMOVE;
    MockMOVEToken public moveToken;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;

    address public deployer = address(0x1);
    address public originator = address(1);
    address public recipient = address(0x2);
    address public otherUser = address(0x3);
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 100 * 10 ** 8; // 100 MOVEToken (assuming 8 decimals)
    uint256 public timeLock = 100;
    bytes32 public initiator = keccak256(abi.encodePacked(deployer));
    bytes32 public bridgeTransferId =
        keccak256(abi.encodePacked(block.timestamp, initiator, recipient, amount, hashLock, timeLock));

    uint256 public constant COUNTERPARTY_TIME_LOCK_DURATION = 24 * 60 * 60; // 24 hours

    function setUp() public {
        // Deploy the MOVEToken contract and mint some tokens to the deployer
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this)); // Contract will hold initial MOVE tokens

        // Time lock durations
        uint256 initiatorTimeLockDuration = 48 * 60 * 60; // 48 hours for the initiator
        uint256 counterpartyTimeLockDuration = 24 * 60 * 60; // 24 hours for the counterparty

        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.timestamp, block.prevrandao))));

        // Deploy the NativeBridgeInitiator contract with a 48-hour time lock
        nativeBridgeInitiatorMOVEImplementation = new NativeBridgeInitiatorMOVE();
        proxyAdmin = new ProxyAdmin(deployer);
        proxy = new TransparentUpgradeableProxy(
            address(nativeBridgeInitiatorMOVEImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256)",
                address(moveToken),
                deployer,
                initiatorTimeLockDuration,
                0 ether // Initial pool balance
            )
        );
        nativeBridgeInitiatorMOVE = NativeBridgeInitiatorMOVE(address(proxy));

        // Deploy the NativeBridgeCounterparty contract with a 24-hour time lock
        nativeBridgeCounterpartyMOVEImplementation = new NativeBridgeCounterpartyMOVE();
        proxy = new TransparentUpgradeableProxy(
            address(nativeBridgeCounterpartyMOVEImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256)",
                address(nativeBridgeInitiatorMOVE),
                deployer,
                counterpartyTimeLockDuration
            )
        );
        nativeBridgeCounterpartyMOVE = NativeBridgeCounterpartyMOVE(address(proxy));

        // Set the counterparty contract in the NativeBridgeInitiator contract
        vm.startPrank(deployer);
        nativeBridgeInitiatorMOVE.setCounterpartyAddress(address(nativeBridgeCounterpartyMOVE));
        vm.stopPrank();
    }

    function testLockBridgeTransfer() public {
        uint256 moveAmount = 100 * 10 ** 8;
        moveToken.transfer(originator, moveAmount);
        vm.startPrank(originator);

        // Approve the NativeBridgeInitiatorMOVE contract to spend MOVEToken
        moveToken.approve(address(nativeBridgeInitiatorMOVE), amount);

        // Initiate the bridge transfer
        nativeBridgeInitiatorMOVE.initiateBridgeTransfer(amount, initiator, hashLock);

        vm.stopPrank();

        vm.startPrank(deployer); // Only the owner (deployer) can call lockBridgeTransfer
        bool result =
            nativeBridgeCounterpartyMOVE.lockBridgeTransfer(initiator, bridgeTransferId, hashLock, recipient, amount);
        vm.stopPrank();

        (
            bytes32 pendingInitiator,
            address pendingRecipient,
            uint256 pendingAmount,
            bytes32 pendingHashLock,
            uint256 pendingTimelock,
            NativeBridgeCounterpartyMOVE.MessageState pendingState
        ) = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assert(result);
        assertEq(pendingInitiator, initiator);
        assertEq(pendingRecipient, recipient);
        assertEq(pendingAmount, amount);
        assertEq(pendingHashLock, hashLock);
        assertGt(pendingTimelock, block.timestamp);
        assertEq(uint8(pendingState), uint8(NativeBridgeCounterpartyMOVE.MessageState.PENDING));
    }

    function testCompleteBridgeTransfer() public {
        bytes32 preImage = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(preImage));

        uint256 moveAmount = 100 * 10 ** 8;
        moveToken.transfer(originator, moveAmount);
        vm.startPrank(originator);

        // Approve the NativeBridgeInitiatorMOVE contract to spend MOVEToken
        moveToken.approve(address(nativeBridgeInitiatorMOVE), amount);

        // Initiate the bridge transfer
        nativeBridgeInitiatorMOVE.initiateBridgeTransfer(amount, initiator, testHashLock);

        vm.stopPrank();

        vm.startPrank(deployer); // Only the owner (deployer) can call lockBridgeTransfer
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(initiator, bridgeTransferId, testHashLock, recipient, amount);
        vm.stopPrank();

        vm.startPrank(otherUser);

        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, preImage);

        (
            bytes32 completedInitiator,
            address completedRecipient,
            uint256 completedAmount,
            bytes32 completedHashLock,
            uint256 completedTimeLock,
            NativeBridgeCounterpartyMOVE.MessageState completedState
        ) = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(completedInitiator, initiator);
        assertEq(completedRecipient, recipient);
        assertEq(completedAmount, amount);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.timestamp);
        assertEq(uint8(completedState), uint8(NativeBridgeCounterpartyMOVE.MessageState.COMPLETED));

        vm.stopPrank();
    }

    function testAbortBridgeTransfer() public {
        uint256 moveAmount = 100 * 10 ** 8;
        moveToken.transfer(originator, moveAmount);
        vm.startPrank(originator);

        // Approve the NativeBridgeInitiatorMOVE contract to spend MOVEToken
        moveToken.approve(address(nativeBridgeInitiatorMOVE), amount);

        // Initiate the bridge transfer
        nativeBridgeInitiatorMOVE.initiateBridgeTransfer(amount, initiator, hashLock);

        vm.stopPrank();

        vm.startPrank(deployer);

        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(initiator, bridgeTransferId, hashLock, recipient, amount);

        vm.stopPrank();

        // Advance the block number to beyond the timelock period
        vm.warp(block.timestamp + COUNTERPARTY_TIME_LOCK_DURATION + 1);

        // Try to abort as a malicious user (this should fail)
        //vm.startPrank(otherUser);
        //vm.expectRevert("Ownable: caller is not the owner");
        //nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId);
        //vm.stopPrank();

        // Abort as the owner (this should pass)
        vm.startPrank(deployer); // The deployer is the owner
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId);

        (
            bytes32 abortedInitiator,
            address abortedRecipient,
            uint256 abortedAmount,
            bytes32 abortedHashLock,
            uint256 abortedTimeLock,
            NativeBridgeCounterpartyMOVE.MessageState abortedState
        ) = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(abortedInitiator, initiator);
        assertEq(abortedRecipient, recipient);
        assertEq(abortedAmount, amount);
        assertEq(abortedHashLock, hashLock);
        assertLe(abortedTimeLock, block.timestamp, "Timelock is not less than or equal to current timestamp");
        assertEq(uint8(abortedState), uint8(NativeBridgeCounterpartyMOVE.MessageState.REFUNDED));

        vm.stopPrank();
    }
}
