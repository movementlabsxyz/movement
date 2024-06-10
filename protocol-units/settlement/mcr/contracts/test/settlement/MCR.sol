// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/staking/MovementStaking.sol";
import "../../src/token/MOVEToken.sol";
import "../../src/settlement/MCR.sol";
import "../../src/settlement/MCRStorage.sol";
import "../../src/settlement/interfaces/IMCR.sol";

contract MCRTest is Test, IMCR {
    function testInitialize() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);
    }

    function testCannotInitializeTwice() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);
    }

    function testSimpleStaking() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);

        // have them participate in the genesis ceremony
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 34);
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        vm.prank(carol);
        moveToken.approve(address(staking), 100);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 33);

        // end the genesis ceremony
        vm.prank(address(mcr));
        staking.acceptGenesisCeremony();

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            ),
            blockId: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            )
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc1);
        vm.prank(bob);
        mcr.submitBlockCommitment(bc1);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).commitment ==
                bc1.commitment
        );
        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).blockId == bc1.blockId
        );
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).height == 1);
    }

    function testDishonestValidator() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);

        // have them participate in the genesis ceremony
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 34);
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        vm.prank(carol);
        moveToken.approve(address(staking), 100);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 33);

        // end the genesis ceremony
        vm.prank(address(mcr));
        staking.acceptGenesisCeremony();

        // carol will be dishonest
        MCRStorage.BlockCommitment memory dishonestCommitment = MCRStorage
            .BlockCommitment({
                height: 1,
                commitment: keccak256(
                    abi.encodePacked(uint256(3), uint256(2), uint256(1))
                ),
                blockId: keccak256(
                    abi.encodePacked(uint256(3), uint256(2), uint256(1))
                )
            });
        vm.prank(carol);
        mcr.submitBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(
            AttesterAlreadyCommitted.selector
        );
        mcr.submitBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            ),
            blockId: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            )
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc1);
        vm.prank(bob);
        mcr.submitBlockCommitment(bc1);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).commitment ==
                bc1.commitment
        );
        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).blockId == bc1.blockId
        );
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).height == 1);
    }

    function testRollsOverHandlingDishonesty() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        vm.warp(300 seconds);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);

        // have them participate in the genesis ceremony
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 34);
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        vm.prank(carol);
        moveToken.approve(address(staking), 100);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 33);

        // end the genesis ceremony
        vm.prank(address(mcr));
        staking.acceptGenesisCeremony();

        // carol will be dishonest
        MCRStorage.BlockCommitment memory dishonestCommitment = MCRStorage
            .BlockCommitment({
                height: 1,
                commitment: keccak256(
                    abi.encodePacked(uint256(3), uint256(2), uint256(1))
                ),
                blockId: keccak256(
                    abi.encodePacked(uint256(3), uint256(2), uint256(1))
                )
            });
        vm.prank(carol);
        mcr.submitBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(
            AttesterAlreadyCommitted.selector
        );
        mcr.submitBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            ),
            blockId: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            )
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc1);
        vm.prank(bob);
        mcr.submitBlockCommitment(bc1);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        vm.warp(310 seconds);

        // make a block commitment
        MCRStorage.BlockCommitment memory bc2 = MCRStorage.BlockCommitment({
            height: 2,
            commitment: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            ),
            blockId: keccak256(
                abi.encodePacked(uint256(1), uint256(2), uint256(3))
            )
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc2);

        // check that roll over happened
        assertEq(mcr.getCurrentEpoch(), mcr.getEpochByBlockTime());
        assertEq(mcr.getCurrentEpochStake(address(moveToken), alice), 34);
        assertEq(mcr.getCurrentEpochStake(address(moveToken), bob), 33);
        assertEq(mcr.getCurrentEpochStake(address(moveToken), carol), 33);

        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).commitment ==
                bc1.commitment
        );
        assert(
            mcr.getAcceptedCommitmentAtBlockHeight(1).blockId == bc1.blockId
        );
        assert(mcr.getAcceptedCommitmentAtBlockHeight(1).height == 1);
    }

    address[] honestSigners = new address[](0);
    address[] dishonestSigners = new address[](0);

    function testChangingValidatorSet() public {
        vm.pauseGasMetering();

        uint256 blockTime = 300;
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        MCR mcr = new MCR();
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);

        vm.warp(blockTime);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);

        // have them participate in the genesis ceremony
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 34);
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        vm.prank(carol);
        moveToken.approve(address(staking), 100);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 33);

        // honest signers
        honestSigners.push(alice);
        honestSigners.push(bob);

        // dishonest signers
        dishonestSigners.push(carol);

        uint reorgs = 50;
        for (uint i = 0; i < reorgs; i++) {
            uint commitmentHeights = 10;
            for (uint j = 0; j < commitmentHeights; j++) {
                uint256 blockHeight = i * 10 + j + 1;
                blockTime += 1;
                vm.warp(blockTime);

                // commit dishonestly
                MCRStorage.BlockCommitment
                    memory dishonestCommitment = MCRStorage.BlockCommitment({
                        height: blockHeight,
                        commitment: keccak256(
                            abi.encodePacked(uint256(3), uint256(2), uint256(1))
                        ),
                        blockId: keccak256(
                            abi.encodePacked(uint256(3), uint256(2), uint256(1))
                        )
                    });
                for (uint k = 0; k < dishonestSigners.length / 2; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                // commit honestly
                MCRStorage.BlockCommitment memory honestCommitment = MCRStorage
                    .BlockCommitment({
                        height: blockHeight,
                        commitment: keccak256(
                            abi.encodePacked(uint256(1), uint256(2), uint256(3))
                        ),
                        blockId: keccak256(
                            abi.encodePacked(uint256(1), uint256(2), uint256(3))
                        )
                    });
                for (uint k = 0; k < honestSigners.length; k++) {
                    vm.prank(honestSigners[k]);
                    mcr.submitBlockCommitment(honestCommitment);
                }

                // commit dishonestly some more
                for (
                    uint k = dishonestSigners.length / 2;
                    k < dishonestSigners.length;
                    k++
                ) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                MCR.BlockCommitment memory acceptedCommitment = mcr
                    .getAcceptedCommitmentAtBlockHeight(blockHeight);
                assert(
                    acceptedCommitment.commitment == honestCommitment.commitment
                );
                assert(acceptedCommitment.blockId == honestCommitment.blockId);
                assert(acceptedCommitment.height == blockHeight);
            }

            // add a new signer
            address payable newSigner = payable(vm.addr(4 + i));
            moveToken.mint(newSigner, 100);
            vm.prank(newSigner);
            moveToken.approve(address(staking), 33);
            vm.prank(newSigner);
            staking.stake(address(mcr), moveToken, 33);

            if (i % 3 == 2) {
                dishonestSigners.push(newSigner);
            } else {
                honestSigners.push(newSigner);
            }

            if (i % 5 == 4) {
                // remove a dishonest signer
                address dishonestSigner = dishonestSigners[0];
                vm.prank(dishonestSigner);
                staking.unstake(address(mcr), address(moveToken), 33);
                dishonestSigners[0] = dishonestSigners[
                    dishonestSigners.length - 1
                ];
                dishonestSigners.pop();
            }

            if (i % 8 == 7) {
                // remove an honest signer
                address honestSigner = honestSigners[0];
                vm.prank(honestSigner);
                staking.unstake(address(mcr), address(moveToken), 33);
                honestSigners[0] = honestSigners[honestSigners.length - 1];
                honestSigners.pop();
            }

            blockTime += 5;
            vm.warp(blockTime);
        }
    }
}
