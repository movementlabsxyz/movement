// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeCounterparty} from "../src/AtomicBridgeCounterparty.sol";
import {IWETH9} from "../src/IWETH9.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract AtomicBridgeCounterpartyTest is Test {
    AtomicBridgeCounterparty public atomicBridgeCounterparty;
    IWETH9 public weth;

    address public deployer = address(1);
    address public recipient = address(2);
    address public otherUser = address(3);
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public timeLock = 100;
    bytes32 public initiator = keccak256(abi.encodePacked(deployer));
    bytes32 public bridgeTransferId = keccak256(abi.encodePacked(block.number, initiator, recipient, amount, hashLock, timeLock));

    function setUp() public {
        // Sepolia WETH9 address
        address wethAddress = 0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14;
        weth = IWETH9(wethAddress);

        atomicBridgeCounterparty = new AtomicBridgeCounterparty();
        atomicBridgeCounterparty.initialize(wethAddress);
    }

    function testLockBridgeTransferAssets() public {
        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeCounterparty), amount);

        bool result = atomicBridgeCounterparty.lockBridgeTransferAssets(
            initiator,
            bridgeTransferId,
            hashLock,
            timeLock,
            recipient,
            amount
        );

        (
            bytes32 pendingInitiator,
            address pendingRecipient,
            uint256 pendingAmount,
            bytes32 pendingHashLock,
            uint256 pendingTimelock
        ) = atomicBridgeCounterparty.pendingTransfers(bridgeTransferId);

        assert(result);
        assertEq(pendingInitiator, initiator);
        assertEq(pendingRecipient, recipient);
        assertEq(pendingAmount, amount);
        assertEq(pendingHashLock, hashLock);
        assertGt(pendingTimelock, block.timestamp);

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 preImage = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(preImage));

        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeCounterparty), amount);

        atomicBridgeCounterparty.lockBridgeTransferAssets(
            initiator,
            bridgeTransferId,
            testHashLock,
            timeLock,
            recipient,
            amount
        );

        vm.stopPrank();
        vm.startPrank(otherUser);

        atomicBridgeCounterparty.completeBridgeTransfer(bridgeTransferId, preImage);

        (
            bytes32 completedInitiator,
            address completedRecipient,
            uint256 completedAmount,
            bytes32 completedHashLock,
            uint256 completedTimeLock 
        ) = atomicBridgeCounterparty.completedTransfers(bridgeTransferId);

        assertEq(completedInitiator, initiator);
        assertEq(completedRecipient, recipient);
        assertEq(completedAmount, amount);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.timestamp);

        vm.stopPrank();
    }

    function testAbortBridgeTransfer() public {
    vm.deal(deployer, 1 ether);
    vm.startPrank(deployer);

    weth.deposit{value: amount}();
    weth.approve(address(atomicBridgeCounterparty), amount);

    atomicBridgeCounterparty.lockBridgeTransferAssets(
        initiator,
        bridgeTransferId,
        hashLock,
        timeLock,
        recipient,
        amount
    );

    vm.stopPrank();

    // Advance the block timestamp to beyond the timelock period
    vm.warp(block.timestamp + timeLock + 1);
    vm.startPrank(deployer);

    atomicBridgeCounterparty.abortBridgeTransfer(bridgeTransferId);

    (
        bytes32 abortedInitiator,
        address abortedRecipient,
        uint256 abortedAmount,
        bytes32 abortedHashLock,
        uint256 abortedTimeLock
    ) = atomicBridgeCounterparty.abortedTransfers(bridgeTransferId);

    // Correct assertions
    assertEq(abortedInitiator, initiator);
    assertEq(abortedRecipient, recipient);
    assertEq(abortedAmount, amount);
    assertEq(abortedHashLock, hashLock);
    assertLe(abortedTimeLock, block.timestamp, "Timelock is not less than or equal to current block timestamp");

    vm.stopPrank();
}

}

