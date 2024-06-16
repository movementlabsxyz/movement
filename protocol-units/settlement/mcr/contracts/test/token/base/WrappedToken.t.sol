// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/token/base/MintableToken.sol";
import "../../../src/token/base/WrappedToken.sol";
// import base access control instead of upgradeable access control


contract WrappedTokenTest is Test {

    function testInitialize() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        WrappedToken token = new WrappedToken();
        token.initialize("Base Token", "BASE", underlyingToken);

        // Check the token details
        assertEq(token.name(), "Base Token");
        assertEq(token.symbol(), "BASE");

    }

    function testCannotInitializeTwice() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        WrappedToken token = new WrappedToken();
        token.initialize("Base Token", "BASE", underlyingToken);
   
        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        token.initialize("Base Token", "BASE", underlyingToken);

    }

    function testGrants() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        WrappedToken token = new WrappedToken();
        token.initialize("Base Token", "BASE", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // valid minting succeeds
        vm.prank(address(token));
        underlyingToken.mint(address(this), 100);
        assert(underlyingToken.balanceOf(address(this)) == 100);

        // invalid minting fails
        address payable signer = payable(vm.addr(1)); 
        vm.prank(signer);
        vm.expectRevert(); // todo: catch type
        underlyingToken.mint(signer, 100);

    }

}