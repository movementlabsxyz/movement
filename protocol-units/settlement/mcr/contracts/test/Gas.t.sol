// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/MCR.sol";
import "forge-std/console.sol";

contract GasTest is Test {

    // forge test --match-test [testName] --gas-report -vvv
    // forge test --gas-report -vvv

    mapping(uint256 => uint256) public gasConsumption;
    MCR public mcr;

    function setUp() public {
        mcr = new MCR(
            5, 128, 100 ether, 0
        );
    }

    function testGetFunctions() public {
        uint256 snapshot = vm.snapshot();
        gasConsumption[0] = mcr.getGenesisStakeRequired();
        vm.revertTo(snapshot);
        gasConsumption[0] = mcr.genesisStakeRequired();
        // this is crazy for me, difference is abysmal, I've never heard of it.
        // using this functions during a state write is a lot better
        // to just use the state read instead of a function
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
        mcr.getTotalStakeForCurrentEpoch();
        mcr.getFuncMemory();
        mcr.getInternal();
        mcr.getInternalMemory();
        mcr.getState();
        mcr.getStateMemory();
        mcr.getMultiuseState();
        mcr.getMultiuseMemory();
        // getState consistently better performant
        // actually this is weird, trying to figure it out why multiuseState performs better than multiuseMemory
        // Not public state getting also consistently better performant
    }

    // Test using Struct as parameter vs decomposing it prior to using it.
    function testDifferentHigher() public {
        vm.expectRevert();
        mcr.differentCheck();
        vm.expectRevert();
        mcr.higherCheck();
        // Higher wins
    }
    // 

}
