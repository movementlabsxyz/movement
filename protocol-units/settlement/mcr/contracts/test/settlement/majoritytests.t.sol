// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base.t.sol";

contract MCRPostconfirmationTest is MCRPostconfirmationBase {
    /// @notice Test that a confirmation and postconfirmation by single attester works if they have majority stake
    function testPostconfirmationWithMajorityStake() public {
        // Setup with alice having majority
        (address alice, address bob, ) = setupGenesisWithThreeAttesters(34, 33, 33);
        
        // Create commitment for height 1
        uint256 targetHeight = 1;
        bytes32 commitmentHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        bytes32 blockIdHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        
        MCRStorage.SuperBlockCommitment memory commitment = MCRStorage.SuperBlockCommitment({
            height: targetHeight,
            commitment: commitmentHash,
            blockId: blockIdHash
        });

        // Submit commitments
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(commitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(commitment);

        // Verify commitments were stored
        MCRStorage.SuperBlockCommitment memory aliceCommitment = mcr.getCommitmentByAttester(targetHeight, alice);
        MCRStorage.SuperBlockCommitment memory bobCommitment = mcr.getCommitmentByAttester(targetHeight, bob);
        assert(aliceCommitment.commitment == commitment.commitment);
        assert(bobCommitment.commitment == commitment.commitment);

        // Verify acceptor state
        assert(mcr.currentAcceptorIsLive());
        assertEq(mcr.getSuperBlockHeightAssignedEpoch(targetHeight), mcr.getAcceptingEpoch());

        // Attempt postconfirmation
        vm.prank(alice);
        mcr.postconfirmSuperBlocks();

        // Verify postconfirmation
        MCRStorage.SuperBlockCommitment memory postconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        assert(postconfirmed.commitment == commitment.commitment);
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), targetHeight);
    }

    /// @notice Test that a confirmation and postconfirmation by single attester fails if they have majority stake
    function testPostconfirmationWithoutMajorityStake() public {
        // Setup with no one having majority
        (address alice, address bob, ) = setupGenesisWithThreeAttesters(33, 33, 34);
        
        // Create commitment for height 1
        uint256 targetHeight = 1;
        bytes32 commitmentHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        bytes32 blockIdHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        
        MCRStorage.SuperBlockCommitment memory commitment = MCRStorage.SuperBlockCommitment({
            height: targetHeight,
            commitment: commitmentHash,
            blockId: blockIdHash
        });

        // Submit commitments
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(commitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(commitment);

        // Verify commitments were stored
        MCRStorage.SuperBlockCommitment memory aliceCommitment = mcr.getCommitmentByAttester(targetHeight, alice);
        MCRStorage.SuperBlockCommitment memory bobCommitment = mcr.getCommitmentByAttester(targetHeight, bob);
        assert(aliceCommitment.commitment == commitment.commitment);
        assert(bobCommitment.commitment == commitment.commitment);

        // Verify acceptor state
        assert(mcr.currentAcceptorIsLive());
        assertEq(mcr.getSuperBlockHeightAssignedEpoch(targetHeight), mcr.getAcceptingEpoch());

        // Attempt postconfirmation - this should fail because there's no supermajority
        vm.prank(alice);
        mcr.postconfirmSuperBlocks();

        // Verify height hasn't changed (postconfirmation didn't succeed)
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 0);
    }
} 