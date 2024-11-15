// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../src/token/stlMoveToken.sol";
import "../../src/token/MOVETokenDev.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

contract stlMoveTokenTest is Test {
    address public multisig = address(this);
    MOVETokenDev public underlyingToken;
    stlMoveToken public token;

    function setUp() public {
        MOVETokenDev underlyingTokenImpl = new MOVETokenDev();
        TransparentUpgradeableProxy underlyingTokenProxy = new TransparentUpgradeableProxy(
            address(underlyingTokenImpl),
            address(this),
            abi.encodeWithSignature("initialize(address)", multisig)
        );


        stlMoveToken tokenImpl = new stlMoveToken();
        TransparentUpgradeableProxy tokenProxy = new TransparentUpgradeableProxy(
            address(tokenImpl),
            address(this),
            abi.encodeWithSignature("initialize(address)", address(underlyingTokenProxy))
        );
        underlyingToken = MOVETokenDev(address(underlyingTokenProxy));
        token = stlMoveToken(address(tokenProxy));

        // Check the token details
        assertEq(token.name(), "Stakable Locked Move Token");
        assertEq(token.symbol(), "stlMOVE");
    }

    function testCannotInitializeTwice() public {
       

        // Expect reversion
        vm.expectRevert(0xf92ee8a9);
        token.initialize(underlyingToken);
    }

    function testSimulateStaking() public {
       
        vm.prank(multisig);
        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable alice = payable(vm.addr(1));
        address payable bob = payable(vm.addr(2));
        address payable carol = payable(vm.addr(3));
        address payable dave = payable(vm.addr(4));

        // mint locked tokens
        address[] memory addresses = new address[](6);
        addresses[0] = alice;
        addresses[1] = bob;
        addresses[2] = carol;
        addresses[3] = dave;
        addresses[4] = alice;
        addresses[5] = bob;
        uint256[] memory mintAmounts = new uint256[](6);
        mintAmounts[0] = 100;
        mintAmounts[1] = 100;
        mintAmounts[2] = 100;
        mintAmounts[3] = 100;
        mintAmounts[4] = 0;
        mintAmounts[5] = 0;
        uint256[] memory lockAmounts = new uint256[](6);
        lockAmounts[0] = 100;
        lockAmounts[1] = 100;
        lockAmounts[2] = 100;
        lockAmounts[3] = 100;
        lockAmounts[4] = UINT256_MAX;
        lockAmounts[5] = UINT256_MAX;
        uint256[] memory locks = new uint256[](6);
        locks[0] = block.timestamp + 100;
        locks[1] = block.timestamp + 100;
        locks[2] = block.timestamp + 100;
        locks[3] = block.timestamp + 100;
        locks[4] = block.timestamp + 200;
        locks[5] = block.timestamp + 200;
        token.mintAndLock(addresses, mintAmounts, lockAmounts, locks);
        assertEq(token.balanceOf(alice), 100);
        assertEq(token.balanceOf(bob), 100);
        assertEq(token.balanceOf(carol), 100);
        assertEq(token.balanceOf(dave), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 400);
        assertEq(underlyingToken.balanceOf(alice), 0);
        assertEq(underlyingToken.balanceOf(bob), 0);
        assertEq(underlyingToken.balanceOf(carol), 0);
        assertEq(underlyingToken.balanceOf(dave), 0);

        vm.warp(block.timestamp + 1);
        // cannot release locked tokens
        vm.prank(alice);
        token.release();
        assertEq(token.balanceOf(alice), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 400);
        assertEq(underlyingToken.balanceOf(alice), 0);
        vm.prank(bob);
        token.release();
        assertEq(token.balanceOf(bob), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 400);
        assertEq(underlyingToken.balanceOf(bob), 0);
        vm.prank(carol);
        token.release();
        assertEq(token.balanceOf(carol), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 400);
        assertEq(underlyingToken.balanceOf(carol), 0);
        vm.prank(dave);
        token.release();
        assertEq(token.balanceOf(dave), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 400);
        assertEq(underlyingToken.balanceOf(dave), 0);

        // add a transfer sink to represent a staking pool
        address payable stakingPool = payable(vm.addr(5));
        token.grantTransferSinkRole(stakingPool);
        token.grantBuyerRole(stakingPool);

        // mint some funds on the underlying token for the staking pool to reward stakers
        underlyingToken.mint(stakingPool, 100);

        // use to custodian to stake the locked tokens
        vm.prank(alice);
        token.transfer(stakingPool, 100);
        assertEq(token.balanceOf(alice), 0);
        assertEq(underlyingToken.balanceOf(stakingPool), 200);
        assertEq(underlyingToken.balanceOf(address(token)), 300);
        vm.prank(bob);
        token.transfer(stakingPool, 100);
        assertEq(token.balanceOf(bob), 0);
        assertEq(underlyingToken.balanceOf(stakingPool), 300);
        assertEq(underlyingToken.balanceOf(address(token)), 200);
        vm.prank(carol);
        token.transfer(stakingPool, 100);
        assertEq(token.balanceOf(carol), 0);
        assertEq(underlyingToken.balanceOf(stakingPool), 400);
        assertEq(underlyingToken.balanceOf(address(token)), 100);
        // ! dave does not stake

        // alice gets reward and cashes out through the custodian, but cannot withdraw
        vm.prank(stakingPool);
        underlyingToken.approve(address(token), 110);
        vm.prank(stakingPool);
        token.buyCustodialToken(alice, 110);
        assertEq(token.balanceOf(alice), 110);
        assertEq(underlyingToken.balanceOf(stakingPool), 290);
        assertEq(underlyingToken.balanceOf(address(token)), 210);
        vm.prank(alice);
        token.release();
        assertEq(token.balanceOf(alice), 110);
        assertEq(underlyingToken.balanceOf(alice), 0);
        assertEq(underlyingToken.balanceOf(address(token)), 210);

        // bob does not get a reward but cashes out through the custodian
        vm.prank(stakingPool);
        underlyingToken.approve(address(token), 100);
        vm.prank(stakingPool);
        token.buyCustodialToken(bob, 100);
        assertEq(token.balanceOf(bob), 100);
        assertEq(underlyingToken.balanceOf(stakingPool), 190);
        assertEq(underlyingToken.balanceOf(address(token)), 310);
        vm.prank(bob);
        token.release();
        assertEq(token.balanceOf(bob), 100);
        assertEq(underlyingToken.balanceOf(bob), 0);
        assertEq(underlyingToken.balanceOf(address(token)), 310);

        // time passes
        vm.warp(block.timestamp + 101);

        // alice withdraws as much as she can
        vm.prank(alice);
        token.release();
        assertEq(token.balanceOf(alice), 10);
        assertEq(underlyingToken.balanceOf(alice), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 210);

        // bob withdraws as much as he can
        vm.prank(bob);
        token.release();
        assertEq(token.balanceOf(bob), 0);
        assertEq(underlyingToken.balanceOf(bob), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 110);

        // carol withdraws as much as she can, but it she doesn't have any because here funds are still staked
        vm.prank(carol);
        token.release();
        assertEq(token.balanceOf(carol), 0);
        assertEq(underlyingToken.balanceOf(carol), 0);
        assertEq(underlyingToken.balanceOf(address(token)), 110);

        // carol gets reward and cashes out through the custodian
        vm.prank(stakingPool);
        underlyingToken.approve(address(token), 110);
        vm.prank(stakingPool);
        token.buyCustodialToken(carol, 110);
        assertEq(token.balanceOf(carol), 110);
        assertEq(underlyingToken.balanceOf(stakingPool), 80); // spent 20 in total on rewards
        assertEq(underlyingToken.balanceOf(address(token)), 220);

        // carol withdraws as much as she can
        vm.prank(carol);
        token.release();
        assertEq(token.balanceOf(carol), 10);
        assertEq(underlyingToken.balanceOf(carol), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 120);

        // dave withdraws as much as he can
        vm.prank(dave);
        token.release();
        assertEq(token.balanceOf(dave), 0);
        assertEq(underlyingToken.balanceOf(dave), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 20);

        // time passes
        vm.warp(block.timestamp + 101);

        // alice withdraws as much as she can; she can withdraw her rewards
        vm.prank(alice);
        token.release();
        assertEq(token.balanceOf(alice), 0);
        assertEq(underlyingToken.balanceOf(alice), 110);
        assertEq(underlyingToken.balanceOf(address(token)), 10);

        // bob withdraws as much as he can; he can withdraw his rewards, but doesn't have any
        vm.prank(bob);
        token.release();
        assertEq(token.balanceOf(bob), 0);
        assertEq(underlyingToken.balanceOf(bob), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 10);

        // carol withdraws as much as she can; she can't withdraw her rewards
        vm.prank(carol);
        token.release();
        assertEq(token.balanceOf(carol), 10);
        assertEq(underlyingToken.balanceOf(carol), 100);
        assertEq(underlyingToken.balanceOf(address(token)), 10);
    }
}
