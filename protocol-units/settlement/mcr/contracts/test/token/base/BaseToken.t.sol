// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/token/base/BaseToken.sol";

contract BaseTokenTest is Test {

    function testInitialize() public {

        BaseToken token = new BaseToken();

        // Call the initialize function
        token.initialize("Base Token", "BASE");

        // Check the token details
        assertEq(token.name(), "Base Token");
        assertEq(token.symbol(), "BASE");
    }

    function testCannotInitializeTwice() public {

        BaseToken token = new BaseToken();

        // Initialize the contract
        token.initialize("Base Token", "BASE");

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        token.initialize("Base Token", "BASE");
    }
}