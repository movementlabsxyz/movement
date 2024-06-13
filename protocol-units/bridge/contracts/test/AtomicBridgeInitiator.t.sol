// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.7.6;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiator} from "../src/AtomicBridgeInitator.sol";
import {WETH10} from "../src/WETH/WETH10.sol"; 

contract AtomicBridgeInitiatorTest is Test {
    AtomicBridgeInitiator public atomicBridgeInitiator;
    WETH10 public weth;

    address public originator = address(1);
    address public recipient = address(2);
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint public amount = 1 ether;
    uint public timeLock = 100;

    function setUp() public {
        // Deploy the WETH contract
        weth = new WETH10();
        // Deploy the AtomicBridgeInitiator contract with the WETH address
        atomicBridgeInitiator = new AtomicBridgeInitiator(address(weth));
    }

    function testInitiateBridgeTransferWithEth() public {
        vm.deal(originator, 1 ether);
        vm.startPrank(originator);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount
            originator,
            recipient,
            hashLock,
            timeLock
        );

        (
            bool exists, 
            uint transferAmount,  
            address transferOriginator, 
            address transferRecipient,
            bytes32 transferHashLock,
            uint transferTimeLock 
        ) = atomicBridgeInitiator.getBridgeTransferDetail(bridgeTransferId);

        assertTrue(exists);
        assertEq(transferAmount, amount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 secret = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(secret));

        vm.deal(originator, 1 ether);
        vm.startPrank(originator);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount is 0 
            originator, 
            recipient, 
            testHashLock, 
            timeLock
        );

        vm.stopPrank();

        vm.startPrank(recipient);
        atomicBridgeInitiator.completeBridgeTransfer(bridgeTransferId, secret);

        (bool exists,,,,,) = atomicBridgeInitiator.getBridgeTransferDetail(bridgeTransferId);
        assertFalse(exists);

        (
            bool completedExists, 
            uint completedAmount, 
            address completedOriginator, 
            address completedRecipient, 
            bytes32 completedHashLock, 
            uint completedTimeLock 
        ) = atomicBridgeInitiator.getCompletedBridgeTransferDetail(bridgeTransferId);
        assertTrue(completedExists);
        assertEq(completedAmount, amount);
        assertEq(completedOriginator, originator);
        assertEq(completedRecipient, recipient);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.timestamp);

        vm.stopPrank();
    }

    function testInitiateBridgeTransferWithWeth() public {
        uint256 wethAmount = 1 ether; // use ethers unit

        vm.deal(originator, 1 ether);
        vm.startPrank(originator);
        weth.deposit{value: wethAmount}();
        weth.transfer(originator, wethAmount);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer(
            wethAmount,
            originator,
            recipient,
            hashLock,
            timeLock
        );

        (
            bool exists, 
            uint transferAmount,  
            address transferOriginator, 
            address transferRecipient,
            bytes32 transferHashLock,
            uint transferTimeLock 
        ) = atomicBridgeInitiator.getBridgeTransferDetail(bridgeTransferId);

        assertTrue(exists);
        assertEq(transferAmount, wethAmount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.timestamp);

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
        weth.transfer(originator, wethAmount);

        assertEq(weth.balanceOf(originator), wethAmount, "WETH balance mismatch");

        vm.startPrank(originator);

        // Try to initiate bridge transfer
        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: ethAmount}(
            wethAmount,
            originator,
            recipient,
            hashLock,
            timeLock
        );

        // Fetch the details of the initiated bridge transfer
        (
            bool exists, 
            uint transferAmount,  
            address transferOriginator, 
            address transferRecipient,
            bytes32 transferHashLock,
            uint transferTimeLock 
        ) = atomicBridgeInitiator.getBridgeTransferDetail(bridgeTransferId);

        // Assertions
        assertTrue(exists, "Bridge transfer does not exist");
        assertEq(transferAmount, totalAmount, "Transfer amount mismatch");
        assertEq(transferOriginator, originator, "Originator address mismatch");
        assertEq(transferRecipient, recipient, "Recipient address mismatch");
        assertEq(transferHashLock, hashLock, "HashLock mismatch");
        assertGt(transferTimeLock, block.timestamp, "TimeLock is not greater than current block timestamp");

        vm.stopPrank();
    }


    function testRefundBridgeTransfer() public {
        vm.deal(originator, 1 ether);
        vm.startPrank(originator);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer{value: amount}(
            0, // _wethAmount is 0
            originator, 
            recipient, 
            hashLock, 
            timeLock
        );

        vm.stopPrank();

        vm.warp(block.timestamp + timeLock + 1);
        vm.startPrank(originator);
        atomicBridgeInitiator.refundBridgeTransfer(bridgeTransferId);

        (bool exists,,,,,) = atomicBridgeInitiator.getBridgeTransferDetail(bridgeTransferId);
        assertFalse(exists);

        vm.stopPrank();
    }
}

