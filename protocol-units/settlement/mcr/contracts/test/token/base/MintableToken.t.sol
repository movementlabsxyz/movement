// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/token/base/MintableToken.sol";

contract MintableTokenTest is Test {

    function testInitialize() public {

        MintableToken token = new MintableToken();

        // Call the initialize function
        token.initialize("Base Token", "BASE");

        // Check the token details
        assertEq(token.name(), "Base Token");
        assertEq(token.symbol(), "BASE");
    }

    function testCannotInitializeTwice() public {

        MintableToken token = new MintableToken();

        // Initialize the contract
        token.initialize("Base Token", "BASE");

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        token.initialize("Base Token", "BASE");
    }
}