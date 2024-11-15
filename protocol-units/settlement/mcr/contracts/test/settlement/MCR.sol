// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/staking/MovementStaking.sol";
import "../../src/token/MOVETokenDev.sol";
import "../../src/settlement/MCR.sol";
import "../../src/settlement/MCRStorage.sol";
import "../../src/settlement/interfaces/IMCR.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";

contract MCRTest is Test, IMCR {
    MOVETokenDev public moveToken;
    MovementStaking public staking;
    MCR public mcr;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(string,string)";
    string public stakingSignature = "initialize(address)";
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[])";

    function setUp() public {
        MOVETokenDev moveTokenImplementation = new MOVETokenDev();
        MovementStaking stakingImplementation = new MovementStaking();
        MCR mcrImplementation = new MCR();

        // Contract MCRTest is the admin
        admin = new ProxyAdmin(address(this));

        // Deploy proxies
        TransparentUpgradeableProxy moveProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation),
            address(admin),
            abi.encodeWithSignature(moveSignature, "Move Token", "MOVE")
        );
        TransparentUpgradeableProxy stakingProxy = new TransparentUpgradeableProxy(
            address(stakingImplementation),
            address(admin),
            abi.encodeWithSignature(stakingSignature, IMintableToken(address(moveProxy)))
        );
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveProxy);
        TransparentUpgradeableProxy mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(admin),
            abi.encodeWithSignature(mcrSignature, stakingProxy, 0, 5, 10 seconds, custodians)
        );
        moveToken = MOVETokenDev(address(moveProxy));
        staking = MovementStaking(address(stakingProxy));
        mcr = MCR(address(mcrProxy));
        mcr.setOpenAttestationEnabled(true);
    }

    function testCannotInitializeTwice() public {
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians);
    }

    function testSimpleStaking() public {
        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        staking.whitelistAddress(bob);
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);
        staking.whitelistAddress(carol);

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
        mcr.acceptGenesisCeremony();

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc1);
        vm.prank(bob);
        mcr.submitBlockCommitment(bc1);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        (uint256 height, bytes32 commitment, bytes32 blockId) = mcr.acceptedBlocks(1);
        assert(commitment == bc1.commitment);
        assert(blockId == bc1.blockId);
        assert(height == 1);
    }

    function testDishonestValidator() public {
        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        moveToken.mint(bob, 100);
        staking.whitelistAddress(bob);
        address payable carol = payable(vm.addr(3));
        moveToken.mint(carol, 100);
        staking.whitelistAddress(carol);

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
        mcr.acceptGenesisCeremony();

        // carol will be dishonest
        MCRStorage.BlockCommitment memory dishonestCommitment = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        vm.prank(carol);
        mcr.submitBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(AttesterAlreadyCommitted.selector);
        mcr.submitBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc1);
        vm.prank(bob);
        mcr.submitBlockCommitment(bc1);

        (uint256 height, bytes32 commitment, bytes32 blockId) = mcr.acceptedBlocks(1);
        // now we move to block 2 and make some commitment just to trigger the epochRollover
        assert(commitment == bc1.commitment);
        assert(blockId == bc1.blockId);
        assert(height == 1);
    }

    function testRollsOverHandlingDishonesty() public {
        vm.warp(300 seconds);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        staking.whitelistAddress(bob);
        moveToken.mint(bob, 100);
        address payable carol = payable(vm.addr(3));
        staking.whitelistAddress(carol);
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
        mcr.acceptGenesisCeremony();

        // carol will be dishonest
        MCRStorage.BlockCommitment memory dishonestCommitment = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        vm.prank(carol);
        mcr.submitBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(AttesterAlreadyCommitted.selector);
        mcr.submitBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.BlockCommitment memory bc1 = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
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
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitBlockCommitment(bc2);

        // check that roll over happened
        assertEq(mcr.getCurrentEpoch(), mcr.getEpochByBlockTime());
        assertEq(mcr.getCurrentEpochStake(address(moveToken), alice), 34);
        assertEq(mcr.getCurrentEpochStake(address(moveToken), bob), 33);
        assertEq(mcr.getCurrentEpochStake(address(moveToken), carol), 33);
        (uint256 height, bytes32 commitment, bytes32 blockId) = mcr.acceptedBlocks(1);
        assert(commitment == bc1.commitment);
        assert(blockId == bc1.blockId);
        assert(height == 1);
    }

    address[] honestSigners = new address[](0);
    address[] dishonestSigners = new address[](0);

    function testChangingValidatorSet() public {
        vm.pauseGasMetering();

        uint256 blockTime = 300;

        vm.warp(blockTime);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);

        address payable bob = payable(vm.addr(2));
        staking.whitelistAddress(bob);
        moveToken.mint(bob, 100);

        address payable carol = payable(vm.addr(3));
        staking.whitelistAddress(carol);
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

        uint256 reorgs = 50;
        for (uint256 i = 0; i < reorgs; i++) {
            uint256 commitmentHeights = 10;
            for (uint256 j = 0; j < commitmentHeights; j++) {
                uint256 blockHeight = i * 10 + j + 1;
                blockTime += 1;
                vm.warp(blockTime);

                // commit dishonestly
                MCRStorage.BlockCommitment memory dishonestCommitment = MCRStorage.BlockCommitment({
                    height: blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
                    blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
                });
                for (uint256 k = 0; k < dishonestSigners.length / 2; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                // commit honestly
                MCRStorage.BlockCommitment memory honestCommitment = MCRStorage.BlockCommitment({
                    height: blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
                    blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
                });
                for (uint256 k = 0; k < honestSigners.length; k++) {
                    vm.prank(honestSigners[k]);
                    mcr.submitBlockCommitment(honestCommitment);
                }

                // commit dishonestly some more
                for (uint256 k = dishonestSigners.length / 2; k < dishonestSigners.length; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitBlockCommitment(dishonestCommitment);
                }

                (uint256 height, bytes32 commitment, bytes32 blockId) = mcr.acceptedBlocks(blockHeight);
                assert(commitment == honestCommitment.commitment);
                assert(blockId == honestCommitment.blockId);
                assert(height == blockHeight);
            }

            // add a new signer
            address payable newSigner = payable(vm.addr(4 + i));
            staking.whitelistAddress(newSigner);
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
                dishonestSigners[0] = dishonestSigners[dishonestSigners.length - 1];
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

    function testForcedAttestation() public {
        vm.pauseGasMetering();

        uint256 blockTime = 300;

        vm.warp(blockTime);

        // three well-funded signers
        address payable alice = payable(vm.addr(1));

        // default signer should be able to force attestation
        MCRStorage.BlockCommitment memory forcedCommitment = MCRStorage.BlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        mcr.setAcceptedCommitmentAtBlockHeight(forcedCommitment);

    }
}