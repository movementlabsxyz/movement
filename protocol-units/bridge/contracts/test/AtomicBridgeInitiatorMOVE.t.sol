// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiator, IAtomicBridgeInitiator, OwnableUpgradeable} from "../src/AtomicBridgeInitiator.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MOVEToken} from "../src/MOVEToken.sol";  // Assuming MOVEToken is deployed in the same directory
import {console} from "forge-std/console.sol";

contract AtomicBridgeInitiatorMOVETest is Test {
    AtomicBridgeInitiator public atomicBridgeInitiatorImplementation;
    MOVEToken public moveToken;   // Change to MOVEToken
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    AtomicBridgeInitiator public atomicBridgeInitiator;

    address public originator =  address(1);
    bytes32 public recipient = keccak256(abi.encodePacked(address(2)));
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public timeLock = 100;

    function setUp() public {
        // Deploy the MOVEToken contract
        moveToken = new MOVEToken();
        moveToken.initialize(address(this)); // Initialize MOVEToken with the deployer's address

        // Generate a random address for the originator in each test
        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.number, block.prevrandao))));

        // Deploy the AtomicBridgeInitiator contract with the MOVEToken address
        atomicBridgeInitiatorImplementation = new AtomicBridgeInitiator();
        proxyAdmin = new ProxyAdmin(msg.sender);
        proxy = new TransparentUpgradeableProxy(
            address(atomicBridgeInitiatorImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature("initialize(address,address)", address(moveToken), address(this))
        );

        atomicBridgeInitiator = AtomicBridgeInitiator(address(proxy));
    }

    function testInitiateBridgeTransferWithMove() public {
        uint256 moveAmount = 100 * 10**8; // 100 MOVEToken (assuming 8 decimals)
        moveToken.mint(originator, moveAmount); // Mint tokens to originator
        vm.startPrank(originator);

        // Approve the bridge contract to spend MOVEToken
        moveToken.approve(address(atomicBridgeInitiator), moveAmount);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            hashLock, 
            timeLock
        );

        (
            uint256 transferAmount,
            address transferOriginator,
            bytes32 transferRecipient,
            bytes32 transferHashLock,
            uint256 transferTimeLock,
            AtomicBridgeInitiator.MessageState transferState
        ) = atomicBridgeInitiator.bridgeTransfers(bridgeTransferId);

        assertEq(transferAmount, moveAmount);
        assertEq(transferOriginator, originator);
        assertEq(transferRecipient, recipient);
        assertEq(transferHashLock, hashLock);
        assertGt(transferTimeLock, block.number);
        assertEq(uint8(transferState), uint8(AtomicBridgeInitiator.MessageState.INITIALIZED));

        vm.stopPrank();
    }

    function testCompleteBridgeTransfer() public {
        bytes32 secret = "secret";
        bytes32 testHashLock = keccak256(abi.encodePacked(secret));
        uint256 moveAmount = 100 * 10**8; // 100 MOVEToken

        moveToken.mint(originator, moveAmount); // Mint tokens to originator
        vm.startPrank(originator);

        // Approve the bridge contract to spend MOVEToken
        moveToken.approve(address(atomicBridgeInitiator), moveAmount);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            testHashLock, 
            timeLock
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

        assertEq(completedAmount, moveAmount);
        assertEq(completedOriginator, originator);
        assertEq(completedRecipient, recipient);
        assertEq(completedHashLock, testHashLock);
        assertGt(completedTimeLock, block.number);
        assertEq(uint8(completedState), uint8(AtomicBridgeInitiator.MessageState.COMPLETED));
    }

    function testRefundBridgeTransfer() public {
        uint256 moveAmount = 100 * 10**8; // 100 MOVEToken
        moveToken.mint(originator, moveAmount); // Mint tokens to originator
        vm.startPrank(originator);

        // Approve the bridge contract to spend MOVEToken
        moveToken.approve(address(atomicBridgeInitiator), moveAmount);

        bytes32 bridgeTransferId = atomicBridgeInitiator.initiateBridgeTransfer(
            moveAmount, 
            recipient, 
            hashLock, 
            timeLock
        );
        vm.stopPrank();

        // Advance time and block height to ensure the time lock has expired
        vm.warp(block.number + timeLock + 1);
        uint256 futureBlockNumber = block.number + timeLock + 4200;
        vm.roll(futureBlockNumber);

        vm.startPrank(originator);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, originator));
        atomicBridgeInitiator.refundBridgeTransfer(bridgeTransferId);
        vm.stopPrank();

        vm.expectEmit();
        emit IAtomicBridgeInitiator.BridgeTransferRefunded(bridgeTransferId);
        atomicBridgeInitiator.refundBridgeTransfer(bridgeTransferId);

        assertEq(moveToken.balanceOf(originator), moveAmount, "MOVE balance mismatch");
    }
}