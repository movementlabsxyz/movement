// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVETokenDev.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {console} from "forge-std/console.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

contract MOVETokenDevTest is Test {
    MOVETokenDev public token;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    address public multisig = 0x00db70A9e12537495C359581b7b3Bc3a69379A00;
    bytes32 public MINTER_ROLE;
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

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
        vm.expectRevert();
        token.initialize(multisig);
        vm.stopPrank();
    }

    function testGrants() public view {
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

    // Tests that non-admin accounts cannot grant roles by checking for the expected revert
    function testCannotGrantRoleFuzz(address messenger, address receiver) public {

        // repeat with new test if messenger is multisig or 0
        vm.assume(messenger != multisig);
        vm.assume(messenger != address(0));
        console.log("............................"); // TODO : if the console logs are removed, the test fails, why?
        console.log("messenger", messenger);
        console.log("multisig", multisig);
        console.log("............................");

        // impersonate the messenger address for all subsequent calls
        vm.startPrank(messenger);

        // Expect the call to revert with AccessControlUnauthorizedAccount error
        // - messenger: the account trying to grant the role
        // - DEFAULT_ADMIN_ROLE (0x00): the role needed to grant any role
        console.log("... messenger", messenger); 
        console.logBytes32(DEFAULT_ADMIN_ROLE);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, messenger, DEFAULT_ADMIN_ROLE)
        );


        // Attempt to grant MINTER_ROLE to receiver address
        // This should fail since messenger doesn't have DEFAULT_ADMIN_ROLE
        token.grantRole(MINTER_ROLE, receiver);

        vm.stopPrank();
    }

}
