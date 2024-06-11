// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/token/base/MintableToken.sol";
import "../../../src/token/locked/LockedToken.sol";
// import base access control instead of upgradeable access control

contract LockedTokenTest is Test {

    function testInitialize() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        LockedToken token = new LockedToken();
        token.initialize("Locked Token", "LOCKED", underlyingToken);

        // Check the token details
        assertEq(token.name(), "Locked Token");
        assertEq(token.symbol(), "LOCKED");

    }

    function testBasicLock() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        LockedToken token = new LockedToken();
        token.initialize("Locked Token", "LOCKED", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable alice = payable(vm.addr(1));

        // mint locked tokens
        address[] memory addresses = new address[](1);
        addresses[0] = alice;
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = 100;
        uint256[] memory locks = new uint256[](1);
        locks[0] = block.timestamp + 100;
        token.mintAndLock(
            addresses,
            amounts,
            amounts, // in this test case, we are not adding separate lock amounts
            locks
        );
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // cannot release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // tick forward
        vm.warp(101);

        // release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(address(token)) == 0);
        assert(underlyingToken.balanceOf(alice) == 100);

    }

    function testLockWithEarnings() public {
        
        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        LockedToken token = new LockedToken();
        token.initialize("Locked Token", "LOCKED", underlyingToken);

        underlyingToken.grantMinterRole(address(token));

        // signers
        address payable alice = payable(vm.addr(1));

        // mint locked tokens
        address[] memory addresses = new address[](1);
        addresses[0] = alice;
        uint256[] memory mintAmounts = new uint256[](1);
        mintAmounts[0] = 100;
        uint256[] memory lockAmounts = new uint256[](1);
        lockAmounts[0] = 150;
        uint256[] memory locks = new uint256[](1);
        locks[0] = block.timestamp + 100;
        token.mintAndLock(
            addresses,
            mintAmounts,
            lockAmounts,
            locks
        );
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // cannot release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // alice earns on locked tokens
        token.mint(alice, 50);
        assert(token.balanceOf(alice) == 150);

        // tick forward
        vm.warp(101);

        // release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(address(token)) == 0);
        assert(underlyingToken.balanceOf(alice) == 150);
    }

}