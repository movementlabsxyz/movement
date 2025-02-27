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
import "@openzeppelin/contracts/utils/Strings.sol";


contract MCRTest is Test, IMCR {
    MOVETokenDev public moveToken;
    MovementStaking public staking;
    MCR public mcr;
    ProxyAdmin public admin;
    string public moveSignature = "initialize(address)";
    string public stakingSignature = "initialize(address)";
    string public mcrSignature = "initialize(address,uint256,uint256,uint256,address[],uint256,address)";
    uint256 epochDuration = 7200 seconds;
    uint256 acceptorTerm = epochDuration/12 seconds/4;
    bytes32 honestCommitmentTemplate = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
    bytes32 honestBlockIdTemplate = keccak256(abi.encodePacked(uint256(1), uint256(2), uint256(3)));
    bytes32 dishonestCommitmentTemplate = keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)));
    bytes32 dishonestBlockIdTemplate = keccak256(abi.encodePacked(uint256(3), uint256(2), uint256(1)));
    
    // make an honest commitment
    function makeHonestCommitment(uint256 height) internal view returns (MCRStorage.SuperBlockCommitment memory) {
        return MCRStorage.SuperBlockCommitment({
            height: height,
            commitment: honestCommitmentTemplate,
            blockId: honestBlockIdTemplate
        });
    }
       
    // make a dishonest commitment
    function makeDishonestCommitment(uint256 height) internal view returns (MCRStorage.SuperBlockCommitment memory) {
        return MCRStorage.SuperBlockCommitment({
            height: height,
            commitment: dishonestCommitmentTemplate,
            blockId: dishonestBlockIdTemplate
        });
    }


    // ----------------------------------------------------------------
    // -------- Helper functions --------------------------------------
    // ----------------------------------------------------------------

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
        // TODO while this works it is hard to access that this is the moveToken. We should not rely on the custodian array
        custodians[0] = address(moveProxy);  

        bytes memory mcrInitData = abi.encodeWithSignature(
            mcrSignature, 
            stakingProxy,               // _stakingContract, address of staking contract
            0,                          // _lastPostconfirmedSuperBlockHeight, start from genesis
            5,                          // _leadingSuperBlockTolerance, max blocks ahead of last confirmed
            epochDuration,              // _epochDuration, how long an epoch lasts, constant stakes in that time
            custodians,                 // _custodians, array with moveProxy address
            acceptorTerm,               // _acceptorTerm, how long an acceptor serves
            // TODO can we replace the following line with the moveToken address?
            address(moveProxy)           // _moveTokenAddress, the primary custodian for rewards in the staking contract
        );
        TransparentUpgradeableProxy mcrProxy = new TransparentUpgradeableProxy(
            address(mcrImplementation),
            address(admin),
            mcrInitData
        );

        mcr = MCR(address(mcrProxy));
        mcr.setOpenAttestationEnabled(true);

        // check that the setup was correctly performed
        assertEq(staking.getEpochDuration(address(mcr)), epochDuration, "Epoch duration not set correctly");
    }

    // Helper function to setup genesis with 1 attester and their stake
    function setupGenesisWithOneAttester(uint256 stakeAmount) internal returns (address attester) {
        console.log("[setupGenesisWithOneAttester] This is domain:", address(mcr));
        attester = payable(vm.addr(1));
        staking.whitelistAddress(attester);
        moveToken.mint(attester, stakeAmount);
        vm.prank(attester);
        moveToken.approve(address(staking), stakeAmount);
        vm.prank(attester);
        staking.stake(address(mcr), moveToken, stakeAmount);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), attester), stakeAmount);
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), stakeAmount);

        // TODO check why the registering did not work in the setup function
        // setup the epoch duration
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        staking.registerDomain(epochDuration, custodians);
        console.log("[setupGenesisWithThreeAttesters] Registered domain with epochDuration ", staking.getEpochDuration(address(mcr)));

        // TODO this seems odd that we need to do this here.. check for correctnes of this approach
        mcr.grantRole(mcr.DEFAULT_ADMIN_ROLE(), address(mcr));

        // attempt genesis when L1 chain has already advanced into the future
        // vm.warp(3*epochDuration);

        // End genesis ceremony
        console.log("[setupGenesisWithThreeAttesters] Ending genesis ceremony");
        vm.prank(address(mcr));
        mcr.acceptGenesisCeremony();

        // Verify stakes
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), attester), stakeAmount, "Alice's stake not correct");
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), stakeAmount, "Total stake not correct");
    }


    // Helper function to setup genesis with 3 attesters and their stakes
    function setupGenesisWithThreeAttesters(
        uint256 aliceStakeAmount,
        uint256 bobStakeAmount, 
        uint256 carolStakeAmount
    ) internal returns (address alice, address bob, address carol) {
        console.log("[setupGenesisWithThreeAttesters] This is domain:", address(mcr));
        uint256 totalStakeAmount = aliceStakeAmount + bobStakeAmount + carolStakeAmount;

        // Create attesters
        alice = payable(vm.addr(1));
        bob = payable(vm.addr(2));
        carol = payable(vm.addr(3));
        address[] memory attesters = new address[](3);
        attesters[0] = alice;
        attesters[1] = bob;
        attesters[2] = carol;

        // Setup attesters
        for (uint i = 0; i < attesters.length; i++) {
            staking.whitelistAddress(attesters[i]);
            moveToken.mint(attesters[i], totalStakeAmount); // we mint the total stake amount for each attester, just so we have some buffer
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
        string memory stakeInfo = string.concat(
            "[setupGenesisWithThreeAttesters] A/B/C/total stake: ",
            Strings.toString(mcr.getStakeForAcceptingEpoch(address(moveToken), alice)), "/",
            Strings.toString(mcr.getStakeForAcceptingEpoch(address(moveToken), bob)), "/",
            Strings.toString(mcr.getStakeForAcceptingEpoch(address(moveToken), carol)), "/",
            Strings.toString(mcr.getTotalStakeForAcceptingEpoch())
        );
        string memory balanceInfo = string.concat(
            "[setupGenesisWithThreeAttesters] A/B/C/total balance: ",
            Strings.toString(moveToken.balanceOf(alice)), "/",
            Strings.toString(moveToken.balanceOf(bob)), "/",
            Strings.toString(moveToken.balanceOf(carol)), "/",
            Strings.toString(moveToken.totalSupply())
        );
        console.log(stakeInfo);
        console.log(balanceInfo);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), aliceStakeAmount, "Alice's stake not correct");
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), bobStakeAmount, "Bob's stake not correct");
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), carolStakeAmount, "Carol's stake not correct");
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), totalStakeAmount, "Total stake not correct");

        // TODO check why the registering did not work in the setup function
        // setup the epoch duration
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        staking.registerDomain(epochDuration, custodians);
        console.log("[setupGenesisWithThreeAttesters] Registered domain with epochDuration ", staking.getEpochDuration(address(mcr)));

        // TODO this seems odd that we need to do this here.. check for correctnes of this approach
        mcr.grantRole(mcr.DEFAULT_ADMIN_ROLE(), address(mcr));

        // attempt genesis when L1 chain has already advanced into the future
        // vm.warp(3*epochDuration);

        // End genesis ceremony
        console.log("[setupGenesisWithThreeAttesters] Ending genesis ceremony");
        vm.prank(address(mcr));
        mcr.acceptGenesisCeremony();

        // Verify stakes
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), aliceStakeAmount, "Alice's stake not correct");
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), bobStakeAmount, "Bob's stake not correct");
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), carolStakeAmount, "Carol's stake not correct");
        assertEq(mcr.getTotalStakeForAcceptingEpoch(), totalStakeAmount, "Total stake not correct");
    } 

    /// @notice Helper function to setup a new signer with staking
    /// @param seed used to generate signer address
    /// @param stakeAmount Amount of tokens to stake
    /// @return newStakedAttester Address of the newly setup signer
    function newStakedAttester(uint256 seed, uint256 stakeAmount) internal returns (address) {
        address payable newAttester = payable(vm.addr(seed));
        staking.whitelistAddress(newAttester);
        moveToken.mint(newAttester, stakeAmount * 3);  // Mint 3x for flexibility    
        vm.prank(newAttester);
        moveToken.approve(address(staking), stakeAmount);
        vm.prank(newAttester);
        staking.stake(address(mcr), moveToken, stakeAmount);        
        assert(mcr.getStakeForAcceptingEpoch(address(moveToken), newAttester) == stakeAmount);

        return newAttester;
    }

    // we need this function to print the commitment in a readable format, e.g. for logging purposes
    function commitmentToHexString(bytes32 commitment) public pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes memory str = new bytes(2 + 32 * 2);
        str[0] = "0";
        str[1] = "x";
        for (uint i = 0; i < 32; i++) {
            str[2+i*2] = alphabet[uint8(commitment[i] >> 4)];
            str[2+i*2+1] = alphabet[uint8(commitment[i] & 0x0f)];
        }
        return string(str);
    }

    // this function checks if the honest attesters have a supermajority of the stake
    function logStakeInfo(address[] memory _honestAttesters, address[] memory _dishonestAttesters) internal view returns (bool) {
        // calculate the honest attesters stake
        uint256 honestStake = 0;
        for (uint256 k = 0; k < _honestAttesters.length; k++) {
            honestStake += mcr.getStakeForAcceptingEpoch(address(moveToken), _honestAttesters[k]);
        }

        // calculate the dishonest attesters stake
        uint256 dishonestStake = 0;
        for (uint256 k = 0; k < _dishonestAttesters.length; k++) {
            dishonestStake += mcr.getStakeForAcceptingEpoch(address(moveToken), _dishonestAttesters[k]);
        }
        
        uint256 supermajorityStake = 2 * (honestStake + dishonestStake) / 3 + 1;
        // create the string to print for the console log
        // string memory logString = string.concat(
        //     "have honest stake ( supermajority stake ) / dishonest stake / total stake = ",
        //     Strings.toString(honestStake), "( ", Strings.toString(supermajorityStake), " ) / ",
        //     Strings.toString(dishonestStake), " / ", Strings.toString(honestStake + dishonestStake)
        // );
        // console.log(logString);

        return honestStake >= supermajorityStake;
    }

    // remove an attester from the attesters array
    function removeAttester(address attester, address[] storage attesters, uint256 attesterStake) internal {
        vm.prank(attester);
        staking.unstake(address(mcr), address(moveToken), attesterStake);
        
        // Find and remove attester from array using swap and pop
        for (uint i = 0; i < attesters.length; i++) {
            if (attesters[i] == attester) {
                attesters[i] = attesters[attesters.length - 1];
                attesters.pop();
                break;
            }
        }
    }

    // ----------------------------------------------------------------
    // -------- Test functions ----------------------------------------
    // ----------------------------------------------------------------

    function testCannotInitializeTwice() public {
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        mcr.initialize(staking, 0, 5, 10 seconds, custodians,120 seconds, address(moveToken));
    }

    /// @notice Test that an attester cannot submit multiple commitments for the same height
    function testAttesterCannotCommitTwice() public {
        // three well-funded signers
        (, , address carol) = setupGenesisWithThreeAttesters(1, 1, 1);

        // carol will be dishonest
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));

        // carol will try to sign again
        vm.prank(carol);
        vm.expectRevert(AttesterAlreadyCommitted.selector);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));
    }

    /// @notice Test that honest supermajority succeeds despite dishonest attesters
    function testHonestSupermajoritySucceeds() public {
        // Setup with alice+bob having supermajority (67%)
        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(2, 1, 1);

        // Dishonest carol submits first
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));

        // Honest majority submits
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        vm.prank(bob); 
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));

        // Trigger postconfirmation with majority
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();

        // Verify honest commitment was postconfirmed
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assertEq(retrievedCommitment.commitment, honestCommitmentTemplate);
        assertEq(retrievedCommitment.blockId, honestBlockIdTemplate);
        assertEq(retrievedCommitment.height, 1);
    }


    /// @notice Test that no postconfirmation happens when stakes are equal
    function testNoPostconfirmationWithEqualStakes() public {
        // Setup with equal stakes (no possible supermajority)
        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(1, 1, 1);

        // Honnest commitments
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        // Dishonest commitment
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));

        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 0, "Height should not advance - Alice");
        // Verify no commitment was postconfirmed
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assertEq(retrievedCommitment.height, 0, "No commitment should be postconfirmed");
        assertEq(retrievedCommitment.commitment, bytes32(0), "No commitment should be postconfirmed");
    }

    /// @notice Test that rollover handling works with dishonesty
    function testRolloverHandlingWithDishonesty() public {
        uint256 L1BlockTimeStart = 30 * epochDuration; // TODO why though?
        vm.warp(L1BlockTimeStart);

        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(2, 1, 1);

        // dishonest carol
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));

        // honest majority
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));

        // now we move to next epoch
        vm.warp(L1BlockTimeStart + epochDuration);

        // postconfirm and rollover
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();

        // check that roll over happened
        assertEq(mcr.getAcceptingEpoch(), mcr.getPresentEpoch());
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), 2);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), bob), 1);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), 1);
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assert(retrievedCommitment.commitment == honestCommitmentTemplate);
        assert(retrievedCommitment.blockId == honestBlockIdTemplate);
        assert(retrievedCommitment.height == 1);
    }

    // State variable (at contract level)
    // dynamic array defined as state variable to permit to use push
    address[] honestAttesters = new address[](0);
    address[] dishonestAttesters = new address[](0);

    /// @notice Tests the MCR system's resilience with changing Attester sets by:
    /// 1. Starting with honest majority (2/3 honest, 1/3 dishonest)
    /// 2. Adding new attester periodically
    /// 3. Removing attester periodically
    /// 4. Verifying honest commitments prevail over 50 reorganizations
    // TODO i am not convinced we need such a complicated unit test here. Consider what this is trying to achieve and break it up.
    function testChangingAttesterSet() public {
        // TODO explain why we need to pause gas metering here
        vm.pauseGasMetering();
        uint256 attesterStake = 1; 
        uint256 L1BlockTimeStart = 30 * epochDuration; // TODO why though?
        uint256 L1BlockTime = L1BlockTimeStart;
        vm.warp(L1BlockTime);
        uint256 changingAttesterSetEvents = 10; // number of times we change the attester set
        uint256 commitmentHeights = 1; // number of commitments after each change event

        // alice needs to have attesterStake + 1 so we reach supermajority
        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(attesterStake+1, attesterStake, attesterStake);

        // honest attesters
        honestAttesters.push(alice);
        honestAttesters.push(bob);

        // dishonest attesters
        dishonestAttesters.push(carol);

        for (uint256 i = 0; i < changingAttesterSetEvents; i++) {
            for (uint256 j = 0; j < commitmentHeights; j++) {
                uint256 superBlockHeightNow = i * commitmentHeights + j + 1;

                L1BlockTime += epochDuration;
                vm.warp(L1BlockTime);
                // alice triggers rollover
                vm.prank(alice);
                mcr.postconfirmSuperBlocksAndRollover();

                // get the assigned epoch for the superblock height
                // commit roughly half of dishones attesters 
                MCRStorage.SuperBlockCommitment memory dishonestCommitment = makeDishonestCommitment(superBlockHeightNow);
                for (uint256 k = 0; k < dishonestAttesters.length / 2; k++) {
                    vm.prank(dishonestAttesters[k]);
                    mcr.submitSuperBlockCommitment(dishonestCommitment);
                }

                // commit honestly
                MCRStorage.SuperBlockCommitment memory honestCommitment = makeHonestCommitment(superBlockHeightNow);
                for (uint256 k = 0; k < honestAttesters.length; k++) {
                    vm.prank(honestAttesters[k]);
                    mcr.submitSuperBlockCommitment(honestCommitment);
                }

                // TODO: The following does not serve any purpose, as enough attesters are already committed
                // commit dishonestly the rest
                // for (uint256 k = dishonestAttesters.length / 2; k < dishonestAttesters.length; k++) {
                //     vm.prank(dishonestAttesters[k]);
                //     mcr.submitSuperBlockCommitment(dishonestCommitment);
                // }

                vm.prank(alice);
                mcr.postconfirmSuperBlocksAndRollover();

                MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(superBlockHeightNow);
                assert(retrievedCommitment.commitment == honestCommitment.commitment);
                assert(retrievedCommitment.blockId == honestCommitment.blockId);
                assert(retrievedCommitment.height == superBlockHeightNow);

            }

            uint256 honestStakedAttesterLength = honestAttesters.length;
            uint256 dishonestStakedAttesterLength = dishonestAttesters.length;

            // TODO replace the below with this function call
            // address newAttester = newStakedAttester(4 + i, attesterStake); // TODO why 4 not 3?

            // add a new attester
            address payable newAttester = payable(vm.addr(4 + i));
            
            staking.whitelistAddress(newAttester);
            moveToken.mint(newAttester, 3*attesterStake);
            vm.prank(newAttester);
            moveToken.approve(address(staking), attesterStake);
            vm.prank(newAttester);
            staking.stake(address(mcr), moveToken, attesterStake);

            L1BlockTime += epochDuration;
            vm.warp(L1BlockTime);

            // Force rollover by having alice (who has majority stake) call postconfirmSuperBlocksAndRollover
            vm.prank(alice);  // alice has attesterStake+1 from setup
            mcr.postconfirmSuperBlocksAndRollover();
            // confirm that the new attester has stake
            assert(mcr.getStakeForAcceptingEpoch(address(moveToken), newAttester) == attesterStake);

            // push every third signer to dishonest attesters. If pushed earlier we fail a super majority test.
            if (i % 3 == 2) {
                dishonestAttesters.push(newAttester);
                assert(dishonestAttesters.length == dishonestStakedAttesterLength + 1);
            } else {
                honestAttesters.push(newAttester);
                assert(honestAttesters.length == honestStakedAttesterLength + 1);
            }

            // TODO explain here why we do the following
            if (i % 5 == 4) {
                // removeAttester(dishonestAttesters[0], dishonestAttesters, attesterStake);
            }
            // TODO only having this but not the above is a more complex interesting scenario that would fail the line as we rollover in the postconfirmation:  
            // assert(retrievedCommitment.commitment == honestCommitment.commitment); (above)
            // this is interesting but it requires moving this upwards in the code and maybe not applying both
            if (i % 8 == 7) {
                // remove an honest attester
                // removeAttester(honestAttesters[0], honestAttesters, attesterStake);
            }

            assert(logStakeInfo(honestAttesters, dishonestAttesters));

            // L1BlockTime += 5;
            // vm.warp(L1BlockTime);
            // assert the time here
            assertEq(L1BlockTime, L1BlockTimeStart + (i+1) * (commitmentHeights + 1) * epochDuration);
        }
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), changingAttesterSetEvents * commitmentHeights);
    }

    function testForcedAttestation() public {
        vm.pauseGasMetering();

        uint256 blockTime = 300;
        vm.warp(blockTime);

        // default signer should be able to force commitment
        MCRStorage.SuperBlockCommitment memory forcedCommitment = makeDishonestCommitment(1);
        mcr.forceLatestCommitment(forcedCommitment);

        // get the latest commitment
        MCRStorage.SuperBlockCommitment memory retrievedCommitment = mcr.getPostconfirmedCommitment(1);
        assertEq(retrievedCommitment.blockId, forcedCommitment.blockId);
        assertEq(retrievedCommitment.commitment, forcedCommitment.commitment);
        assertEq(retrievedCommitment.height, forcedCommitment.height);

        // create an unauthorized signer
        address payable alice = payable(vm.addr(1));

        // try to force a different commitment with unauthorized user
        MCRStorage.SuperBlockCommitment memory badForcedCommitment = makeHonestCommitment(1);
        
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
        // vm.prank(address(mcr)); // TODO is this needed?
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
        mcr.postconfirmSuperBlocksAndRollover();
        
        // Verify postconfirmation worked
        MCRStorage.SuperBlockCommitment memory postconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        assert(postconfirmed.commitment == commitment.commitment);

        // confirm current superblock height
        uint256 currentHeightNew = mcr.getLastPostconfirmedSuperBlockHeight();
        assertEq(currentHeightNew, currentHeight + 1);

    }


    /// @notice Test that a confirmation and postconfirmation by single attester works if they have majority stake
    function testPostconfirmationWithMajorityStake() public {
        // Setup with alice having majority
        (address alice, address bob, ) = setupGenesisWithThreeAttesters(34, 33, 33);
        
        // Create commitment for height 1
        uint256 targetHeight = 1;
        
        MCRStorage.SuperBlockCommitment memory commitment = makeHonestCommitment(targetHeight);

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
        mcr.postconfirmSuperBlocksAndRollover();

        // Verify postconfirmation
        MCRStorage.SuperBlockCommitment memory postconfirmed = mcr.getPostconfirmedCommitment(targetHeight);
        assert(postconfirmed.commitment == commitment.commitment);
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), targetHeight);
    }

    /// @notice Test that a confirmation and postconfirmation by single attester fails if they have majority stake
    function testPostconfirmationWithoutMajorityStake() public {
        // Setup with no one having majority
        (address alice, address bob, ) = setupGenesisWithThreeAttesters(33, 33, 34);
        
        // Create commitment for height 1
        uint256 targetHeight = 1;
        
        MCRStorage.SuperBlockCommitment memory commitment = makeHonestCommitment(targetHeight);

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

        // Attempt postconfirmation - this should fail because there's no supermajority
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();

        // Verify height hasn't changed (postconfirmation didn't succeed)
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 0);
    }

    /// @notice Test that stake activation and postconfirmation works away from the Genesis. 
    /// TODO at genesis this behaves different and we should test this, specifically. unstake and stake are directly applied to epoch 0 until it is rolled over
    function testStakeActivationAndPostconfirmation() public {
        // Setup initial attesters with equal stakes, but Carol hasn't staked yet
        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(1, 1, 0);

        // Create commitment for height 1 by the only stable attester
        MCRStorage.SuperBlockCommitment memory commitment = makeHonestCommitment(1);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(commitment);

        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 0, "Last postconfirmed superblock height should be 0, as no supermajority was reached (2/3 < threshold)");
        assertEq(mcr.getAcceptingEpoch(),0, "Accepting epoch should be 0");

        vm.warp(epochDuration);
        assertEq(mcr.getPresentEpoch(),1, "Present epoch should be 1");
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getAcceptingEpoch(),1, "Accepting epoch should be 1");

        // Carol stakes 1, Alice unstakes
        // vm.prank(carol);
        // moveToken.approve(address(staking), 1); // TODO :  is this needed?
        vm.prank(carol);
        staking.stake(address(mcr), moveToken, 1);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), 0, "Carol's stake is still 0.");
        // Alice unstakes so her commitment is not counted in the next accepting epoch
        vm.prank(alice);
        staking.unstake(address(mcr), address(moveToken), 1);
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), 1, "Alice's stake should still be 1");
        assertEq(staking.getUnstake(address(mcr), 2, address(moveToken), alice), 1, "Alice's unstake in epoch 2 should be 1");

        // Warp to next epoch 
        vm.warp(block.timestamp + epochDuration);
        assertEq(mcr.getPresentEpoch(), 2, "Present epoch should be 2");
        assertEq(mcr.getAcceptingEpoch(), 1, "Accepting epoch should be 1");

        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getAcceptingEpoch(), 2, "Accepting epoch should be 2");

        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), carol), 1, "Carol's stake should already be active");
        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), 0, "Alice's stake should be 0");
        assertEq(moveToken.balanceOf(alice), 2, "Alice's balance should be 2");

        // Carol commits to height 1
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(commitment);

        // perform postconfirmation
        vm.prank(carol);
        mcr.postconfirmSuperBlocksAndRollover();

        // show the commitments that have been made
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 1, "Last postconfirmed superblock height should be 1, as supermajority was reached (2/2 > threshold)");
    }

    // ----------------------------------------------------------------
    // -------- Acceptor tests --------------------------------------
    // ----------------------------------------------------------------

    /// @notice Test that getAcceptorStartTime correctly calculates term start times
    function testAcceptorStartL1BlockHeight() public {
        // Test at block 0
        assertEq(mcr.getAcceptorStartL1BlockHeight(block.number), 1, "Acceptor term should start at L1Block 1");

        // Test at half an acceptor term
        vm.roll(acceptorTerm/2);
        assertEq(mcr.getAcceptorStartL1BlockHeight(block.number), 1, "Acceptor term should start at L1Block 1");

        // Test at an acceptor term boundary
        vm.roll(acceptorTerm);
        assertEq(mcr.getAcceptorStartL1BlockHeight(block.number), 1, "Acceptor term should start at L1Block 1");

        // Test at an acceptor term boundary
        vm.roll(acceptorTerm+1);
        assertEq(mcr.getAcceptorStartL1BlockHeight(block.number), acceptorTerm+1, "Acceptor term should start at L1Block acceptorTerm+1");

        // Test at 1.5 acceptor terms
        vm.warp(3 * acceptorTerm / 2);
        assertEq(mcr.getAcceptorStartL1BlockHeight(block.number), acceptorTerm+1, "Acceptor term should start at L1Block acceptorTerm+1");        
    }

    /// @notice Test setting acceptor term with validation
    function testSetAcceptorTerm() public {
        // Ensure we can retrieve the epoch duration correct
        assertEq(epochDuration, staking.getEpochDuration(address(mcr)));
        
        // Set acceptor term to 256 blocks (should succeed)
        mcr.setAcceptorTerm(256);
        assertEq(mcr.acceptorTerm(), 256, "Term should be updated to 256");

        // Try setting acceptor term to over 256 blocks (should fail)
        vm.expectRevert(MCR.AcceptorTermTooLong.selector);
        mcr.setAcceptorTerm(257);
        assertEq(mcr.acceptorTerm(), 256, "Term should remain at 256");

        // Check validity with respect to epoch duration
        uint256 validTerm = epochDuration/12 seconds/4;
        mcr.setAcceptorTerm(validTerm);
        assertEq(mcr.acceptorTerm(), validTerm, "Term should be updated to epoch related value");

        // Try setting acceptor term to epoch duration
        uint256 invalidTerm = epochDuration/12 seconds;
        vm.expectRevert(MCR.AcceptorTermTooLong.selector);
        mcr.setAcceptorTerm(invalidTerm);
        assertEq(mcr.acceptorTerm(), validTerm, "Term should remain at epoch related value");
    }

    /// @notice Test that getAcceptor correctly selects an acceptor based on block hash
    function testGetAcceptor() public {
        // Setup with three attesters with equal stakes
        (address alice, , address carol) = setupGenesisWithThreeAttesters(1, 1, 1);

        uint256 myAcceptorTerm = 4;
        mcr.setAcceptorTerm(myAcceptorTerm);
        address initialAcceptor = mcr.getAcceptor();
        assertTrue( initialAcceptor == carol, "Acceptor should be Carol");

        vm.roll(2); // we started at block 1
        assertEq(mcr.getAcceptor(), initialAcceptor, "Acceptor should not change within term");
        
        vm.roll(myAcceptorTerm); // L1blocks started at 1, not 0
        assertEq(mcr.getAcceptor(), initialAcceptor, "Acceptor should not change within term");

        // Move to next acceptor Term
        vm.roll(myAcceptorTerm+1); // L1blocks started at 1, not 0
        address newAcceptor = mcr.getAcceptor();
        assertTrue( newAcceptor == alice, "New acceptor should be Alice");
    }

    // An acceptor that is in place for acceptorTerm time should be replaced by a new acceptor after their term ended.
    // TODO reward logic is not yet implemented
    function testAcceptorRewards() public {
        (address alice, address bob, ) = setupGenesisWithThreeAttesters(1, 1, 0);
        assertEq(mcr.getAcceptor(), bob, "Bob should be the acceptor");

        // make superBlock commitments
        MCRStorage.SuperBlockCommitment memory initCommitment = makeHonestCommitment(1);
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(initCommitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(initCommitment);

        // bob postconfirms and gets a reward
        vm.prank(bob);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 1);

        // make second superblock commitment
        MCRStorage.SuperBlockCommitment memory secondCommitment = makeHonestCommitment(2);
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(secondCommitment);
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(secondCommitment);

        // alice can postconfirm, but does not get the reward
        // TODO check that bob did not get the reward
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 2);

        // bob tries to postconfirm, but already done by alice
        // TODO: bob should still get the reward
        vm.prank(bob);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 2);
    }


    // ----------------------------------------------------------------
    // -------- Reward tests --------------------------------------
    // ----------------------------------------------------------------

    function testRewardPoints() public {
        // Setup with Alice having supermajority-enabling stake
        (address alice, address bob, address carol) = setupGenesisWithThreeAttesters(2, 1, 1);
        console.log("Alice is:", alice);
        console.log("Bob is:", bob);
        console.log("Carol is:", carol);

        // Mint tokens to MCR contract for rewards
        moveToken.mint(address(mcr), 100); // MCR needs tokens to pay rewards
        console.log("Minted tokens to staking contract");
        assertEq(moveToken.balanceOf(address(mcr)), 100, "MCR contract should have 100 tokens");

        // MCR needs to approve staking contract to spend its tokens
        // TODO check this is necessary (comment and uncomment)
        vm.prank(address(mcr));
        moveToken.approve(address(staking), type(uint256).max);

        // Record initial balances
        uint256 aliceInitialBalance = moveToken.balanceOf(alice);
        uint256 bobInitialBalance = moveToken.balanceOf(bob);
        uint256 carolInitialBalance = moveToken.balanceOf(carol);

        // Log all contract addresses
        console.log("\n=== Contract Addresses ===");
        console.log("MCR contract:", address(mcr));
        console.log("Staking contract:", address(staking));
        console.log("MOVE token:", address(moveToken));
        console.log("Test contract (admin):", address(this));
        console.log("Proxy admin:", address(admin));

        // Log all roles and permissions
        console.log("\n=== Roles and Permissions ===");
        console.log("Token minter (this contract):", address(this));
        console.log("COMMITMENT_ADMIN role holder:", address(this));
        console.log("DEFAULT_ADMIN_ROLE holder:", address(this));

        // Log initial balances and stakes
        console.log("\n=== Initial State ===");
        console.log("Initial balances - A/B/C:", aliceInitialBalance, bobInitialBalance, carolInitialBalance);
        console.log("Initial stakes - A/B/C:", 
            mcr.getStakeForAcceptingEpoch(address(moveToken), alice),
            mcr.getStakeForAcceptingEpoch(address(moveToken), bob),
            mcr.getStakeForAcceptingEpoch(address(moveToken), carol)
        );
        console.log("Total stake:", mcr.getTotalStakeForAcceptingEpoch());
        console.log("\n=== Starting Test ===");

        // Exit genesis epoch
        vm.warp(block.timestamp + epochDuration);
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getAcceptingEpoch(), 1, "Should have exited genesis");

        // Submit commitments for height 1 honestly (Alice and Bob > 2/3)
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(1));

        // Check initial reward points
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), alice), 0, "Alice should have no points yet");
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), bob), 0, "Bob should have no points yet");
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), carol), 0, "Carol should have no points yet");
        console.log("Initial reward points - A/B/C:", 
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), alice),
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), bob),
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), carol)
        );

        // Trigger postconfirmation
        console.log("Triggering postconfirmation for height 1");
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();

        // New reward points
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), alice), 1, "Alice should have 1 points");
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), bob), 1, "Bob should have 1 point");
        assertEq(mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), carol), 0, "Carol should have 0 point");
        console.log("New reward points - A/B/C:", 
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), alice),
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), bob),
            mcr.getAttesterRewardPoints(mcr.getAcceptingEpoch(), carol)
        );

        // Alice and Carol commit to height 2 honestly (Alice + Carol > 2/3)
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(2));
        vm.prank(bob);
        mcr.submitSuperBlockCommitment(makeDishonestCommitment(2));
        vm.prank(carol);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(2));

        // Trigger postconfirmation, reward distribution by rolling over to next epoch
        console.log("Triggering postconfirmation for height 2");
        vm.warp(block.timestamp + epochDuration);
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getAcceptingEpoch(), 2, "Should be in epoch 2");
        console.log("Postconfirmation complete");

        // Verify rewards were distributed and points were cleared
        assertEq(mcr.attesterRewardPoints(mcr.getAcceptingEpoch(), alice), 0, "Alice's points should be cleared");
        assertEq(mcr.attesterRewardPoints(mcr.getAcceptingEpoch(), bob), 0, "Bob's points should be cleared");
        assertEq(mcr.attesterRewardPoints(mcr.getAcceptingEpoch(), carol), 0, "Carol's points should be cleared");
        console.log("Reward distribution complete");
        console.log("A/B/C balances:", moveToken.balanceOf(alice), moveToken.balanceOf(bob), moveToken.balanceOf(carol));
        console.log("A/B/C stakes:", 
            mcr.getStakeForAcceptingEpoch(address(moveToken), alice),
            mcr.getStakeForAcceptingEpoch(address(moveToken), bob),
            mcr.getStakeForAcceptingEpoch(address(moveToken), carol)
        );

        console.log("Alice initial balance:", aliceInitialBalance);
        console.log("Alice stake:", mcr.getStakeForAcceptingEpoch(address(moveToken), alice));
        console.log("Alice reward:", mcr.getStakeForAcceptingEpoch(address(moveToken), alice) * 2);
        console.log("Alice final balance:", moveToken.balanceOf(alice));
        assertEq(moveToken.balanceOf(alice), aliceInitialBalance + mcr.getStakeForAcceptingEpoch(address(moveToken), alice) * 2, "Alice reward not correct.");
        assertEq(moveToken.balanceOf(bob), bobInitialBalance + mcr.getStakeForAcceptingEpoch(address(moveToken), bob), "Bob reward not correct.");
        assertEq(moveToken.balanceOf(carol), carolInitialBalance + mcr.getStakeForAcceptingEpoch(address(moveToken), carol), "Carol reward not correct.");
    }
 

    function testPostconfirmationRewards() public {
        console.log("This is domain:", address(mcr));
        console.log("This is moveToken:", address(moveToken));
        // Setup with Alice having supermajority-enabling stake
        address alice = setupGenesisWithOneAttester(1);
        console.log("Alice is:", alice);
        assertEq(moveToken.balanceOf(alice), 0, "Alice should have 0 tokens");
        // Mint tokens to MCR contract for rewards
        moveToken.mint(address(mcr), 100); // MCR needs tokens to pay rewards
        console.log("Minted tokens to staking contract");
        assertEq(moveToken.balanceOf(address(mcr)), 100, "MCR contract should have 100 tokens");
        // MCR needs to approve staking contract to spend its tokens
        vm.prank(address(mcr));
        moveToken.approve(address(staking), type(uint256).max);
        // vm.prank(address(mcr));
        // moveToken.approve(address(moveToken), type(uint256).max);
 
        // Attester attests to height 1
        vm.prank(alice);
        mcr.submitSuperBlockCommitment(makeHonestCommitment(1));

        // get out of genesis epoch
        vm.warp(block.timestamp + epochDuration);

        // Attester postconfirms and gets a reward
        vm.prank(alice);
        mcr.postconfirmSuperBlocksAndRollover();
        assertEq(mcr.getLastPostconfirmedSuperBlockHeight(), 1);
        assertEq(mcr.getAcceptingEpoch(), 1, "Should be in epoch 1");
        console.log("at epoch %s", mcr.getAcceptingEpoch());

        assertEq(mcr.getStakeForAcceptingEpoch(address(moveToken), alice), 1, "Alice should have 1 token on stake");
        assertEq(moveToken.balanceOf(alice), 1, "Alice should have 1 token on balance");
    }
}
