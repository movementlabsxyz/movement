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
    string public moveSignature = "initialize(address)";
    string public stakingSignature = "initialize(address)";
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[],uint256)";

    function toHexString(bytes memory data) public pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes memory str = new bytes(2 + data.length * 2);
        str[0] = "0";
        str[1] = "x";
        for (uint i = 0; i < data.length; i++) {
            str[2+i*2] = alphabet[uint8(data[i] >> 4)];
            str[2+i*2+1] = alphabet[uint8(data[i] & 0x0f)];
        }
        return string(str);
    }

    function setUp() public {
        MOVETokenDev moveTokenImplementation = new MOVETokenDev();
        MovementStaking stakingImplementation = new MovementStaking();
        MCR mcrImplementation = new MCR();

        // Contract MCRTest is the admin
        admin = new ProxyAdmin(address(this));

        // Deploy proxies
        bytes memory initData = abi.encodeWithSignature(moveSignature, address(this));
        TransparentUpgradeableProxy moveProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation),
            address(admin),
            initData
        );
        // Set up the moveToken variable to interact with the proxy
        moveToken = MOVETokenDev(address(moveProxy));

        bytes memory stakingInitData = abi.encodeWithSignature(stakingSignature, IMintableToken(address(moveProxy)));
        TransparentUpgradeableProxy stakingProxy = new TransparentUpgradeableProxy(
            address(stakingImplementation),
            address(admin),
            stakingInitData
        );
        // Set up the staking variable to interact with the proxy
        staking = MovementStaking(address(stakingProxy));

        address[] memory custodians = new address[](1);
        custodians[0] = address(moveProxy);

        bytes memory mcrInitData = abi.encodeWithSignature(
            mcrSignature, 
            stakingProxy,                // address of staking contract
            0,                          // start from genesis
            5,                          // max blocks ahead of last confirmed
            10 seconds,                 // time window for block confirmation
            custodians,                 // array with moveProxy address
            120 seconds                 // how long an acceptor serves
        );
        TransparentUpgradeableProxy mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(admin),
            mcrInitData
        );

        mcr = MCR(address(mcrProxy));
        mcr.setOpenAttestationEnabled(true);
        console.log("Setup complete");
    }

    function testCannotInitializeTwice() public {
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians,120 seconds);
    }

    // A acceptor that is in place for acceptorTerm time should be replaced by a new acceptor after their term ended.
    function testAcceptorRotation() public {
        // funded signers
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);
        address payable bob = payable(vm.addr(2));
        staking.whitelistAddress(bob);
        moveToken.mint(bob, 100);

        // have them participate in the genesis ceremony
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 34);
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        // end the genesis ceremony
        mcr.acceptGenesisCeremony();

        // get the current acceptor
        assertEq(mcr.getCurrentAcceptor(), alice);
        // assert that bob is NOT the acceptor
        assertNotEq(mcr.getCurrentAcceptor(), bob);
        
        // make a block commitment
        MCRStorage.SuperBlockCommitment memory initCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(initCommitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(initCommitment);

        // TODO these tests need to be split up into different test functions (happy / unhappy path)
        // bob should not be the current acceptor
        vm.prank(bob);
        vm.expectRevert("NotAcceptor");  // Expect the "NotAcceptor" revert message
        mcr.postconfirmSuperBlocks();
        // alice can confirm the block comittment
        vm.prank(alice);
        mcr.postconfirmSuperBlocks();

        // now check the block is L1-confirmed
        // assertEq(mcr.getCurrentEpoch(), mcr.getEpochByBlockTime());


        // get to next Acceptor

        // make a block commitment with Bob

        // check that Bob is the current acceptor


    }

    /// @notice Test that the staking and postconfirmation works with multiple stakers
    function testPostconfirmationWithMultipleStakers() public {
                
        // Define stakes upfront
        uint256 aliceStakeAmount = 34;
        uint256 bobStakeAmount = 33;
        uint256 carolStakeAmount = 33;
        uint256 totalStakeAmount = aliceStakeAmount + bobStakeAmount + carolStakeAmount;
        
        // Create attesters
        address payable alice = payable(vm.addr(1));
        address payable bob = payable(vm.addr(2));
        address payable carol = payable(vm.addr(3));
        address[] memory attesters = new address[](3);
        attesters[0] = alice;
        attesters[1] = bob;
        attesters[2] = carol;

        // Setup attesters
        for (uint i = 0; i < attesters.length; i++) {
            staking.whitelistAddress(attesters[i]);
            moveToken.mint(attesters[i], totalStakeAmount);
            vm.prank(attesters[i]);
            moveToken.approve(address(staking), totalStakeAmount);
        }

        // Stake
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, aliceStakeAmount);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, bobStakeAmount);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, carolStakeAmount);

        // Verify stakes
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), aliceStakeAmount);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), bobStakeAmount);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), carolStakeAmount);
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), totalStakeAmount);

        // Verify attesters
        address[] memory stakedAttesters = staking.getStakedAttestersForAcceptingEpoch(address(mcr));
        assertEq(stakedAttesters.length, 3, "There should be 3 attesters");
        
        // End genesis ceremony
        mcr.acceptGenesisCeremony();

        // Check initial state
        uint256 initHeight = mcr.getLastPostconfirmedSuperBlockHeight();
        assertEq(initHeight, 0);
        
        // Create commitment for height 1
        uint256 targetHeight = 1;
        bytes32 commitmentHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        bytes32 blockIdHash = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
        
        MCRStorage.SuperBlockCommitment memory commitment = MCRStorage.SuperBlockCommitment({
            height: targetHeight,
            commitment: commitmentHash,
            blockId: blockIdHash
        });

        // Submit commitments
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(commitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(commitment);

        // Verify commitments were stored
        MCRStorage.SuperBlockCommitment memory aliceCommitment = mcr.getCommitmentByAttester(targetHeight, alice);
        MCRStorage.SuperBlockCommitment memory bobCommitment = mcr.getCommitmentByAttester(targetHeight, bob);
        assert(aliceCommitment.commitment == commitment.commitment);
        assert(bobCommitment.commitment == commitment.commitment);

        // Verify acceptor state
        assert(mcr.currentAcceptorIsLive());
        assertEq(mcr.getSuperBlockHeightAssignedEpoch(targetHeight), mcr.getAcceptingEpoch());

        // Attempt postconfirmation
        vm.prank(alice);
        mcr.postconfirmSuperBlocks();

        // Verify postconfirmation
        MCRStorage.SuperBlockCommitment memory postconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        assert(postconfirmed.commitment == commitment.commitment);
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), targetHeight);
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
        MCRStorage.SuperBlockCommitment memory dishonestCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(AttesterAlreadyCommitted.selector);
        mcr.submitSuperBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.SuperBlockCommitment memory initCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(initCommitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(initCommitment);

        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        // now we move to block 2 and make some commitment just to trigger the epochRollover
        assert(retrievedCommitment.commitment == initCommitment.commitment);
        assert(retrievedCommitment.blockId == initCommitment.blockId);
        assert(retrievedCommitment.height == 1);
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
        MCRStorage.SuperBlockCommitment memory dishonestCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(dishonestCommitment);

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(AttesterAlreadyCommitted.selector);
        mcr.submitSuperBlockCommitment(dishonestCommitment);

        // make a block commitment
        MCRStorage.SuperBlockCommitment memory initCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(initCommitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(initCommitment);

        // now we move to block 2 and make some commitment just to trigger the epochRollover
        vm.warp(310 seconds);

        // make a block commitment
        MCRStorage.SuperBlockCommitment memory bc2 = MCRStorage.SuperBlockCommitment({
            height: 2,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(bc2);

        // check that roll over happened
        assertEq(mcr.getAcceptingEpoch(), mcr.getPresentEpoch());
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), 34);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), 33);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), 33);
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assert(retrievedCommitment.commitment == initCommitment.commitment);
        assert(retrievedCommitment.blockId == initCommitment.blockId);
        assert(retrievedCommitment.height == 1);
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
                MCRStorage.SuperBlockCommitment memory dishonestCommitment = MCRStorage.SuperBlockCommitment({
                    height: blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
                    blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
                });
                for (uint256 k = 0; k < dishonestSigners.length / 2; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitSuperBlockCommitment(dishonestCommitment);
                }

                // commit honestly
                MCRStorage.SuperBlockCommitment memory honestCommitment = MCRStorage.SuperBlockCommitment({
                    height: blockHeight,
                    commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
                    blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
                });
                for (uint256 k = 0; k < honestSigners.length; k++) {
                    vm.prank(honestSigners[k]);
                    mcr.submitSuperBlockCommitment(honestCommitment);
                }

                // commit dishonestly some more
                for (uint256 k = dishonestSigners.length / 2; k < dishonestSigners.length; k++) {
                    vm.prank(dishonestSigners[k]);
                    mcr.submitSuperBlockCommitment(dishonestCommitment);
                }

                MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(blockHeight);
                assert(retrievedCommitment.commitment == honestCommitment.commitment);
                assert(retrievedCommitment.blockId == honestCommitment.blockId);
                assert(retrievedCommitment.height == blockHeight);
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

        // default signer should be able to force commitment
        MCRStorage.SuperBlockCommitment memory forcedCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)))
        });
        mcr.forceLatestCommitment(forcedCommitment);

        // get the latest commitment
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assertEq(retrievedCommitment.blockId, forcedCommitment.blockId);
        assertEq(retrievedCommitment.commitment, forcedCommitment.commitment);
        assertEq(retrievedCommitment.height, forcedCommitment.height);

        // create an unauthorized signer
        address payable alice = payable(vm.addr(1));

        // try to force a different commitment with unauthorized user
        MCRStorage.SuperBlockCommitment memory badForcedCommitment = MCRStorage.SuperBlockCommitment({
            height: 1,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        
        // Alice should not have COMMITMENT_ADMIN role
        assertEq(mcr.hasRole(mcr.COMMITMENT_ADMIN(), alice), false);
        
        vm.prank(alice);
        vm.expectRevert("FORCE_LATEST_COMMITMENT_IS_COMMITMENT_ADMIN_ONLY");
        mcr.forceLatestCommitment(badForcedCommitment);
    }

    /// @notice Test that a confirmation and postconfirmation by single attester works
    function testSimplePostconfirmation() public {
        // Setup - single attester
        address payable alice = payable(vm.addr(1));
        staking.whitelistAddress(alice);
        moveToken.mint(alice, 100);
        
        // Stake
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 100);
        
        // End genesis ceremony
        mcr.acceptGenesisCeremony();
        
        // confirm current superblock height
        uint256 currentHeight = mcr.getLastPostconfirmedSuperBlockHeight();

        // Create and submit commitment
        uint256 targetHeight = 1;
        MCRStorage.SuperBlockCommitment memory commitment = MCRStorage.SuperBlockCommitment({
            height: targetHeight,
            commitment: keccak256(abi.encodePacked(uint256(1))),
            blockId: keccak256(abi.encodePacked(uint256(1)))
        });

        // Submit commitment
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(commitment);
        
        // Verify commitment was stored
        MCRStorage.SuperBlockCommitment memory stored = mcr.getCommitmentByAttester(targetHeight, alice);
        assert(stored.commitment == commitment.commitment);
        
        // Attempt postconfirmation
        vm.prank(alice);
        mcr.postconfirmSuperBlocks();
        
        // Verify postconfirmation worked
        MCRStorage.SuperBlockCommitment memory postconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        assert(postconfirmed.commitment == commitment.commitment);

        // confirm current superblock height
        uint256 currentHeightNew = mcr.getLastPostconfirmedSuperBlockHeight();
        assertEq(currentHeightNew, currentHeight + 1);

    }
}