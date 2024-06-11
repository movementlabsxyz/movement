// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../../src/token/base/MintableToken.sol";
import "../../../src/token/custodian/CustodianToken.sol";
// import base access control instead of upgradeable access control

contract CustodianTokenTest is Test {

    function testInitialize() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        // Check the token details
        assertEq(token.name(), "Custodian Token");
        assertEq(token.symbol(), "CUSTODIAN");

    }

    function testCannotInitializeTwice() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);
   
        // Attempt to initialize again should fail
        vm.expectRevert(0xf92ee8a9);
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

    }

    function testGrants() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

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

    function testCustodianMint() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // valid minting succeeds
        token.mint(address(this), 100);
        assert(token.balanceOf(address(this)) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);

        // valid minting is incremental
        address payable signer = payable(vm.addr(1)); 
        token.mint(signer, 100);
        assert(token.balanceOf(signer) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 200);

        // signers with the minter role can call through the custodian
        token.grantMinterRole(signer);
        vm.prank(signer);
        token.mint(signer, 100);
        assert(token.balanceOf(signer) == 200);
        assert(underlyingToken.balanceOf(address(token)) == 300);

        // signers without the minter role cannot call through the custodian
        token.revokeMinterRole(signer);
        vm.prank(signer);
        vm.expectRevert(); // todo: catch type
        token.mint(signer, 100);

    }

    function testCustodianTransferToValidSink() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable validSink = payable(vm.addr(2));
        token.grantTransferSinkRole(validSink);
        address payable alice = payable(vm.addr(5));

        // transfer to valid sink succeeds
        token.mint(alice, 100);
        vm.prank(alice);
        token.transfer(validSink, 100);
        assert(token.balanceOf(alice) == 0);
        assert(underlyingToken.balanceOf(validSink) == 100);

    }

    function testCustodianTransferToInvalidSink() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable invalidSink = payable(vm.addr(2));
        address payable alice = payable(vm.addr(5));

        // transfer to invalid sink fails
        token.mint(alice, 100);
        vm.prank(alice);
        vm.expectRevert(); // todo: catch type
        token.transfer(invalidSink, 100);

    }

    function testCustodianBuyValidSource() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable validSource = payable(vm.addr(2));
        token.grantBuyerRole(validSource);
        address payable alice = payable(vm.addr(5));

        // fund the valid source in the underlying token
        underlyingToken.mint(validSource, 100);

        // approve the custodian to spend the underlying token
        vm.prank(validSource);
        underlyingToken.approve(address(token), 100);

        // buy from valid source succeeds
        vm.prank(validSource);
        token.buyCustodialTokenFor(alice, 100);
        assert(token.balanceOf(alice) == 100);
        assert(underlyingToken.balanceOf(address(token)) == 100);
        assert(underlyingToken.balanceOf(validSource) == 0);

    }

    function testCustodianBuyInvalidSource() public {

        MintableToken underlyingToken = new MintableToken();
        underlyingToken.initialize("Underlying Token", "UNDERLYING");

        CustodianToken token = new CustodianToken();
        token.initialize("Custodian Token", "CUSTODIAN", underlyingToken);

        underlyingToken.grantMinterRole(address(token));
        assert(underlyingToken.hasRole(underlyingToken.MINTER_ROLE(), address(token)));

        // signers
        address payable invalidSource = payable(vm.addr(2));
        address payable alice = payable(vm.addr(5));

        // fund the valid source in the underlying token
        underlyingToken.mint(invalidSource, 100);

        // approve the custodian to spend the underlying token
        vm.prank(invalidSource);
        underlyingToken.approve(address(token), 100);

        // buy from valid source succeeds
        vm.prank(invalidSource);
        vm.expectRevert(); // todo: catch type
        token.buyCustodialTokenFor(alice, 100);

    }

}