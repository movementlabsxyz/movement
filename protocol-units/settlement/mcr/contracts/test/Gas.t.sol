// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import "forge-std/Test.sol";
import "../src/MCR.sol";
import "forge-std/console.sol";

contract GasTest is Test {
    // forge test --match-test [testName] --gas-report -vvv
    // forge test --gas-report -vvv

    mapping(uint256 => uint256) public gasConsumption;
    MCR public mcr;

    function setUp() public {
        mcr = new MCR(5, 128, 100 ether, 0);
    }

    function testGetFunctions() public {
        uint256 snapshot = vm.snapshot();
        gasConsumption[0] = mcr.genesisStakeRequired();
        vm.revertTo(snapshot);
        gasConsumption[0] = mcr.getGenesisStakeRequired();
        // return state is cheaper
    }

    function testLesserVSGreaterEq() public {
        uint256 snapshot = vm.snapshot();
        mcr.stakeGenesis();
        vm.revertTo(snapshot);
        mcr.stakeGenesisGreaterEq();
        // >= or <= results in more computing as it evaluates twice
    }

    function testMapReturnType() public {
        uint256 snapshot = vm.snapshot();
        // Returns distinct BlockCommitment
        MCR.BlockCommitment memory getReturn = mcr.getAcceptedCommitmentAtBlockHeight(0);
        vm.revertTo(snapshot);
        // Returns singleton
        (uint256 height, bytes32 commitment, bytes32 blockId) = mcr.acceptedBlocks(0);
        // Singleton is cheaper, maybe because of not using a public function
        // but we might want to keep using Types instead of singletons
    }

    function testRevert() public {
        uint256 snapshot = vm.snapshot();
        vm.expectRevert();
        mcr.revertRequire();
        vm.revertTo(snapshot);
        vm.expectRevert();
        mcr.revertCustom();
        // Custom is cheaper
    }

    function testReading() public {
        uint256 snapshot = vm.snapshot();
        mcr.getTotalStakeForCurrentEpoch();
        vm.revertTo(snapshot);
        mcr.getFuncMemory();
        vm.revertTo(snapshot);
        mcr.getInternal();
        vm.revertTo(snapshot);
        mcr.getInternalMemory();
        vm.revertTo(snapshot);
        mcr.getState();
        vm.revertTo(snapshot);
        mcr.getStateMemory();
        vm.revertTo(snapshot);
        mcr.getMultiuseState();
        vm.revertTo(snapshot);
        uint256 snapshot7 = vm.snapshot();
        mcr.getMultiuseMemory();
        
        // apparently currrent implenetation beats everything
        // this is counter intuitive for me.
        // if a state is being read/written multiple times, set to memory first then write to storage.

    }

    function testDifferentHigher() public {
        uint256 snapshot = vm.snapshot();

        vm.expectRevert();
        mcr.differentCheck();
        vm.revertTo(snapshot);

        vm.expectRevert();
        mcr.higherCheck();
        // Higher wins
    }

    function testInitializingToZero() public {
        uint256 snapshot = vm.snapshot();
        mcr.getTotalStakeForEpoch(0);
        vm.revertTo(snapshot);
        mcr.getTotalNotInitiateZero(0);
        // Not initializing to zero is cheaper
    }

    // Test using Struct as parameter vs decomposing it prior to using it.
    function testStructDecomposing() public {
        uint256 snapshot = vm.snapshot();
        mcr.returnStruct(0, address(0x0));
        vm.revertTo(snapshot);
        mcr.returnDecomposed(0, address(0x0));
        // Struct is cheaper
    }
}
