// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVEToken.sol";

contract MOVETokenTest is Test {
    function testInitialize() public {
        MOVEToken token = new MOVEToken();

        // Call the initialize function
        token.initialize();

        // Check the token details
        assertEq(token.name(), "Move Token");
        assertEq(token.symbol(), "MOVE");
    }

    function testCannotInitializeTwice() public {
        MOVEToken token = new MOVEToken();

        // Call the initialize function
        token.initialize();

        // Initialize the contract
        vm.expectRevert(0xf92ee8a9);
        token.initialize();
    }

    function testGrants() public {
        MOVEToken token = new MOVEToken();

        // Call the initialize function
        token.initialize();

        // Check the token details
        assertEq(token.hasRole(token.MINTER_ROLE(), address(this)), true);
    }

    function testMint() public {
        MOVEToken token = new MOVEToken();

        // Call the initialize function
        token.initialize();
        uint256 initialBalance = token.balanceOf(address(0x1337));
        // Mint tokens
        token.mint(address(0x1337), 100);

        // Check the token details
        assertEq(token.balanceOf(address(0x1337)), initialBalance + 100);
    }
}
