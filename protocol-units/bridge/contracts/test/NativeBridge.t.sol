// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {NativeBridge, AccessControlUpgradeable, INativeBridge} from "../src/NativeBridge.sol";
import {IERC20Errors} from "openzeppelin-contracts/contracts/interfaces/draft-IERC6093.sol";
import {IAccessControl} from "openzeppelin-contracts/contracts/access/IAccessControl.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

import {MockMOVEToken} from "../src/MockMOVEToken.sol";

contract NativeBridgeTest is Test {
    NativeBridge public nativeBridgeImplementation;
    NativeBridge public nativeBridge;
    MockMOVEToken public moveToken;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;

    address public deployer = address(0x1337);
    address public relayer = address(0x8e1a7e8);
    address public recipient = address(0x2);
    address public otherUser = address(0x3);
    address public insuranceFund = address(0x4);

    function setUp() public {
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this));

        nativeBridgeImplementation = new NativeBridge();
        proxyAdmin = new ProxyAdmin(deployer);
        proxy = new TransparentUpgradeableProxy(
            address(nativeBridgeImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,address,address,address)",
                address(moveToken),
                deployer,
                relayer,
                address(0),
                insuranceFund
            )
        );
        nativeBridge = NativeBridge(address(proxy));
        moveToken.transfer(insuranceFund, 1000000 * 1e8);
    }

    function testInitiateBridgeFuzz(address _originator, bytes32 _recipient, uint256 _amount) public {
        excludeSender(deployer);
        vm.assume(_originator != address(0));
        vm.assume(_originator != deployer);

        _amount = bound(_amount, 1, 1000000 * 1e8 - 1);
        moveToken.transfer(_originator, _amount);
        vm.startPrank(_originator);
        // require approval
        vm.expectRevert(
            abi.encodeWithSelector(IERC20Errors.ERC20InsufficientAllowance.selector, address(nativeBridge), 0, _amount)
        );
        nativeBridge.initiateBridgeTransfer(_recipient, _amount);

        moveToken.approve(address(nativeBridge), _amount);

        vm.expectRevert(INativeBridge.ZeroAmount.selector);
        nativeBridge.initiateBridgeTransfer(_recipient, 0);

        bytes32 bridgeTransferId = nativeBridge.initiateBridgeTransfer(_recipient, _amount);

        (bytes32 bridgeTransferId_, address originator, bytes32 recipient_, uint256 amount) =
            nativeBridge.noncesToOutboundTransfers(1);

        assertEq(originator, _originator);
        assertEq(recipient_, _recipient);
        assertEq(amount, _amount);
        assertEq(bridgeTransferId_, bridgeTransferId);
        vm.stopPrank();
    }

    function testInboundRateLimitFuzz(address receiver, uint256 _amount, uint256 _denominator, uint256 _divisor) public {
        uint256 insuranceBalance = moveToken.balanceOf(insuranceFund);

        // amount has to be divisible by 4
         _amount = bound(_amount, 3, insuranceBalance);
         // fund bridge with sufficient funds
        moveToken.transfer(address(nativeBridge), _amount);
        // risk denominator has to be between 4 and move total supply
        _denominator = bound(_denominator, 4, moveToken.totalSupply());
        _divisor = bound(_divisor, 4, 20);

        uint256 perTransfer = _amount / _divisor;

        bytes32 tx1BridgeTransferId = keccak256(abi.encodePacked(keccak256(abi.encodePacked(receiver)), receiver, perTransfer, uint256(1)));
        bytes32 tx2BridgeTransferId = keccak256(abi.encodePacked(keccak256(abi.encodePacked(receiver)), receiver, perTransfer, uint256(2)));
        

        vm.startPrank(relayer);
        nativeBridge.completeBridgeTransfer(tx1BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 1);
        vm.warp(1 days - 1);

        uint256 snapshot = vm.snapshot();

        // if the sum of the two transfers is below the rate limit, the second transfer should go through, else it fails
        if (perTransfer * 2 >= insuranceBalance / 4) {
            vm.expectRevert(INativeBridge.InboundRateLimitExceeded.selector);
            nativeBridge.completeBridgeTransfer(tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2);
            vm.warp(1 days + 1);
            if (perTransfer >= insuranceBalance / 4) {
                // unable to bridge above rate limit
                vm.expectRevert(INativeBridge.InboundRateLimitExceeded.selector);
                nativeBridge.completeBridgeTransfer(tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2);
            } else {
                nativeBridge.completeBridgeTransfer(tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2);
            }
        } else {
            nativeBridge.completeBridgeTransfer(tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2);
        }
        vm.stopPrank();

        vm.revertTo(snapshot);
        vm.warp(1 days - 1);
        vm.prank(deployer);
        nativeBridge.setRiskDenominator(_denominator);
        vm.startPrank(relayer);

        tx2BridgeTransferId = keccak256(abi.encodePacked(keccak256(abi.encodePacked(receiver)), receiver, perTransfer, uint256(2)));
        
        // if the sum of the two transfers is below the rate limit with altered denominator, the second transfer should go through, else it fails
        if (perTransfer * 2 >= insuranceBalance / _denominator) {
            vm.expectRevert(INativeBridge.InboundRateLimitExceeded.selector);
            nativeBridge.completeBridgeTransfer(
                tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2
            );
            // Warp to the next period and retry the transfer
            vm.warp(1 days + 1);
            if (perTransfer >= insuranceBalance / _denominator) {
                // unable to bridge above rate limit
                vm.expectRevert(INativeBridge.InboundRateLimitExceeded.selector);
                nativeBridge.completeBridgeTransfer(
                    tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2
                );
            } else {
                nativeBridge.completeBridgeTransfer(
                    tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2
                );
            }
        } else {
            // Complete the second transfer if rate limit is not exceeded
            nativeBridge.completeBridgeTransfer(
                tx2BridgeTransferId, keccak256(abi.encodePacked(receiver)), receiver, perTransfer, 2
            );
        }
        vm.stopPrank();

        vm.revertTo(snapshot);
        vm.warp(1 days - 1);
        // test accumulation against the rate limiter
        vm.startPrank(relayer);
        uint256 rateLimitBudgetTracker = perTransfer;

        // for divisor amount of transfers, the rate limit should be enforced
        for (uint256 i = 1; i < _divisor; i++) {
            if (_amount / _divisor + rateLimitBudgetTracker < insuranceBalance / 4) {
                nativeBridge.completeBridgeTransfer(
                keccak256(abi.encodePacked(keccak256(abi.encodePacked(receiver)), receiver, _amount / _divisor, uint256(i + 1))),
                keccak256(abi.encodePacked(receiver)), receiver, _amount / _divisor, i + 1);
                rateLimitBudgetTracker += _amount / _divisor;
            } else {
                vm.expectRevert(INativeBridge.InboundRateLimitExceeded.selector);
                nativeBridge.completeBridgeTransfer(
                    keccak256(abi.encodePacked(keccak256(abi.encodePacked(receiver)), receiver, _amount / _divisor, uint256(i + 1))),
                    keccak256(abi.encodePacked(receiver)), receiver, _amount / _divisor, i + 1);
            }
        }
        vm.stopPrank();
    }

    function testCompleteBridgeFuzz(bytes32 _originator, address _recipient, uint256 _amount, uint256 _nonce) public {
        excludeSender(deployer);
        vm.assume(_recipient != address(0));
        vm.assume(relayer != address(0));
        _amount = bound(_amount, 1, moveToken.balanceOf(nativeBridge.insuranceFund()) / 4 - 2);
        // nonce cannot be uint256 max because we are testing a +1 addition case to the nonce
        _nonce = bound(_nonce, 1, type(uint256).max - 1);

        bytes32 bridgeTransferId = keccak256(abi.encodePacked(_originator, _recipient, _amount, _nonce));

        moveToken.transfer(address(nativeBridge), _amount);

        console.log("Testing unathourized relayer");
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), keccak256("RELAYER_ROLE")
            )
        );
        nativeBridge.completeBridgeTransfer(bridgeTransferId, _originator, _recipient, _amount, _nonce);

        vm.startPrank(relayer);
        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridgeTransfer(
            bridgeTransferId, keccak256(abi.encodePacked(otherUser)), _recipient, _amount, _nonce
        );

        if (_recipient != otherUser) {
            console.log("Testing with wrong recipient");
            vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
            nativeBridge.completeBridgeTransfer(bridgeTransferId, _originator, otherUser, _amount, _nonce);
        }

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridgeTransfer(bridgeTransferId, _originator, _recipient, _amount + 1, _nonce);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridgeTransfer(bridgeTransferId, _originator, _recipient, _amount, _nonce + 1);

        console.log("Testing correct values");
        nativeBridge.completeBridgeTransfer(bridgeTransferId, _originator, _recipient, _amount, _nonce);

        uint256 nonce = nativeBridge.idsToInboundNonces(bridgeTransferId);

        assertEq(nonce, _nonce);
        vm.stopPrank();
    }

    function testBatchCompleteFuzz(uint256 length) external {
        length = bound(length, 1, 100);
        bytes32[] memory bridgeTransferIds = new bytes32[](length);
        bytes32[] memory originators = new bytes32[](length);
        address[] memory recipients = new address[](length);
        uint256[] memory amounts = new uint256[](length);
        uint256[] memory nonces = new uint256[](length);

        uint256 fundContract;
        for (uint256 i; i < length; i++) {
            originators[i] = keccak256(abi.encodePacked(i));
            recipients[i] = address(uint160(i + 1));
            amounts[i] = i + 1;
            nonces[i] = i + 1;
            bridgeTransferIds[i] = keccak256(abi.encodePacked(originators[i], recipients[i], amounts[i], nonces[i]));
            fundContract += amounts[i];
        }

        console.log("native bridge address:", address(nativeBridge));
        moveToken.transfer(address(nativeBridge), fundContract);
        vm.startPrank(relayer);

        nativeBridge.batchCompleteBridgeTransfer(bridgeTransferIds, originators, recipients, amounts, nonces);

        for (uint256 i; i < length; i++) {
            uint256 nonce = nativeBridge.idsToInboundNonces(bridgeTransferIds[i]);

            assertEq(nonce, nonces[i]);
        }

        vm.expectRevert(INativeBridge.CompletedBridgeTransferId.selector);
        nativeBridge.batchCompleteBridgeTransfer(bridgeTransferIds, originators, recipients, amounts, nonces);
        vm.stopPrank();
    }
}
