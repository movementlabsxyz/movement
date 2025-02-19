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

    /// @notice Test that the staking works as expected
    function testSimpleStaking() public {
        console.log("Starting testSimpleStaking");
        
        // three well-funded signers
        address payable alice = payable(vm.addr(1));
        console.log("Created alice address:", address(alice));
        
        console.log("Whitelisting alice...");
        staking.whitelistAddress(alice);
        console.log("Alice whitelisted");
        
        console.log("Minting tokens for alice...");
        moveToken.mint(alice, 100);
        console.log("Alice token balance:", moveToken.balanceOf(alice));

        address payable bob = payable(vm.addr(2));
        console.log("Created bob address:", address(bob));
        staking.whitelistAddress(bob);
        moveToken.mint(bob, 100);
        console.log("Bob token balance:", moveToken.balanceOf(bob));

        address payable carol = payable(vm.addr(3));
        console.log("Created carol address:", address(carol));
        moveToken.mint(carol, 100);
        staking.whitelistAddress(carol);
        console.log("Carol token balance:", moveToken.balanceOf(carol));

        console.log("\nStarting genesis ceremony participation...");
        
        // have them participate in the genesis ceremony
        console.log("Alice approving tokens...");
        vm.prank(alice);
        moveToken.approve(address(staking), 100);
        
        console.log("Alice staking...");
        vm.prank(alice);
        staking.stake(address(mcr), moveToken, 33);
        uint256 aliceStake = mcr.getStakeForAcceptingEpoch(address(moveToken), alice);
        console.log("Alice staked amount:", aliceStake);
        assertEq(aliceStake, 33, "Alice's stake amount not correctly recorded");
        

        console.log("\nBob approving and staking...");
        vm.prank(bob);
        moveToken.approve(address(staking), 100);
        vm.prank(bob);
        staking.stake(address(mcr), moveToken, 33);
        uint256 bobStake = mcr.getStakeForAcceptingEpoch(address(moveToken), bob);
        console.log("Bob staked amount:", bobStake);
        assertEq(bobStake, 33, "Bob's stake amount not correctly recorded");

        console.log("\nCarol approving and staking...");
        vm.prank(carol);
        moveToken.approve(address(staking), 100);
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 34);
        uint256 carolStake = mcr.getStakeForAcceptingEpoch(address(moveToken), carol);
        console.log("Carol staked amount:", carolStake);
        assertEq(carolStake, 34, "Carol's stake amount not correctly recorded");

        // check that the total stake is 100
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), 100, "Total stake should be 100");
        console.log("Total stake:", mcr.getTotalStakeForAcceptingEpoch());

        // log the attester list
        address[] memory attesters = staking.getStakedAttestersForAcceptingEpoch(address(mcr));
        console.log("Attesters:", attesters.length);
        assertEq(attesters.length, 3, "There should be 3 attesters");
        for (uint256 i = 0; i < attesters.length; i++) {
            console.log("Attester:", attesters[i]);
        }

        // end the genesis ceremony
        console.log("\nAccepting genesis ceremony...");
        mcr.acceptGenesisCeremony();
        console.log("Genesis ceremony accepted");


        // check if there is a postconfirmed superblock
        uint256 initHeight = mcr.getLastPostconfirmedSuperBlockHeight();
        console.log("Last postconfirmed superblock height:", initHeight);
        // retrieve the postconfirmed superblock at that height
        MCRStorage.SuperBlockCommitment memory retrievedCommitmentEmpty = mcr.getPostconfirmedCommitment(initHeight);
        console.log("Retrieved commitment height:", retrievedCommitmentEmpty.height);
        console.log("Retrieved commitment:", uint256(retrievedCommitmentEmpty.commitment));
        
        // make a superBlock commitment
        uint256 targetHeight = 1;
        console.log("\nMaking superBlock commitment...");
        MCRStorage.SuperBlockCommitment memory initCommitment = MCRStorage.SuperBlockCommitment({
            height: targetHeight,
            commitment: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3))),
            blockId: keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)))
        });
        
        console.log("Alice submitting commitment...");
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(initCommitment);
        // retrieve the commitment at height 1 (the one we submitted)
        MCRStorage.SuperBlockCommitment memory retrievedCommitmentAlice = mcr.getCommitmentByAttester(targetHeight, alice);
        console.log("Alice commitment :", uint256(retrievedCommitmentAlice.commitment));
        assert(retrievedCommitmentAlice.commitment == initCommitment.commitment);
        
        console.log("Bob submitting commitment...");
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(initCommitment);
        // retrieve the commitment
        MCRStorage.SuperBlockCommitment memory retrievedCommitmentBob = mcr.getCommitmentByAttester(targetHeight, bob);
        console.log("Bob commitment: ", uint256(retrievedCommitmentBob.commitment));
        assert(retrievedCommitmentBob.commitment == initCommitment.commitment);

        // who is the current acceptor?
        // TODO: add acceptor role and check it is alice
        // console.log("Current acceptor:", mcr.getCurrentAcceptor());

        // check if acceptor is live, note that currentAcceptorIsLive() returns bool
        console.log("Current acceptor is live:", mcr.currentAcceptorIsLive());
        assert(mcr.currentAcceptorIsLive());
        // check what is the assigned epoch for the height targetHeight
        console.log("Assigned epoch for targetHeight:", mcr.getSuperBlockHeightAssignedEpoch(1));
        console.log("Accepting epoch:", mcr.getAcceptingEpoch());
        console.log("Present epoch:", mcr.getPresentEpoch());
        // in this setup the accepting epoch is the same as the assigned epoch for the targetHeight
        assertEq(mcr.getSuperBlockHeightAssignedEpoch(1), mcr.getAcceptingEpoch());


        // alice postconfirms the superblock using attemptPostconfirm
        console.log("Alice postconfirming...");
        vm.prank(alice);
        // can we make this so we catch any errors from
        mcr.postconfirmSuperBlocks();


        // check that the commitment is postconfirmed at targetHeight
        MCRStorage.SuperBlockCommitment memory retrievedCommitmentAlicePostconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        console.log("Alice postconfirmed - height:", retrievedCommitmentAlicePostconfirmed.height);
        console.log("Alice postconfirmed - commitment:", uint256(retrievedCommitmentAlicePostconfirmed.commitment));
        // check that the heights are correct
        uint256 newHeight = mcr.getLastPostconfirmedSuperBlockHeight();
        console.log("Last postconfirmed superblock height:", newHeight);
        assertEq(newHeight, initHeight + 1);
        assertEq(targetHeight, newHeight);
        
        assert(retrievedCommitmentAlicePostconfirmed.commitment == initCommitment.commitment);
        assert(retrievedCommitmentAlicePostconfirmed.height == targetHeight);
        
        // now we move to block 2 and make some commitment just to trigger the epochRollover
        console.log("\nRetrieving commitment...");
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(targetHeight);
        console.log("Retrieved commitment height:", retrievedCommitment.height);
        console.log("Expected height:", initCommitment.height);
        console.log("Retrieved commitment:", toHexString(abi.encode(retrievedCommitment.commitment)));
        console.log("Expected commitment:", toHexString(abi.encode(initCommitment.commitment)));
        
        assert(retrievedCommitment.commitment == initCommitment.commitment);
        assert(retrievedCommitment.blockId == initCommitment.blockId);
        assert(retrievedCommitment.height == targetHeight);
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
}