// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVEToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract MOVETokenTest is Test {
    MOVEToken public token;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(string,string)";

    function setUp() public {
        MOVEToken moveTokenImplementation = new MOVEToken();

        // Contract MCRTest is the admin
        admin = new ProxyAdmin(address(this));

        // Deploy proxies
        TransparentUpgradeableProxy moveProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation), address(admin), abi.encodeWithSignature(moveSignature, "Move Token", "MOVE")
        );
        token = MOVEToken(address(moveProxy));
    }

    function testCannotInitializeTwice() public {
        // Initialize the contract
        vm.expectRevert(0xf92ee8a9);
        token.initialize();
    }

    function testGrants() public {

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), true);
    }

    function testMint() public {
        uint256 intialBalance = token.balanceOf(address(0x1337));
        // Mint tokens
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.balanceOf(address(0x1337)), intialBalance + 100);
    }

    function testRevokeMinterRole() public {
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(address(this));

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), false);

        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), token.MINTER_ROLE()));
        token.mint(address(0x1337), 100);
    }

    function testGrantRevokeMinterAdminRole() public {
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(address(this));

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), false);

        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), token.MINTER_ROLE()));
        token.mint(address(0x1337), 100);

        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), false);
        // Grant minter role
        token.grantMinterRole(address(0x1337));

        vm.prank(address(0x1337));
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), true);

        // Revoke minter role
        token.revokeMinterRole(address(0x1337));

        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), false);

        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(0x1337), token.MINTER_ROLE()));
        vm.prank(address(0x1337));
        token.mint(address(0x1337), 100);

        assertEq(token.hasRole(token.MINTER_ADMIN_ROLE(), address(this)), true);
        // Revoke minter admin role
        token.revokeMinterAdminRole(address(this));

        assertEq(token.hasRole(token.MINTER_ADMIN_ROLE(), address(this)), false);

        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), token.MINTER_ADMIN_ROLE()));
        token.grantMinterRole(address(this));

        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), token.MINTER_ROLE()));
        token.mint(address(0x1337), 100);

    }
}
