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
    bytes32 public bridgeTransferId = keccak256(abi.encodePacked(block.number, deployer, recipient, amount, hashLock, timeLock));

    function setUp() public {
        // Sepolia WETH9 address
        address wethAddress = 0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14;
        weth = IWETH9(wethAddress);

        atomicBridgeCounterparty = new AtomicBridgeCounterparty(IERC20(wethAddress));
    }

    function testLockBridgeTransferAssets() public {
        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeCounterparty), amount);

        bool result = atomicBridgeCounterparty.lockBridgeTransferAssets(
            bridgeTransferId,
            hashLock,
            timeLock,
            recipient,
            amount
        );

        (
            address initiator,
            address storedRecipient,
            uint256 storedAmount,
            bytes32 storedHashLock,
            uint256 storedTimeLock
        ) = atomicBridgeCounterparty.pendingTransfers(bridgeTransferId);

        assert(result);
        assertEq(initiator, deployer);
        assertEq(storedRecipient, recipient);
        assertEq(storedAmount, amount);
        assertEq(storedHashLock, hashLock);
        assertGt(storedTimeLock, block.timestamp);

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes memory preImage = "secret";
        bytes32 testHashLock = keccak256(preImage);

        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeCounterparty), amount);

        atomicBridgeCounterparty.lockBridgeTransferAssets(
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
            address initiator,
            address storedRecipient,
            uint256 storedAmount,
            bytes32 storedHashLock,
            uint256 storedTimeLock
        ) = atomicBridgeCounterparty.completedTransfers(bridgeTransferId);

        assertEq(initiator, deployer);
        assertEq(storedRecipient, recipient);
        assertEq(storedAmount, amount);
        assertEq(storedHashLock, testHashLock);
        assertGt(storedTimeLock, block.timestamp);

        vm.stopPrank();
    }

    function testAbortBridgeTransfer() public {
        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeCounterparty), amount);

        atomicBridgeCounterparty.lockBridgeTransferAssets(
            bridgeTransferId,
            hashLock,
            timeLock,
            recipient,
            amount
        );

        vm.stopPrank();

        vm.warp(block.timestamp + timeLock + 1);
        vm.startPrank(deployer);

        atomicBridgeCounterparty.abortBridgeTransfer(bridgeTransferId);

        (
            address initiator,
            address storedRecipient,
            uint256 storedAmount,
            bytes32 storedHashLock,
            uint256 storedTimeLock
        ) = atomicBridgeCounterparty.abortedTransfers(bridgeTransferId);

        assertEq(initiator, deployer);
        assertEq(storedRecipient, recipient);
        assertEq(storedAmount, amount);
        assertEq(storedHashLock, hashLock);
        assertGt(storedTimeLock, block.timestamp);

        vm.stopPrank();
    }
}

