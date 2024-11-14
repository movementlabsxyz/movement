// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {NativeBridgeCounterpartyMOVE, OwnableUpgradeable, INativeBridgeCounterpartyMOVE} from "../src/NativeBridgeCounterpartyMOVE.sol";
import {NativeBridgeInitiatorMOVE, INativeBridgeInitiatorMOVE} from "../src/NativeBridgeInitiatorMOVE.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";

contract NativeBridgeMOVETest is Test {
    NativeBridgeCounterpartyMOVE public nativeBridgeCounterpartyMOVEImplementation;
    NativeBridgeCounterpartyMOVE public nativeBridgeCounterpartyMOVE;
    NativeBridgeInitiatorMOVE public nativeBridgeInitiatorMOVEImplementation;
    NativeBridgeInitiatorMOVE public nativeBridgeInitiatorMOVE;
    MockMOVEToken public moveToken;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxyInitiator;
    TransparentUpgradeableProxy public proxyCounterparty;
    
    address public deployer = address(0x1337);
    address public ethAddress = address(0x1);
    address public recipient = address(0x2);
    address public otherUser = address(0x3);
    uint256 public _amount = 100 * 10 ** 8; // 100 MOVEToken (assuming 8 decimals)
    uint256 public timeLock = 1 days;

    bytes32 public moveAddress = keccak256(abi.encodePacked(ethAddress));
    uint256 public constant COUNTERPARTY_TIME_LOCK_DURATION = 24 * 60 * 60; // 24 hours

    function setUp() public {
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this));

        uint256 initiatorTimeLockDuration = 48 * 60 * 60; // 48 hours for the initiator
        uint256 counterpartyTimeLockDuration = 24 * 60 * 60; // 24 hours for the counterparty

        nativeBridgeInitiatorMOVEImplementation = new NativeBridgeInitiatorMOVE();
        proxyAdmin = new ProxyAdmin(deployer);
        proxyInitiator = new TransparentUpgradeableProxy(
            address(nativeBridgeInitiatorMOVEImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256,uint256)",
                address(moveToken),
                deployer,
                initiatorTimeLockDuration,
                0 ether
            )
        );
        nativeBridgeInitiatorMOVE = NativeBridgeInitiatorMOVE(address(proxyInitiator));

        nativeBridgeCounterpartyMOVEImplementation = new NativeBridgeCounterpartyMOVE();
        proxyCounterparty = new TransparentUpgradeableProxy(
            address(nativeBridgeCounterpartyMOVEImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,uint256)",
                address(nativeBridgeInitiatorMOVE),
                deployer,
                counterpartyTimeLockDuration
            )
        );
        nativeBridgeCounterpartyMOVE = NativeBridgeCounterpartyMOVE(address(proxyCounterparty));

        vm.startPrank(deployer);
        nativeBridgeInitiatorMOVE.setCounterpartyAddress(address(nativeBridgeCounterpartyMOVE));
        vm.stopPrank();
    }

    function testInitiateBridgeTransfer()
        public
        returns (bytes32 bridgeTransferId, address originator, bytes32 recipient, uint256 amount, bytes32 hashLock, uint256 initialTimestamp, uint256 nonce, bytes32 preImage)
    {

        preImage = "secret";
        hashLock = keccak256(abi.encodePacked(preImage));
        nonce;
        originator = ethAddress;
        recipient = moveAddress;
        amount = _amount;
        moveToken.transfer(ethAddress, amount);
        vm.startPrank(ethAddress);

        moveToken.approve(address(nativeBridgeInitiatorMOVE), amount);
        bridgeTransferId = nativeBridgeInitiatorMOVE.initiateBridgeTransfer(recipient, amount, hashLock);
        nonce++;
        initialTimestamp = block.timestamp;
        vm.stopPrank();
    }

    function testCompleteInitiatorBridgeTransfer() public {
        (
            bytes32 bridgeTransferId,
            address originator,
            bytes32 recipient,
            uint256 amount,
            bytes32 hashLock,
            uint256 initialTimestamp,
            uint256 parallelNonce,
            bytes32 preImage
        ) = testInitiateBridgeTransfer();

        vm.startPrank(otherUser);

        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, otherUser, recipient, amount, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, keccak256(abi.encodePacked(otherUser)), amount, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount + 1, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong hashLock");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, keccak256(abi.encodePacked(hashLock)), initialTimestamp, parallelNonce, preImage);
        
        console.log("Testing with wrong initialTimestamp");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp + 1, parallelNonce, preImage);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce + 1, preImage);

        console.log("Testing with wrong preImage");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidSecret.selector);
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce, keccak256(abi.encodePacked(preImage)));
         
        nativeBridgeInitiatorMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce, preImage);

        NativeBridgeInitiatorMOVE.MessageState state = nativeBridgeInitiatorMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(uint8(state), uint8(NativeBridgeInitiatorMOVE.MessageState.COMPLETED));

        vm.stopPrank();
    }

    function testRefundInitiatorBridgeTransfer() public {
        (
            bytes32 bridgeTransferId,
            address originator,
            bytes32 recipient,
            uint256 amount,
            bytes32 hashLock,
            uint256 initialTimestamp,
            uint256 parallelNonce,
            bytes32 preImage
        ) = testInitiateBridgeTransfer();

        vm.startPrank(otherUser);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, otherUser));
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce);
        vm.stopPrank();

        vm.startPrank(deployer);
        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, otherUser, recipient, amount, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, keccak256(abi.encodePacked(recipient)), amount, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount + 1, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong hashLock");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, keccak256(abi.encodePacked(hashLock)), initialTimestamp, parallelNonce);
        
        console.log("Testing with wrong initialTimestamp");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp + 1, parallelNonce);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridgeInitiatorMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce + 1);

        console.log("Not enough time has passed");
        vm.expectRevert(INativeBridgeInitiatorMOVE.TimeLockNotExpired.selector);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce); 

        vm.warp(2 days + 1);
        nativeBridgeInitiatorMOVE.refundBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce);

        NativeBridgeInitiatorMOVE.MessageState state = nativeBridgeInitiatorMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(uint8(state), uint8(NativeBridgeInitiatorMOVE.MessageState.REFUNDED));

        vm.stopPrank();
    }

    function testLockBridgeTransfer()
        public
        returns (
            bytes32 bridgeTransferId,
            bytes32 originator,
            address recipient,
            uint256 amount,
            bytes32 hashLock,
            uint256 initialTimestamp,
            uint256 parallelNonce,
            bytes32 preImage
        )
    {
        moveToken.transfer(address(nativeBridgeInitiatorMOVE), _amount);

        parallelNonce = 1;
        originator = moveAddress;
        recipient = ethAddress;
        amount = _amount;
        preImage = keccak256(abi.encodePacked("secret"));
        hashLock = keccak256(abi.encodePacked(preImage));
        initialTimestamp = block.timestamp;
        bridgeTransferId =
            keccak256(abi.encodePacked(originator, recipient, amount, hashLock, initialTimestamp, parallelNonce));

        vm.startPrank(deployer);

        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId, keccak256(abi.encodePacked(otherUser)), recipient, amount, hashLock, block.timestamp, parallelNonce - 1
        );

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId,
            originator,
            otherUser,
            amount,
            hashLock,
            block.timestamp,
            parallelNonce - 1
        );

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId, originator, recipient, amount - 1, hashLock, block.timestamp, parallelNonce
        );

        console.log("Testing with wrong timestamp");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId, originator, recipient, amount - 1, hashLock, block.timestamp + 1, parallelNonce
        );

        console.log("Testing with wrong hashLock");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId,
            originator,
            recipient,
            amount - 1,
            keccak256(abi.encodePacked(hashLock)),
            block.timestamp,
            parallelNonce
        );

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId, originator, recipient, amount, hashLock, block.timestamp, parallelNonce - 1
        );

        nativeBridgeCounterpartyMOVE.lockBridgeTransfer(
            bridgeTransferId, originator, recipient, amount, hashLock, block.timestamp, parallelNonce
        );
        vm.stopPrank();

        NativeBridgeCounterpartyMOVE.MessageState state = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(uint8(state), uint8(NativeBridgeCounterpartyMOVE.MessageState.PENDING));
    }

    function testCompleteCounterpartyBridgeTransfer() public {
        (
            bytes32 bridgeTransferId,
            bytes32 originator,
            address recipient,
            uint256 amount,
            bytes32 hashLock,
            uint256 initialTimestamp,
            uint256 parallelNonce,
            bytes32 preImage
        ) = testLockBridgeTransfer();

        vm.startPrank(otherUser);

        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, keccak256(abi.encodePacked(originator)), recipient, amount, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, otherUser, amount, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount + 1, hashLock, initialTimestamp, parallelNonce, preImage);

        console.log("Testing with wrong hashLock");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, keccak256(abi.encodePacked(hashLock)), initialTimestamp, parallelNonce, preImage);
        
        console.log("Testing with wrong initialTimestamp");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp + 1, parallelNonce, preImage);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce + 1, preImage);

        console.log("Testing with wrong preImage");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidSecret.selector);
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce, keccak256(abi.encodePacked(preImage)));
         
        nativeBridgeCounterpartyMOVE.completeBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce, preImage);

        NativeBridgeCounterpartyMOVE.MessageState state = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(uint8(state), uint8(NativeBridgeCounterpartyMOVE.MessageState.COMPLETED));

        vm.stopPrank();
    }

    function testAbortCounterpartyBridgeTransfer() public {
        (
            bytes32 bridgeTransferId,
            bytes32 originator,
            address recipient,
            uint256 amount,
            bytes32 hashLock,
            uint256 initialTimestamp,
            uint256 parallelNonce,
            bytes32 preImage
        ) = testLockBridgeTransfer();

        vm.startPrank(otherUser);
        vm.expectRevert(abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, otherUser));
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce);
        vm.stopPrank();

        vm.startPrank(deployer);
        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, keccak256(abi.encodePacked(originator)), recipient, amount, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, otherUser, amount, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount + 1, hashLock, initialTimestamp, parallelNonce);

        console.log("Testing with wrong hashLock");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, keccak256(abi.encodePacked(hashLock)), initialTimestamp, parallelNonce);
        
        console.log("Testing with wrong initialTimestamp");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp + 1, parallelNonce);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.InvalidBridgeTransferId.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce + 1);

        console.log("Not enough time has passed");
        vm.expectRevert(INativeBridgeCounterpartyMOVE.TimeLockNotExpired.selector);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce); 

        vm.warp(2 days + 1);
        nativeBridgeCounterpartyMOVE.abortBridgeTransfer(bridgeTransferId, originator, recipient, amount, hashLock, initialTimestamp, parallelNonce);

        NativeBridgeCounterpartyMOVE.MessageState state = nativeBridgeCounterpartyMOVE.bridgeTransfers(bridgeTransferId);

        assertEq(uint8(state), uint8(NativeBridgeCounterpartyMOVE.MessageState.REFUNDED));

        vm.stopPrank();
    }
}
