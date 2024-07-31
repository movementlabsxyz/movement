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

        vm.warp(block.timestamp + 1);
        // cannot release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // tick forward
        vm.warp(block.timestamp + 101);

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
        vm.warp(block.timestamp + 1);
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(alice) == 0);

        // alice earns on locked tokens
        token.mint(alice, 50);
        assert(token.balanceOf(alice) == 150);

        // tick forward
        vm.warp(block.timestamp + 101);

        // release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(address(token)) == 0);
        assert(underlyingToken.balanceOf(alice) == 150);
    }

    function testLockMultiple() public {
        
        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        LockedToken token = new LockedToken();
        token.initialize("Locked Token", "LOCKED", underlyingToken);

        underlyingToken.grantMinterRole(address(token));

        // signers
        address payable alice = payable(vm.addr(1));

        // mint locked tokens
        address[] memory addresses = new address[](3);
        addresses[0] = alice;
        addresses[1] = alice;
        addresses[2] = alice;
        uint256[] memory mintAmounts = new uint256[](3);
        mintAmounts[0] = 100;
        mintAmounts[1] = 50;
        mintAmounts[2] = 25;
        uint256[] memory lockAmounts = new uint256[](3);
        lockAmounts[0] = 100;
        lockAmounts[1] = 50;
        lockAmounts[2] = 25;
        uint256[] memory locks = new uint256[](3);
        locks[0] = block.timestamp + 100;
        locks[1] = block.timestamp + 200;
        locks[2] = block.timestamp + 300;
        token.mintAndLock(
            addresses,
            mintAmounts,
            lockAmounts,
            locks
        );
        assert(token.balanceOf(alice) == 175);
        assert(underlyingToken.balanceOf(address(token)) == 175);
        assert(underlyingToken.balanceOf(alice) == 0);

        // cannot release locked tokens
        vm.warp(block.timestamp + 1);
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 175);
        assert(underlyingToken.balanceOf(address(token)) == 175);
        assert(underlyingToken.balanceOf(alice) == 0);

        // tick forward
        vm.warp(block.timestamp + 301);

        // release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(address(token)) == 0);
        assert(underlyingToken.balanceOf(alice) == 175);
    }

    function testLockMultiplePrematureClaim() public {
        
        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        LockedToken token = new LockedToken();
        token.initialize("Locked Token", "LOCKED", underlyingToken);

        underlyingToken.grantMinterRole(address(token));

        // signers
        address payable alice = payable(vm.addr(1));

        // mint locked tokens
        address[] memory addresses = new address[](3);
        addresses[0] = alice;
        addresses[1] = alice;
        addresses[2] = alice;
        uint256[] memory mintAmounts = new uint256[](3);
        mintAmounts[0] = 100;
        mintAmounts[1] = 50;
        mintAmounts[2] = 25;
        uint256[] memory lockAmounts = new uint256[](3);
        lockAmounts[0] = 100;
        lockAmounts[1] = 50;
        lockAmounts[2] = 25;
        uint256[] memory locks = new uint256[](3);
        locks[0] = block.timestamp + 100;
        locks[1] = block.timestamp + 200;
        locks[2] = block.timestamp + 400;
        token.mintAndLock(
            addresses,
            mintAmounts,
            lockAmounts,
            locks
        );
        assert(token.balanceOf(alice) == 175);
        assert(underlyingToken.balanceOf(address(token)) == 175);
        assert(underlyingToken.balanceOf(alice) == 0);

        // cannot release locked tokens
        vm.warp(block.timestamp + 1);
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 175);
        assert(underlyingToken.balanceOf(address(token)) == 175);
        assert(underlyingToken.balanceOf(alice) == 0);

        // tick forward
        vm.warp(block.timestamp + 301);

        // release locked tokens
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 25);
        assert(underlyingToken.balanceOf(address(token)) == 25);
        assert(underlyingToken.balanceOf(alice) == 150);

        // tick forward
        vm.warp(block.timestamp + 101);
        vm.prank(alice);
        token.release();
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(address(token)) == 0);
        assert(underlyingToken.balanceOf(alice) == 175);

    }


}