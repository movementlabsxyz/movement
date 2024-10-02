// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiator, IAtomicBridgeInitiator, OwnableUpgradeable} from "../src/AtomicBridgeInitiator.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IWETH9} from "../src/IWETH9.sol";
import {console} from "forge-std/console.sol";

contract AtomicBridgeInitiatorWethTest is Test {
    AtomicBridgeInitiator public atomicBridgeInitiatorImplementation;
    IWETH9 public weth;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    AtomicBridgeInitiator public atomicBridgeInitiator;

    address public originator = address(1);
    bytes32 public recipient = keccak256(abi.encodePacked(address(2)));
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public constant timeLockDuration = 48 * 60 * 60; // 48 hours in seconds
    uint256 public initialPoolBalance = 0 ether;

    function setUp() public {
        // Sepolia WETH9 address
        address wethAddress = 0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14;
        weth = IWETH9(wethAddress);

        // Generate random address for each test
        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.number, block.prevrandao))));

        // Deploy the AtomicBridgeInitiator contract with the WETH address, a 48-hour time lock, and initial pool balance
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiator();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256)", 
                wethAddress, 
                address(this), 
                timeLockDuration, 
                initialPoolBalance
            )
        );

        atomicBridgeInitiator = AtomicBridgeInitiator(address(proxy));
    }

    function testInitiateBridgeTransferWithEth() public {
        vm.deal(originator, 1 ether);
        vm.startPrank(originator);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount
            recipient,
            hashLock
        );

        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiator.MessageState transferState
        ) = atomicBridgeInitiator.bridgeTransfers(bridgeTransferId);

        assertEq(transferAmount, amount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiator.MessageState.INITIALIZED));

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 secret = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(secret));

        vm.deal(originator, 1 ether);
        vm.startPrank(originator);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount is 0
            recipient,
            testHashLock
        );

        vm.stopPrank();

        atomicBridgeInitiator.completeBridgeTransfer(bridgeTransferId, secret);
        (
            uint256 completedAmount,
            address completedOriginator,
            bytes32 completedRecipient,
            bytes32 completedHashLock,
            uint256 completedTimeLock,
            AtomicBridgeInitiator.MessageState completedState
        ) = atomicBridgeInitiator.bridgeTransfers(bridgeTransferId);

        assertEq(completedAmount, amount);
        assertEq(completedOriginator, originator);
        assertEq(completedRecipient, recipient);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.timestamp);
        assertEq(uint8(completedState), uint8(AtomicBridgeInitiator.MessageState.COMPLETED));
    }

    function testInitiateBridgeTransferWithWeth() public {
        uint256 wethAmount = 1 ether;
        weth.totalSupply();
        vm.deal(originator, 1 ether);
        vm.startPrank(originator);
        weth.deposit{value: wethAmount}();
        assertEq(weth.balanceOf(originator), wethAmount);
        weth.approve(address(atomicBridgeInitiator), wethAmount);
        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer(wethAmount, recipient, hashLock);

        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiator.MessageState transferState
        ) = atomicBridgeInitiator.bridgeTransfers(bridgeTransferId);

        assertEq(transferAmount, wethAmount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiator.MessageState.INITIALIZED));

        vm.stopPrank();
    }

    function testInitiateBridgeTransferWithEthAndWeth() public {
        uint256 wethAmount = 1 ether;
        uint256 ethAmount = 2 ether;
        uint256 totalAmount = wethAmount + ethAmount;

        // Ensure the originator has sufficient ETH
        vm.deal(originator, 100 ether);

        vm.startPrank(originator);
        // Ensure WETH contract is correctly funded and transfer WETH to originator
        weth.deposit{value: wethAmount}();

        assertEq(weth.balanceOf(originator), wethAmount, "WETH balance mismatch");

        // Approve the transfer
        weth.approve(address(atomicBridgeInitiator), wethAmount);

        // Initiate bridge transfer with both ETH and WETH
        bytes32 bridgeTransferId =
            atomicBridgeInitiator.initiateBridgeTransfer{value: ethAmount}(wethAmount, recipient, hashLock);

        // Fetch the details of the initiated bridge transfer
        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiator.MessageState transferState
        ) = atomicBridgeInitiator.bridgeTransfers(bridgeTransferId);

        // Assertions
        assertEq(transferAmount, totalAmount, "Transfer amount mismatch");
        assertEq(transferOriginator, originator, "Originator address mismatch");
        assertEq(transferRecipient, recipient, "Recipient address mismatch");
        assertEq(transferHashLock, hashLock, "HashLock mismatch");
        assertGt(transferTimeLock, block.timestamp, "TimeLock is not greater than current block number");
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiator.MessageState.INITIALIZED));

        vm.stopPrank();
    }

    function testRefundBridgeTransfer() public {
        vm.deal(originator, 1 ether);

        // Originator initiates a bridge transfer
        vm.startPrank(originator);
        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount is 0
            recipient,
            hashLock
        );
        vm.stopPrank();

        // Advance time to ensure the time lock has expired (48 hours + 1 second)
        vm.warp(block.timestamp + timeLockDuration + 1);

        // Test that a non-owner cannot call refund
        vm.startPrank(originator);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, originator));
        atomicBridgeInitiator.refundBridgeTransfer(bridgeTransferId);
        vm.stopPrank();

        // Refund should be allowed only by the contract owner
        vm.expectEmit();
        emit IAtomicBridgeInitiator.BridgeTransferRefunded(bridgeTransferId);
        atomicBridgeInitiator.refundBridgeTransfer(bridgeTransferId);

        // Verify the WETH balance, originator should receive WETH back
        assertEq(weth.balanceOf(originator), 1 ether, "WETH balance mismatch");
    }
}

