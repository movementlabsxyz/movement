// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/staking/MovementStaking.sol";
import "../../src/token/MOVEToken.sol";

contract MovementStakingTest is Test {

    function testInitialize() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

    }

    function testCannotInitializeTwice() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        staking.initialize(moveToken);
    }

    function testRegister() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new domain
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        assertEq(staking.getCurrentEpoch(domain), 0);

    }

    function testWhitelist() public {
        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);
        
        // Our whitelister
        address whitelister = vm.addr(1);
        // Whitelist them
        staking.whitelistAddress(whitelister);
        assertEq(staking.hasRole(staking.WHITELIST_ROLE(), whitelister), true);
        // Remove them from the whitelist
        staking.removeAddressFromWhitelist(whitelister);
        assertEq(staking.hasRole(staking.WHITELIST_ROLE(), whitelister), false);
        // As a whitelister let's see if I can whitelist myself
        vm.prank(whitelister);
        vm.expectRevert();
        staking.whitelistAddress(whitelister);
    }

    function testSimpleStaker() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // stake at the domain
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 100);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        assertEq(moveToken.balanceOf(staker), 0);
        assertEq(staking.getStakeAtEpoch(domain, 0, address(moveToken), staker), 100);
    }

    function testSimpleGenesisCeremony() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // genesis ceremony
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 100);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        vm.prank(domain);
        staking.acceptGenesisCeremony();
        assertNotEq(staking.currentEpochByDomain(domain), 0);
        assertEq(staking.getCurrentEpochStake(domain, address(moveToken), staker), 100);

    }

    function testSimpleRolloverEpoch() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // genesis ceremony
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 100);
        staking.whitelistAddress(staker);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        vm.prank(domain);
        staking.acceptGenesisCeremony();
        
        // rollover epoch
        for(uint256 i = 0; i < 10; i++) {
            vm.warp((i + 1) * 1 seconds);
            uint256 epochBefore = staking.getCurrentEpoch(domain);
            vm.prank(domain);
            staking.rollOverEpoch();
            uint256 epochAfter = staking.getCurrentEpoch(domain);
            assertEq(epochAfter, epochBefore + 1);
            assertEq(staking.getCurrentEpochStake(domain, address(moveToken), staker), 100);
        }

    }

    function testUnstakeRolloverEpoch() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // genesis ceremony
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 100);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        vm.prank(domain);
        staking.acceptGenesisCeremony();
        
        for(uint256 i = 0; i < 10; i++) {

            vm.warp((i + 1) * 1 seconds);
            uint256 epochBefore = staking.getCurrentEpoch(domain);

            // unstake
            vm.prank(staker);
            staking.unstake(domain, address(moveToken), 10);
            assertEq(staking.getCurrentEpochStake(domain, address(moveToken), staker), 100 - (i * 10));
            assertEq(moveToken.balanceOf(staker), i * 10);

            // roll over
            vm.prank(domain);
            staking.rollOverEpoch();
            uint256 epochAfter = staking.getCurrentEpoch(domain);
            assertEq(epochAfter, epochBefore + 1);

        }

    }

    function testUnstakeAndStakeRolloverEpoch() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // genesis ceremony
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 150);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        vm.prank(domain);
        staking.acceptGenesisCeremony();
        
        for(uint256 i = 0; i < 10; i++) {

            vm.warp((i + 1) * 1 seconds);
            uint256 epochBefore = staking.getCurrentEpoch(domain);

            // unstake
            vm.prank(staker);
            staking.unstake(domain, address(moveToken), 10);

            // stake
            vm.prank(staker);
            moveToken.approve(address(staking), 5);
            vm.prank(staker);
            staking.stake(domain, moveToken, 5);

            // check stake
            assertEq(
                staking.getCurrentEpochStake(domain, address(moveToken), staker), 
                (100 - (i * 10)) + (i * 5)
            );
            assertEq(
                moveToken.balanceOf(staker), 
                (50 - (i + 1) * 5) + (i * 10)
            );

            // roll over
            vm.prank(domain);
            staking.rollOverEpoch();
            uint256 epochAfter = staking.getCurrentEpoch(domain);
            assertEq(epochAfter, epochBefore + 1);

        }

    }

    function testUnstakeStakeAndSlashRolloverEpoch() public {

        MOVEToken moveToken = new MOVEToken();
        moveToken.initialize();

        MovementStaking staking = new MovementStaking();
        staking.initialize(moveToken);

        // Register a new staker
        address payable domain = payable(vm.addr(1));
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveToken);
        vm.prank(domain);
        staking.registerDomain(1 seconds, custodians);

        // genesis ceremony
        address payable staker = payable(vm.addr(2));
        staking.whitelistAddress(staker);
        moveToken.mint(staker, 150);
        vm.prank(staker);
        moveToken.approve(address(staking), 100);
        vm.prank(staker);
        staking.stake(domain, moveToken, 100);
        vm.prank(domain);
        staking.acceptGenesisCeremony();
        
        for(uint256 i = 0; i < 5; i++) {

            vm.warp((i + 1) * 1 seconds);
            uint256 epochBefore = staking.getCurrentEpoch(domain);

            // unstake
            vm.prank(staker);
            staking.unstake(domain, address(moveToken), 10);

            // stake
            vm.prank(staker);
            moveToken.approve(address(staking), 5);
            vm.prank(staker);
            staking.stake(domain, moveToken, 5);

            // check stake
            assertEq(
                staking.getCurrentEpochStake(domain, address(moveToken), staker), 
                (100 - (i * 10)) + (i * 5) - (i * 1)
            );
            assertEq(
                moveToken.balanceOf(staker), 
                (50 - (i + 1) * 5) + (i * 10)
            );

            // slash
            vm.prank(domain);
            address[] memory custodians1 = new address[](1);
            custodians1[0] = address(moveToken);
            address[] memory attesters1 = new address[](1);
            attesters1[0] = staker;
            uint256[] memory amounts1 = new uint256[](1);
            amounts1[0] = 1;
            uint256[] memory refundAmounts1 = new uint256[](1);
            refundAmounts1[0] = 0;
            staking.slash(custodians1, attesters1, amounts1, refundAmounts1);

            // slash immediately takes effect
            assertEq(
                staking.getCurrentEpochStake(domain, address(moveToken), staker), 
                (100 - (i * 10)) + (i * 5) - ((i + 1) * 1)
            );

            // roll over
            vm.prank(domain);
            staking.rollOverEpoch();
            uint256 epochAfter = staking.getCurrentEpoch(domain);
            assertEq(epochAfter, epochBefore + 1);

        }

    }

}