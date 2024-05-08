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

        console2.log("verified!");
        mangled.claim.postStateDigest ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled postStateDigest value");
        mangled = TEST_RECEIPT;

        mangled.claim.output ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled input value");
        mangled = TEST_RECEIPT;
    }

function testHonestValidatorsSubmittingValidCommitments() public {
    // Register multiple validators
    uint256 numValidators = 5;
    address[] memory validators = new address[](numValidators);
    for (uint256 i = 0; i < numValidators; i++) {
        address validator = address(uint160(i + 1));
        validators[i] = validator;
        vm.deal(validator, rStarM.MIN_STAKE());
        vm.prank(validator);
        rStarM.stake{value: rStarM.MIN_STAKE()}();
    }

    // Have each validator submit a valid commitment for the same block hash
    bytes32 blockHash = keccak256(abi.encodePacked("testBlock"));
    bytes memory stateCommitment = abi.encodePacked("validStateCommitment");
    uint256 initialRound = rStarM.currentRound();

    for (uint256 i = 0; i < numValidators; i++) {
        vm.prank(validators[i]);
        rStarM.submitOptimisticCommitment(blockHash, stateCommitment);
    }
    
    bool isAccepted = rStarM.isCommitmentAccepted(initialRound);
    bytes memory highestCommitState = rStarM.getCommitmentHighestCommitState(initialRound);
    // Use these values in your assertions
    assertTrue(isAccepted, "Block should be accepted");

    // Check that the accepted state commitment matches the one submitted by the validators
    //assertEq(highestCommitState, stateCommitment, "Accepted state commitment should match the submitted commitment");

    }
}
