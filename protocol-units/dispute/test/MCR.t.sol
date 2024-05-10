// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/MCR.sol";

contract MCRTest is Test {
    MCR public mcr;
    uint256 public epochDuration = 7 days;

    function setUp() public {
        mcr = new MCR(1 days, 100 ether, epochDuration / 1 days);
    }

    function testUpdateEpoch() public {
        // Test initial epoch
        assertEq(mcr.currentEpoch(), 0);
        assertEq(mcr.epochStartTimestamp(), block.timestamp);

        // Advance time by 3 epochs
        vm.warp(block.timestamp + 3 * epochDuration);

        // Call updateEpoch and check updated values
        mcr.updateEpoch();
        assertEq(mcr.currentEpoch(), 3);
        assertEq(mcr.epochStartTimestamp(), block.timestamp);

        // Advance time by 1 epoch and 1 day
        vm.warp(block.timestamp + epochDuration + 1 days);

        // Call updateEpoch and check updated values
        mcr.updateEpoch();
        assertEq(mcr.currentEpoch(), 4);
        assertEq(mcr.epochStartTimestamp(), block.timestamp - 1 days);
    }
}