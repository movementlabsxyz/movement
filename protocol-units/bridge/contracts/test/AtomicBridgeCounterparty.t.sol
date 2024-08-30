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
    uint256 public timeLock = 100;
    bytes32 public initiator = keccak256(abi.encodePacked(deployer));
    bytes32 public bridgeTransferId =
        keccak256(abi.encodePacked(block.number, initiator, recipient, amount, hashLock, timeLock));

    function setUp() public {
        // Sepolia WETH9 address
        address wethAddress = 0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14;
        weth = IWETH9(wethAddress);

        // Deploy the AtomicBridgeInitiator contract
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiator();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature("initialize(address,address)", wethAddress, deployer)
        );

        atomicBridgeInitiator = AtomicBridgeInitiator(address(proxy));

        // Deploy the AtomicBridgeCounterparty contract
        atomicBridgeCounterpartyImplementation = new AtomicBridgeCounterparty();
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeCounterpartyImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature("initialize(address,address)", address(atomicBridgeInitiator), deployer)
        );

        atomicBridgeCounterparty = AtomicBridgeCounterparty(address(proxy));

        // Set the counterparty contract in the AtomicBridgeInitiator contract
        vm.startPrank(deployer);
        atomicBridgeInitiator.setCounterpartyAddress(address(atomicBridgeCounterparty));
        vm.stopPrank();
    }

    function testLockBridgeTransferAssets() public {
        vm.startPrank(deployer);
        vm.deal(deployer, 1 ether);

        // Deposit WETH into AtomicBridgeInitiator to increase poolBalance
        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeInitiator), amount);
        atomicBridgeInitiator.initiateBridgeTransfer(amount, initiator, hashLock, timeLock);

        bool result = atomicBridgeCounterparty.lockBridgeTransferAssets(
            initiator, bridgeTransferId, hashLock, timeLock, recipient, amount
        );

        (
            bytes32 pendingInitiator,
            address pendingRecipient,
            uint256 pendingAmount,
            bytes32 pendingHashLock,
            uint256 pendingTimelock,
            AtomicBridgeCounterparty.MessageState pendingState
        ) = atomicBridgeCounterparty.bridgeTransfers(bridgeTransferId);

        assert(result);
        assertEq(pendingInitiator, initiator);
        assertEq(pendingRecipient, recipient);
        assertEq(pendingAmount, amount);
        assertEq(pendingHashLock, hashLock);
        assertGt(pendingTimelock, block.number);
        assertEq(uint8(pendingState), uint8(AtomicBridgeCounterparty.MessageState.PENDING));

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 preImage = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(preImage));

        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        // Deposit WETH into AtomicBridgeInitiator to increase poolBalance
        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeInitiator), amount);
        atomicBridgeInitiator.initiateBridgeTransfer(amount, initiator, testHashLock, timeLock);

        atomicBridgeCounterparty.lockBridgeTransferAssets(
            initiator, bridgeTransferId, testHashLock, timeLock, recipient, amount
        );

        vm.stopPrank();
        vm.startPrank(otherUser);

        atomicBridgeCounterparty.completeBridgeTransfer(bridgeTransferId, preImage);

        (
            bytes32 completedInitiator,
            address completedRecipient,
            uint256 completedAmount,
            bytes32 completedHashLock,
            uint256 completedTimeLock,
            AtomicBridgeCounterparty.MessageState completedState
        ) = atomicBridgeCounterparty.bridgeTransfers(bridgeTransferId);

        assertEq(completedInitiator, initiator);
        assertEq(completedRecipient, recipient);
        assertEq(completedAmount, amount);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.number);
        assertEq(uint8(completedState), uint8(AtomicBridgeCounterparty.MessageState.COMPLETED));

        vm.stopPrank();
    }

    function testAbortBridgeTransfer() public {
        vm.deal(deployer, 1 ether);
        vm.startPrank(deployer);

        // Deposit WETH into AtomicBridgeInitiator to increase poolBalance
        weth.deposit{value: amount}();
        weth.approve(address(atomicBridgeInitiator), amount);
        atomicBridgeInitiator.initiateBridgeTransfer(amount, initiator, hashLock, timeLock);

        atomicBridgeCounterparty.lockBridgeTransferAssets(
            initiator, bridgeTransferId, hashLock, timeLock, recipient, amount
        );

        vm.stopPrank();

        // Advance the block number to beyond the timelock period
        vm.roll(block.number + timeLock + 1);

        // Malicious attempt to abort the bridge transfer
        vm.prank(address(0x1337));
        vm.expectRevert();
        atomicBridgeCounterparty.abortBridgeTransfer(bridgeTransferId);

        vm.startPrank(deployer);

        atomicBridgeCounterparty.abortBridgeTransfer(bridgeTransferId);

        (
            bytes32 abortedInitiator,
            address abortedRecipient,
            uint256 abortedAmount,
            bytes32 abortedHashLock,
            uint256 abortedTimeLock,
            AtomicBridgeCounterparty.MessageState abortedState
        ) = atomicBridgeCounterparty.bridgeTransfers(bridgeTransferId);

        assertEq(abortedInitiator, initiator);
        assertEq(abortedRecipient, recipient);
        assertEq(abortedAmount, amount);
        assertEq(abortedHashLock, hashLock);
        assertLe(abortedTimeLock, block.number, "Timelock is not less than or equal to current block number");
        assertEq(uint8(abortedState), uint8(AtomicBridgeCounterparty.MessageState.REFUNDED));

        vm.stopPrank();
    }
}
