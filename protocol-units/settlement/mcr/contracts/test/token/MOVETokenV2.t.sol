// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVETokenDev.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";

contract MOVETokenDevTest is Test {
    MOVETokenDev public token;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    address public multisig = 0x00db70A9e12537495C359581b7b3Bc3a69379A00;
    bytes32 public MINTER_ROLE;

    function setUp() public {
        MOVETokenDev moveTokenImplementation = new MOVETokenDev();

        // Deploy proxies
        TransparentUpgradeableProxy moveProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation), address(multisig), abi.encodeWithSignature(moveSignature, multisig)
        );
        token = MOVETokenDev(address(moveProxy));
        MINTER_ROLE = token.MINTER_ROLE();
    }

    function testCannotInitializeTwice() public {
        vm.startPrank(multisig);
        // Initialize the contract
        vm.expectRevert(MOVETokenDev.AlreadyInitialized.selector);
        token.initialize(multisig);
        vm.stopPrank();
    }

    function testGrants() public {
        // Check the token details
        assertEq(token.hasRole(MINTER_ROLE, multisig), true);
    }

    function testMint() public {
        vm.startPrank(multisig);
        uint256 intialBalance = token.balanceOf(address(0x1337));
        // Mint tokens
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.balanceOf(address(0x1337)), intialBalance + 100);
        vm.stopPrank();
    }

    function testRevokeMinterRole() public {
        vm.startPrank(multisig);
        assertEq(token.hasRole(MINTER_ROLE, multisig), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(multisig);

        // Check the token details
        assertEq(token.hasRole(MINTER_ROLE, multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, MINTER_ROLE)
        );
        token.mint(address(0x1337), 100);
        vm.stopPrank();
    }

    function testGrantRevokeMinterAdminRole() public {
        vm.startPrank(multisig);
        assertEq(token.hasRole(MINTER_ROLE, multisig), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(multisig);

        // Check the token details
        assertEq(token.hasRole(MINTER_ROLE, multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, MINTER_ROLE)
        );
        token.mint(address(0x1337), 100);

        assertEq(token.hasRole(MINTER_ROLE, address(0x1337)), false);
        // Grant minter role
        token.grantMinterRole(address(0x1337));
        vm.stopPrank();
        vm.prank(address(0x1337));
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.hasRole(MINTER_ROLE, address(0x1337)), true);
        vm.startPrank(multisig);
        // Revoke minter role
        token.revokeMinterRole(address(0x1337));

        assertEq(token.hasRole(MINTER_ROLE, address(0x1337)), false);
        vm.stopPrank();
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, address(0x1337), MINTER_ROLE
            )
        );
        vm.prank(address(0x1337));
        token.mint(address(0x1337), 100);
        vm.startPrank(multisig);
        assertEq(token.hasRole(token.MINTER_ADMIN_ROLE(), multisig), true);
        // Revoke minter admin role
        token.revokeMinterAdminRole(multisig);

        assertEq(token.hasRole(token.MINTER_ADMIN_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, token.MINTER_ADMIN_ROLE()
            )
        );
        token.grantMinterRole(multisig);

        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, MINTER_ROLE)
        );
        token.mint(address(0x1337), 100);
        vm.stopPrank();
    }

    function testCannotGrantRoleFuzz(address messenger, address receiver) public {
        vm.startPrank(messenger);
        if (messenger != multisig) {
            vm.expectRevert(
                abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, messenger, 0x00)
            );
            token.grantRole(MINTER_ROLE, receiver);
        }
        vm.stopPrank();
    }
}
