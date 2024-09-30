// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVETokenV2.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";

contract MOVETokenV2Test is Test {
    MOVETokenV2 public token;
    ProxyAdmin public admin;
    string public moveSignature = "initialize()";
    address public multisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);

    function setUp() public {
        MOVETokenV2 moveTokenImplementation = new MOVETokenV2();

        // Contract MCRTest is the admin
        admin = new ProxyAdmin(multisig);

        // Deploy proxies
        TransparentUpgradeableProxy moveProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation),
            address(admin),
            abi.encodeWithSignature(moveSignature)
        );
        token = MOVETokenV2(address(moveProxy));
    }

    function testCannotInitializeTwice() public {
        vm.startPrank(multisig);
        // Initialize the contract
        vm.expectRevert(MOVETokenV2.AlreadyInitialized.selector);
        token.initialize();
        vm.stopPrank();
    }

    function testGrants() public {
        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), multisig), true);
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
        assertEq(token.hasRole(token.MINTER_ROLE(), multisig), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(multisig);

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, token.MINTER_ROLE()
            )
        );
        token.mint(address(0x1337), 100);
        vm.stopPrank();
    }

    function testGrantRevokeMinterAdminRole() public {
        vm.startPrank(multisig);
        assertEq(token.hasRole(token.MINTER_ROLE(), multisig), true);

        token.mint(address(0x1337), 100);
        // Revoke minter role
        token.revokeMinterRole(multisig);

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, token.MINTER_ROLE()
            )
        );
        token.mint(address(0x1337), 100);

        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), false);
        // Grant minter role
        token.grantMinterRole(address(0x1337));
        vm.stopPrank();
        vm.prank(address(0x1337));
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), true);
        vm.startPrank(multisig);
        // Revoke minter role
        token.revokeMinterRole(address(0x1337));

        assertEq(token.hasRole(token.MINTER_ROLE(), address(0x1337)), false);
        vm.stopPrank();
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, address(0x1337), token.MINTER_ROLE()
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
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, multisig, token.MINTER_ROLE()
            )
        );
        token.mint(address(0x1337), 100);
        vm.stopPrank();
    }
}
