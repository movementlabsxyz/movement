// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Test} from "forge-std/Test.sol";
import {console2} from "forge-std/console2.sol";

import "ds-test/test.sol";
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
        rStarM = new RStarM(1, 15, 1, ControlID.CONTROL_ID_0, ControlID.CONTROL_ID_1);
        rStarM.registerValidator();
    }

    function testRegisterValidator() public {
      vm.deal(validator, rStarM.MIN_STAKE());
      vm.prank(validator);
      rStarM.registerValidator();

      (bool isRegistered, uint256 stake) = rStarM.validators(validator);
      assertTrue(isRegistered, "Validator should be registered");
      assertEq(stake, rStarM.MIN_STAKE(), "Validator stake should match the minimum stake");
    }

    // function testAddSigner() public {
    //     assertTrue(rStarM.isSigner(signer1), "signer1 should be a signer after addition");
    // }

    // function testRemoveSigner() public {
    //     rStarM.removeSigner(signer1);
    //     assertTrue(!rStarM.isSigner(signer1), "signer1 should not be a signer after removal");
    // }

    function testVerifyKnownGoodReceipt() external view {
        require(rStarM.verify_integrity(TEST_RECEIPT), "verification failed");
    }

    // Make sure changing the bits causes a failure.
    function testVerifyMangledReceipts() external view {
        RiscZeroReceipt memory mangled = TEST_RECEIPT;

        mangled.seal[0] ^= bytes1(uint8(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled seal value");
        mangled = TEST_RECEIPT;

        mangled.claim.preStateDigest ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled preStateDigest value");
        mangled = TEST_RECEIPT;

        mangled.claim.postStateDigest ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled postStateDigest value");
        mangled = TEST_RECEIPT;

        mangled.claim.exitCode = ExitCode(SystemExitCode.SystemSplit, 0);
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled exitCode value");
        mangled = TEST_RECEIPT;

        mangled.claim.input ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled input value");
        mangled = TEST_RECEIPT;

        mangled.claim.output ^= bytes32(uint256(1));
        require(!rStarM.verify_integrity(mangled), "verification passed on mangled input value");
        mangled = TEST_RECEIPT;

        // Just a quick sanity check
        require(rStarM.verify_integrity(mangled), "verification failed");
    }

    // function testFailSettleNotSigner() public {
    //     vm.prank(signer2);
    //     rStarM.settle(1, exampleProofData);
    // }

    // function testSettleAndRetrieve() public {
    //     vm.prank(signer1);
    //     rStarM.settle(1, exampleProofData);
    //
    //     bytes[] memory proofs = rStarM.getProofsAtHeight(1);
    //     assertEq(proofs.length, 1, "There should be one proof for block height 1");
    //     assertEq(string(proofs[0]), string(exampleProofData), "The proofData should match exampleProofData");
    // }
}
