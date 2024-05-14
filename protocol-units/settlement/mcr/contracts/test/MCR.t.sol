// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/MCR.sol";

contract MCRTest is Test {

    // check that the genesis ceremony works
    function testGenesisCeremony() public {

        MCR mcr = new MCR(
            5, 
            128,
            100 ether, // should accumulate 100 ether
            0
        );

        // three well-funded signers
        address payable signer1 = payable(vm.addr(1)); 
        vm.deal(signer1, 100 ether);
        address payable signer2 = payable(vm.addr(2));
        vm.deal(signer2, 100 ether);
        address payable signer3 = payable(vm.addr(3));
        vm.deal(signer3, 100 ether);

        // signer1 genesisStakes 33 ether
        vm.prank(signer1);
        mcr.stakeGenesis{value : 33 ether}();
        assert(mcr.hasGenesisCeremonyEnded() == false);

        // signer1 should not be able to do normal staking
        vm.prank(signer1);
        vm.expectRevert("Genesis ceremony has not ended.");
        mcr.stake{value : 33 ether}();

        // signer1 should not be able to unstake
        vm.prank(signer1);
        vm.expectRevert("Genesis ceremony has not ended.");
        mcr.unstake(33 ether);

        vm.prank(signer2);
        mcr.stakeGenesis{value : 10 ether}();
        assert(mcr.hasGenesisCeremonyEnded() == false);

        // reup the stake
        vm.prank(signer1);
        mcr.stakeGenesis{value : 10 ether}();
        assert(mcr.hasGenesisCeremonyEnded() == false);

        // add a third signer for good measure
        vm.prank(signer3);
        mcr.stakeGenesis{value : 20 ether}();
        assert(mcr.hasGenesisCeremonyEnded() == false);

        // current epoch should be 0
        assert(mcr.getCurrentEpoch() == 0);

        // now finish with signer 1
        vm.warp(100);
        vm.prank(signer1);
        mcr.stakeGenesis{value : 40 ether}();
        assert(mcr.hasGenesisCeremonyEnded() == true);

        // now check the current epoch has been set to the block time / 5
        assert(mcr.getCurrentEpoch() == block.timestamp / 5);

        // now check the stake of the genesis signers
        assert(mcr.getCurrentEpochStake(signer1) == 83 ether);
        assert(mcr.getCurrentEpochStake(signer2) == 10 ether);
        assert(mcr.getCurrentEpochStake(signer3) == 20 ether);

        // now assert that genesis stake fails
        vm.prank(signer1);
        vm.expectRevert("Genesis ceremony has ended.");
        mcr.stakeGenesis{value : 1 ether}();

    }

    function testSimpleStaking() public {

        MCR mcr = new MCR(
            5, // 5 second block time 
            128,
            100 ether, // should accumulate 100 ether
            0
        );

        vm.warp(5);

        // three well-funded signers
        address payable signer1 = payable(vm.addr(1)); 
        vm.deal(signer1, 100 ether);
        address payable signer2 = payable(vm.addr(2));
        vm.deal(signer2, 100 ether);
        address payable signer3 = payable(vm.addr(3));
        vm.deal(signer3, 100 ether);

        // have them participate in the genesis ceremony
        vm.prank(signer1);
        mcr.stakeGenesis{value : 34 ether}();
        vm.prank(signer2);
        mcr.stakeGenesis{value : 33 ether}();
        vm.prank(signer3);
        mcr.stakeGenesis{value : 33 ether}();

        // now we should be in epoch 1
        assert(mcr.getCurrentEpoch() == 1);

        // now we should be able to stake
        vm.prank(signer1);
        mcr.stake{value : 10 ether}();
        assert(mcr.getCurrentEpochStake(signer1) == 34 ether);
        assert(mcr.getStakeAtEpoch(signer1, 2) == 10 ether); // stake will not have rolle over yet
        assert(signer1.balance == 56 ether);

        // signer2 is going to unstake a reasonable amount
        vm.prank(signer2);
        mcr.unstake(10 ether);
        // balance should be the same until the epoch has ticked over
        assert(mcr.getCurrentEpochStake(signer2) == 33 ether);

        // now we construct a supermajority commitment
        MCR.BlockCommitment memory bc1 = MCR.BlockCommitment({
            height : 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(signer1);
        mcr.submitBlockCommitment(bc1);
        vm.prank(signer2);
        mcr.submitBlockCommitment(bc1);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        vm.warp(10);
        MCR.BlockCommitment memory bc2 = MCR.BlockCommitment({
            height : 2,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(signer1);
        mcr.submitBlockCommitment(bc2);

        // now we should be in epoch 2
        assert(mcr.getCurrentEpoch() == 2);

        // we should now see that the stake has rolled over
        assert(mcr.getCurrentEpochStake(signer1) == 44 ether);

        // we should now see that the unstake has been processed
        assert(mcr.getCurrentEpochStake(signer2) == 23 ether);
        assert(signer2.balance == 77 ether);

        // finally, we should see that the commitment has been accepted
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).commitment == bc1.commitment);
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).blockId == bc1.blockId);
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).height == 1);

    }

    function testDishonestValidator() public {

        MCR mcr = new MCR(
            5, // 5 second block time 
            128,
            100 ether, // should accumulate 100 ether
            0
        );

        vm.warp(5);

        // three well-funded signers
        address payable signer1 = payable(vm.addr(1)); 
        vm.deal(signer1, 100 ether);
        address payable signer2 = payable(vm.addr(2));
        vm.deal(signer2, 100 ether);
        address payable signer3 = payable(vm.addr(3));
        vm.deal(signer3, 100 ether);

        // have them participate in the genesis ceremony
        vm.prank(signer1);
        mcr.stakeGenesis{value : 34 ether}();
        vm.prank(signer2);
        mcr.stakeGenesis{value : 33 ether}();
        vm.prank(signer3);
        mcr.stakeGenesis{value : 33 ether}();

        // signer3 will be dishonest
        MCR.BlockCommitment memory dishonestCommitment = MCR.BlockCommitment({
            height : 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        vm.prank(signer3);
        mcr.submitBlockCommitment(dishonestCommitment);

        // signer3 will try to sign again
        vm.prank(signer3);
        vm.expectRevert("Validator has already committed to a block at this height");
        mcr.submitBlockCommitment(dishonestCommitment);

        // signer1 and signer2 will be honest
        MCR.BlockCommitment memory honestCommitment = MCR.BlockCommitment({
            height : 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(signer1);
        mcr.submitBlockCommitment(honestCommitment);
        vm.prank(signer2);
        mcr.submitBlockCommitment(honestCommitment);

        // the honest block should be accepted
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).commitment == honestCommitment.commitment);
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).blockId == honestCommitment.blockId);
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).height == 1);

    }

    address[] honestSigners = new address[](0);
    address[] dishonestSigners = new address[](0);

    function testChangingValidatorSet() public {

        vm.pauseGasMetering();

        uint256 blockTime = 5;
        MCR mcr = new MCR(
            5, // 5 second block time 
            128,
            100 ether, // should accumulate 100 ether
            0
        );

        vm.warp(blockTime);

        // three well-funded signers
        address payable signer1 = payable(vm.addr(1)); 
        vm.deal(signer1, 100 ether);
        address payable signer2 = payable(vm.addr(2));
        vm.deal(signer2, 100 ether);
        address payable signer3 = payable(vm.addr(3));
        vm.deal(signer3, 100 ether);

        // have them participate in the genesis ceremony
        vm.prank(signer1);
        mcr.stakeGenesis{value : 34 ether}();
        vm.prank(signer2);
        mcr.stakeGenesis{value : 33 ether}();
        vm.prank(signer3);
        mcr.stakeGenesis{value : 33 ether}();

        // honest signers
        honestSigners.push(signer1);
        honestSigners.push(signer2);

        // dishonest signers
        dishonestSigners.push(signer3);

        for (uint i = 0; i < 50; i++) {

            for(uint j = 0; j < 10; j++) {

                uint256 blockHeight = i * 10 + j + 1;
                blockTime += 1;
                vm.warp(blockTime);

                // commit dishonestly
                MCR.BlockCommitment memory dishonestCommitment = MCR.BlockCommitment({
                    height : blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
                    blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
                });
                for (uint k = 0; k < dishonestSigners.length/2; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                // commit honestly
                MCR.BlockCommitment memory honestCommitment = MCR.BlockCommitment({
                    height : blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
                    blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
                });
                for (uint k = 0; k < honestSigners.length; k++) {
                    vm.prank(honestSigners[k]);
                    mcr.submitBlockCommitment(honestCommitment);
                }

                // commit dishonestly some more
                for (uint k = dishonestSigners.length/2; k < dishonestSigners.length; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                MCR.BlockCommitment memory acceptedCommitment = mcr.getAcceptedCommitmentAtBlockHeight(blockHeight);
                assert(acceptedCommitment.commitment == honestCommitment.commitment);
                assert(acceptedCommitment.blockId == honestCommitment.blockId);
                assert(acceptedCommitment.height == blockHeight);

            }

            // add a new signer
            address payable newSigner = payable(vm.addr(4 + i));
            vm.deal(newSigner, 100 ether);
            vm.prank(newSigner);
            mcr.stake{value : 33 ether}();

            if (i % 3 == 2) {
                dishonestSigners.push(newSigner);
            } else {
                honestSigners.push(newSigner);
            }

            if(i % 5 == 4) {
                // remove a dishonest signer
                address dishonestSigner = dishonestSigners[0];
                vm.prank(dishonestSigner);
                mcr.unstake(33 ether);
                dishonestSigners[0] = dishonestSigners[dishonestSigners.length - 1];
                dishonestSigners.pop();
            }

            if(i % 8 == 7) {
               // remove an honest signer
                address honestSigner = honestSigners[0];
                vm.prank(honestSigner);
                mcr.unstake(33 ether);
                honestSigners[0] = honestSigners[honestSigners.length - 1];
                honestSigners.pop();
            }

            blockTime += 5;
            vm.warp(blockTime);

        }

    }

}