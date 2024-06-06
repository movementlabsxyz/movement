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

}