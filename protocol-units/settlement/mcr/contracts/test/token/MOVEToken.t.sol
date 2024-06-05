// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/MOVEToken.sol";

contract MOVETokenTest is Test {
    MOVEToken token;

    function setUp() public {
        token = new MOVEToken();
    }

    function testInitialize() public {
        // Ensure the contract is not initialized yet
        vm.expectRevert("Initializable: contract is already initialized");
        token.initialize();

        // Call the initialize function
        token.initialize();

        // Check the token details
        assertEq(token.name(), "Move Token");
        assertEq(token.symbol(), "MOVE");
    }

    function testCannotInitializeTwice() public {
        // Initialize the contract
        token.initialize();

        // Attempt to initialize again should fail
        vm.expectRevert("Initializable: contract is already initialized");
        token.initialize();
    }
}