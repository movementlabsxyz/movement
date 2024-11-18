// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {NativeBridge, AccessControl, INativeBridge} from "../src/NativeBridge.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";

contract NativeBridgeTest is Test {
    NativeBridgeCounterpartyMOVE public nativeBridgeImplementation;
    NativeBridgeCounterpartyMOVE public nativeBridge;
    MockMOVEToken public moveToken;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;

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

        nativeBridgeImplementation = new NativeBridge();
        proxyAdmin = new ProxyAdmin(deployer);
        proxy = new TransparentUpgradeableProxy(
            address(nativeBridgeImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,address,address)", address(moveToken), deployer, deployer, address(0)
            )
        );
        nativeBridge = NativeBridge(address(proxy));
    }

    function testInitiateBridge()
        public
        returns (bytes32 bridgeTransferId, address originator, bytes32 recipient, uint256 amount, uint256 nonce)
    {
        // initialize vars
        nonce;
        originator = ethAddress;
        recipient = moveAddress;
        amount = _amount;

        moveToken.transfer(ethAddress, amount);
        vm.startPrank(ethAddress);

        // require approval
        vm.expectRevert(ERC20Upgradeable.InsufficientApproval.selector);
        nativeBridge.initiateBridge(recipient, amount);

        moveToken.approve(address(nativeBridge), amount);

        vm.expectRevert(NativeBridge.ZeroAmount.selector);
        nativeBridge.initiateBridge(recipient, 0);

        bridgeTransferId = nativeBridge.initiateBridge(recipient, amount);
        nonce++;
        vm.stopPrank();
    }

    function testCompleteBridge() public {
        uint256 _initialTimestamp = block.timestamp;
        bytes32 _bridgeTransferId = keccak256(abi.encodePacked(ethAddress, moveAddress, _amount, uint256(0)));

        console.log("attempting to complete inexistent transaction");
        vm.expectRevert(INativeBridge.BridgeTransferNotInitialized.selector);
        nativeBridge.completeBridge(_bridgeTransferId, ethAddress, moveAddress, _amount, uint256(0));
        (bytes32 bridgeTransferId, address originator, bytes32 recipient, uint256 amount, uint256 parallelNonce) =
            testInitiateBridgeTransfer();

        vm.startPrank(otherUser);

        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(bridgeTransferId, otherUser, recipient, amount, parallelNonce);

        console.log("Testing with wrong recipient");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(
            bridgeTransferId, originator, keccak256(abi.encodePacked(otherUser)), amount, parallelNonce
        );

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(bridgeTransferId, originator, recipient, amount + 1, parallelNonce);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(bridgeTransferId, originator, recipient, amount, parallelNonce + 1);

        nativeBridge.completeBridge(bridgeTransferId, originator, recipient, amount, parallelNonce);

        NativeBridge.OutgoingBridgeTransfer bridgeTransfer = nativeBridge.bridgeTransfers(bridgeTransferId);

        assertEq(bridgeTransfer.originator == originator);
        assertEq(bridgeTransfer.recipient == recipient);
        assertEq(bridgeTransfer.amount == recipient);
        assertEq(bridgeTransfer.nonce == nonce);
        vm.stopPrank();
    }
}
