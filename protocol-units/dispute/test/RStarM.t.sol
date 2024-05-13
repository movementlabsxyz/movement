// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Test} from "forge-std/Test.sol";
import {console2} from "forge-std/console2.sol";

import "ds-test/test.sol";
import "forge-std/console2.sol";
import "../src/RStarM.sol";
import "forge-std/Vm.sol";
import {
    IRiscZeroVerifier,
    Output,
    OutputLib,
    Receipt as RiscZeroReceipt,
    ReceiptClaim,
    ReceiptClaimLib,
    ExitCode,
    SystemExitCode
} from "../src/IRiscZeroVerifier.sol";
import {TestReceipt} from "./TestReceipt.sol";
import {ControlID} from "../src/groth16/ControlID.sol";
import "forge-std/console2.sol";

contract RStartM is DSTest {
    using OutputLib for Output;
    using ReceiptClaimLib for ReceiptClaim;

    Vm vm = Vm(HEVM_ADDRESS);
    RStarM rStarM;
    address signer1 = address(0x1);
    address signer2 = address(0x2);
    bytes exampleProofData = "exampleProof";
    address validator = address(0x1);

    RiscZeroReceipt internal TEST_RECEIPT = RiscZeroReceipt(
        TestReceipt.SEAL,
        ReceiptClaim(
            TestReceipt.IMAGE_ID,
            TestReceipt.POST_DIGEST,
            ExitCode(SystemExitCode.Halted, 0),
            bytes32(0x0000000000000000000000000000000000000000000000000000000000000000),
            Output(sha256(TestReceipt.JOURNAL), bytes32(0)).digest()
        )
    );

    function setUp() public {
        rStarM = new RStarM(1, 15, 2, ControlID.CONTROL_ID_0, ControlID.CONTROL_ID_1, ControlID.BN254_CONTROL_ID);
        vm.deal(address(this), rStarM.MIN_STAKE());
    }

    function testRegisterValidator() public {
      uint256 initialBalance = 666 ether;
      vm.deal(signer1, initialBalance);
      uint256 minStake = rStarM.MIN_STAKE();

      vm.prank(signer1);
      rStarM.stake{value: minStake}();

      (bool isRegistered, uint256 stake) = rStarM.validators(signer1);
      assertTrue(isRegistered, "Validator should be registered");
      assertEq(stake, minStake, "Validator stake should match the provided stake");
    } 

    function testVerifyKnownGoodReceipt() external view {
        require(rStarM.verify_integrity(TEST_RECEIPT), "verification failed");
    }

    function testVerifyKnownGoodImageIdAndJournal() external view {
        require(
            rStarM.verify(
                TEST_RECEIPT.seal, TestReceipt.IMAGE_ID, TEST_RECEIPT.claim.postStateDigest, sha256(TestReceipt.JOURNAL)
            ),
            "verification failed"
        );
    }

    function testVerifyMangledReceipts() external view {
        RiscZeroReceipt memory mangled = TEST_RECEIPT;

        mangled.seal[0] ^= bytes1(uint8(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled seal value");
        mangled = TEST_RECEIPT;

        mangled.claim.postStateDigest ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled postStateDigest value");
        mangled = TEST_RECEIPT;

        mangled.claim.output ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled input value");
        mangled = TEST_RECEIPT;
    }

    function testHonestValidatorsSubmittingValidCommitments() public {
        bytes32 blockHash = keccak256(abi.encodePacked("testBlock"));
        bytes memory stateCommitment = abi.encodePacked("validStateCommitment");
        uint256 initialRound = rStarM.currentRound();

        // Register multiple validators
        uint256 numValidators = 5;
        address[] memory validators = new address[](numValidators);

        for (uint256 i = 0; i < numValidators; i++) {
            address validator = address(uint160(i + 1));
            validators[i] = validator;
            vm.deal(validator, rStarM.MIN_STAKE() * 2);
            vm.prank(validator);
            uint256 balance = address(validator).balance - 100000;
            rStarM.stake{value: balance}();

            vm.prank(validator);
            (bool isRegistered, uint256 stake) = rStarM.getValidatorStatus();

            vm.prank(validator);
            rStarM.submitOptimisticCommitment(blockHash, stateCommitment);
        }

        // Get the current round after submitting optimistic commitments
        uint256 currentRound = rStarM.currentRound();
        assertTrue(currentRound > initialRound, "Current round should have increased");

        // Total validators are 5. So, the threshold is 3. So the round would have incremented by 2
        assertEq(currentRound, initialRound + 2, "Current round should have increased");

        // Calculate the previous round
        uint256 prevRound = currentRound - 1;

        // Assert that the block is accepted in the previous round
        bool isAccepted = rStarM.isCommitmentAccepted(prevRound);
        assertTrue(isAccepted, "Block should be accepted in the previous round");
    }

        function testDishonestValidatorsSubmittingInvalidCommitments() public {
    bytes32 blockHash = keccak256(abi.encodePacked("testBlock"));
    bytes memory stateCommitment1 = abi.encodePacked("validStateCommitment");
    bytes memory stateCommitment2 = abi.encodePacked("invalidStateCommitment");

    uint256 initialRound = rStarM.currentRound();

    // Register multiple validators
    uint256 numValidators = 5;
    address[] memory validators = new address[](numValidators);

    for (uint256 i = 0; i < numValidators; i++) {
        address validator = address(uint160(i + 1));
        validators[i] = validator;
        vm.deal(validator, rStarM.MIN_STAKE() * 2);
        vm.prank(validator);
        uint256 balance = address(validator).balance - 100000;
        rStarM.stake{value: balance}();
    }

    // Dishonest validators submit invalid commitments
    uint256 dishonestValidatorCount = 2;
    for (uint256 i = 0; i < dishonestValidatorCount; i++) {
        address validator = validators[i];
        vm.prank(validator);
        rStarM.submitOptimisticCommitment(blockHash, stateCommitment2);
    }

    // Honest validators submit valid commitments
    uint256 honestValidatorCount = numValidators - dishonestValidatorCount;
    for (uint256 i = dishonestValidatorCount; i < numValidators; i++) {
        address validator = validators[i];
        vm.prank(validator);
        rStarM.submitOptimisticCommitment(blockHash, stateCommitment1);
    }

    // Get the current round after submitting optimistic commitments
    uint256 currentRound = rStarM.currentRound();

    // Assert that the current round still increases 
    assertEq(currentRound, initialRound + 2, "Current round should increase");

    // Assert that the block is not accepted in the current round
    bool isAccepted = rStarM.isCommitmentAccepted(currentRound);
    assertTrue(!isAccepted, "Block should not be accepted in the current round");
}
}
