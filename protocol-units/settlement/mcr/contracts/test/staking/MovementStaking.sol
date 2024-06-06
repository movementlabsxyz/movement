// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/staking/MovementStaking.sol";

contract MovementStakingTest is Test {

    function testInitialize() public {

        MovementStaking staking = new MovementStaking();

        // Call the initialize function
        staking.initialize();

    }

    function testCannotInitializeTwice() public {

        MovementStaking staking = new MovementStaking();

        // Initialize the contract
        staking.initialize();

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        staking.initialize();
    }
}