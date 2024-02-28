// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "ds-test/test.sol";
import "../src/Settlement.sol";
import "forge-std/Vm.sol";

contract SettlementTest is DSTest {
    Vm vm = Vm(HEVM_ADDRESS);
    Settlement settlement;
    address signer1 = address(0x1);
    address signer2 = address(0x2);
    bytes exampleProofData = "exampleProof";

    function setUp() public {
        settlement = new Settlement();
        settlement.addSigner(signer1);
    }

    function testAddSigner() public {
        assertTrue(settlement.isSigner(signer1), "signer1 should be a signer after addition");
    }

    function testRemoveSigner() public {
        settlement.removeSigner(signer1);
        assertTrue(!settlement.isSigner(signer1), "signer1 should not be a signer after removal");
    }

    function testFailSettleNotSigner() public {
        vm.prank(signer2);
        settlement.settle(1, exampleProofData);
    }

    function testSettleAndRetrieve() public {
        vm.prank(signer1);
        settlement.settle(1, exampleProofData);

        bytes[] memory proofs = settlement.getProofsAtHeight(1);
        assertEq(proofs.length, 1, "There should be one proof for block height 1");
        assertEq(string(proofs[0]), string(exampleProofData), "The proofData should match exampleProofData");
    }

    // Removed testGetSettlement and testFailGetLeadSettlementNoSettlements as they do not apply anymore
}