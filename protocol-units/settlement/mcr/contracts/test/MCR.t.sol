// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/MCR.sol";

contract MCRTest is Test {
    MCR public mcr;
    uint256 public epochDuration = 7 days;
    uint256 public supermajorityStake = 100 ether;
    uint256 public validatorCount = 5;
    address[] public validators;

    function setUp() public {
        mcr = new MCR(1 days, supermajorityStake, 7);
        validators = new address[](validatorCount);
        for (uint256 i = 0; i < validatorCount; i++) {
            validators[i] = address(uint160(i + 1));
        }
    }

    function testUpdateEpoch() public {
        // Test initial epoch
        assertEq(mcr.currentEpoch(), 0);
        assertEq(mcr.epochStartTimestamp(), 0);

        // Advance time by 3 epochs
        vm.warp(block.timestamp + 3 * epochDuration);

        // Call updateEpoch and check updated values
        mcr.updateEpoch();
        assertEq(mcr.currentEpoch(), 3);
        assertApproxEqAbs(mcr.epochStartTimestamp(), block.timestamp, 1);

        // Advance time by 1 epoch and 1 day
        vm.warp(block.timestamp + epochDuration + 1 days);

        // Call updateEpoch and check updated values
        mcr.updateEpoch();
        assertEq(mcr.currentEpoch(), 4);

        // Use the `assertApproxEqAbs` function to compare timestamps within a tolerance
        assertApproxEqAbs(mcr.epochStartTimestamp(), block.timestamp - 1 days, 1);
    }

    function testHonestValidatorsCommit() public {
        // Register validators and stake
        for (uint256 i = 0; i < validatorCount; i++) {
            vm.deal(validators[i], 25 ether);
            vm.prank(validators[i]);
            mcr.stake{value: 25 ether}();
        }

        // Check validators' stakes
        for (uint256 i = 0; i < validatorCount; i++) {
            vm.prank(validators[i]);
            (bool isRegistered, uint256 stake) = mcr.getValidatorStatus();
            assertTrue(isRegistered);
            assertEq(stake, 25 ether);
        }

        // Submit optimistic commitments
        bytes32 blockHash = keccak256(abi.encodePacked("Block 1"));
        bytes memory stateCommitment = abi.encodePacked("State 1");

        for (uint256 i = 0; i < validatorCount; i++) {
            vm.prank(validators[i]);
            mcr.submitOptimisticCommitment(blockHash, stateCommitment);
        }

        // The epoch has been updated in the previous step so we want to 
        // check that the previous epoch commitment is accepted ( - 1)
        assertTrue(mcr.isCommitmentAccepted(mcr.currentEpoch() - 1));

        // Advance to the next epoch
        vm.warp(block.timestamp + epochDuration);

        // Submit new optimistic commitments
        bytes32 newBlockHash = keccak256(abi.encodePacked("Block 2"));
        bytes memory newStateCommitment = abi.encodePacked("State 2");

        for (uint256 i = 0; i < validatorCount; i++) {
            vm.prank(validators[i]);
            mcr.submitOptimisticCommitment(newBlockHash, newStateCommitment);
        }

        // Check if the new block is accepted
        assertTrue(mcr.isCommitmentAccepted(mcr.currentEpoch() - 1));
    }

    function testDishonestValidatorsMinorityCommit() public {
        uint256 honestValidatorCount = 4;
        uint256 dishonestValidatorCount = 1;

        // Register honest validators and stake
        for (uint256 i = 0; i < honestValidatorCount; i++) {
            vm.deal(validators[i], 25 ether);
            vm.prank(validators[i]);
            mcr.stake{value: 25 ether}();
        }

        // Register dishonest validators and stake
        for (uint256 i = honestValidatorCount; i < honestValidatorCount + dishonestValidatorCount; i++) {
            vm.deal(validators[i], 25 ether);
            vm.prank(validators[i]);
            mcr.stake{value: 25 ether}();
        }

        // Submit optimistic commitments from honest validators
        bytes32 blockHash = keccak256(abi.encodePacked("Block 1"));
        bytes memory stateCommitment = abi.encodePacked("State 1");

        for (uint256 i = 0; i < honestValidatorCount; i++) {
            vm.prank(validators[i]);
            mcr.submitOptimisticCommitment(blockHash, stateCommitment);
        }

        // Submit different optimistic commitments from dishonest validators
        bytes32 dishonestBlockHash = keccak256(abi.encodePacked("Dishonest Block"));
        bytes memory dishonestStateCommitment = abi.encodePacked("Dishonest State");

        for (uint256 i = honestValidatorCount; i < honestValidatorCount + dishonestValidatorCount; i++) {
            vm.prank(validators[i]);
            mcr.submitOptimisticCommitment(dishonestBlockHash, dishonestStateCommitment);
        }

        // Check if the epoch commitment is accepted (honest validators' commitment)
        assertTrue(mcr.isCommitmentAccepted(mcr.currentEpoch() - 1));

        // Check if the dishonest validators' commitment is not accepted
        ( , , , bool isAccepted) = mcr.optimisticCommitments(dishonestBlockHash);
        assertFalse(isAccepted);
    }
}