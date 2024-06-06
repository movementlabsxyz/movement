// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/staking/base/BaseStaking.sol";

contract BaseStakingTest is Test {

    function testInitialize() public {

        BaseStaking staking = new BaseStaking();

        // Call the initialize function
        staking.initialize();

    }

    function testCannotInitializeTwice() public {

        BaseStaking staking = new BaseStaking();

        // Initialize the contract
        staking.initialize();

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        staking.initialize();
    }
}