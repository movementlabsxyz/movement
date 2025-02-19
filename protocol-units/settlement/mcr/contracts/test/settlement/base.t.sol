// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./MCR.sol";

contract MCRPostconfirmationBase is MCRTest {
    // Helper function to setup genesis with 3 attesters and their stakes
    function setupGenesisWithThreeAttesters(
        uint256 aliceStakeAmount,
        uint256 bobStakeAmount, 
        uint256 carolStakeAmount
    ) internal returns (address alice, address bob, address carol) {
        uint256 totalStakeAmount = aliceStakeAmount + bobStakeAmount + carolStakeAmount;
        
        // Create attesters
        alice = payable(vm.addr(1));
        bob = payable(vm.addr(2));
        carol = payable(vm.addr(3));
        address[] memory attesters = new address[](3);
        attesters[0] = alice;
        attesters[1] = bob;
        attesters[2] = carol;

        // Setup attesters
        for (uint i = 0; i < attesters.length; i++) {
            staking.whitelistAddress(attesters[i]);
            moveToken.mint(attesters[i], totalStakeAmount);
            vm.prank(attesters[i]);
            moveToken.approve(address(staking), totalStakeAmount);
        }

        // Stake
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, aliceStakeAmount);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, bobStakeAmount);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, carolStakeAmount);

        // Verify stakes
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), aliceStakeAmount);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), bobStakeAmount);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), carolStakeAmount);
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), totalStakeAmount);

        // End genesis ceremony
        mcr.acceptGenesisCeremony();
    }
} 