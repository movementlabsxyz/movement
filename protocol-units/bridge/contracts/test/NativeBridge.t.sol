// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {NativeBridge, AccessControlUpgradeable, INativeBridge} from "../src/NativeBridge.sol";
import {IERC20Errors} from "openzeppelin-contracts/contracts/interfaces/draft-IERC6093.sol";
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

    function setUp() public {
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this));

        nativeBridgeImplementation = new NativeBridge();
        proxyAdmin = new ProxyAdmin(deployer);
        proxy = new TransparentUpgradeableProxy(
            address(nativeBridgeImplementation),
            address(proxyAdmin),
            abi.encodeWithSignature(
                "initialize(address,address,address,address)", address(moveToken), deployer, relayer, address(0)
            )
        );
        nativeBridge = NativeBridge(address(proxy));
    }

    function testInitiateBridgeFuzz(address _originator, bytes32 _recipient, uint256 _amount) public {
        excludeSender(deployer);
        _amount = bound(_amount, 1, 10000000000 * 10 ** 8);
        moveToken.transfer(_originator, _amount);
        vm.startPrank(_originator);

        // require approval
        vm.expectRevert(
            abi.encodeWithSelector(IERC20Errors.ERC20InsufficientAllowance.selector, address(nativeBridge), 0, _amount)
        );
        nativeBridge.initiateBridge(_recipient, _amount);

        moveToken.approve(address(nativeBridge), _amount);

        vm.expectRevert(INativeBridge.ZeroAmount.selector);
        nativeBridge.initiateBridge(_recipient, 0);

        bytes32 bridgeTransferId = nativeBridge.initiateBridge(_recipient, _amount);

        (address originator, bytes32 recipient_, uint256 amount, uint256 nonce) =
            nativeBridge.outgoingBridgeTransfers(bridgeTransferId);

        assertEq(originator, _originator);
        assertEq(recipient_, _recipient);
        assertEq(amount, _amount);
        assertEq(nonce, 1);
        vm.stopPrank();
    }

    function testCompleteBridgeFuzz(bytes32 _originator, address _recipient, uint256 _amount, uint256 _nonce) public {
        excludeSender(deployer);
        _amount = bound(_amount, 1, 10000000000 * 10 ** 8);
        // nonce cannot be uint256 max because we are testing a +1 addition case to the nonce
        _nonce = bound(_nonce, 1, type(uint256).max - 1);

        bytes32 bridgeTransferId = keccak256(abi.encodePacked(_originator, _recipient, _amount, _nonce));

        moveToken.transfer(address(nativeBridge), _amount);

        vm.startPrank(relayer);

        console.log("Testing with wrong originator");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(
            bridgeTransferId, keccak256(abi.encodePacked(otherUser)), _recipient, _amount, _nonce
        );

        if (_recipient != otherUser) {
            console.log("Testing with wrong recipient");
            vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
            nativeBridge.completeBridge(bridgeTransferId, _originator, otherUser, _amount, _nonce);
        }

        console.log("Testing with wrong amount");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(bridgeTransferId, _originator, _recipient, _amount + 1, _nonce);

        console.log("Testing with wrong nonce");
        vm.expectRevert(INativeBridge.InvalidBridgeTransferId.selector);
        nativeBridge.completeBridge(bridgeTransferId, _originator, _recipient, _amount, _nonce + 1);

        console.log("Testing correct values");
        nativeBridge.completeBridge(bridgeTransferId, _originator, _recipient, _amount, _nonce);

        (bytes32 originator_, address recipient_, uint256 amount_, uint256 nonce_) =
            nativeBridge.incomingBridgeTransfers(bridgeTransferId);

        assertEq(originator_, _originator);
        assertEq(recipient_, _recipient);
        assertEq(amount_, _amount);
        assertEq(nonce_, _nonce);
        vm.stopPrank();
    }
}
