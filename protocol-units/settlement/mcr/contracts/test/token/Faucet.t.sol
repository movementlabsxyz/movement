// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import {MOVEFaucet, IERC20} from '../../src/token/faucet/MOVEFaucet.sol';
import {MOVETokenDev} from '../../src/token/MOVETokenDev.sol';
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

contract MOVEFaucetTest is Test {
    MOVEFaucet public faucet;
    MOVETokenDev public token;

    fallback() external payable {}

    function setUp() public {
        MOVETokenDev tokenImpl = new MOVETokenDev();
        TransparentUpgradeableProxy tokenProxy = new TransparentUpgradeableProxy(address(tokenImpl), address(this), abi.encodeWithSignature("initialize(address)", address(this)));
        token = MOVETokenDev(address(tokenProxy));
        faucet = new MOVEFaucet(IERC20(address(token)));
    }

    function testFaucet() public {
        vm.warp(1 days);

        token.balanceOf(address(this));

        token.transfer(address(faucet), 20 * 10 ** token.decimals());

        vm.deal(address(0x1337), 2* 10**17);

        vm.startPrank(address(0x1337));
        vm.expectRevert("MOVEFaucet: eth invalid amount");
        faucet.faucet{value: 10**16}();

        faucet.faucet{value: 10**17}();
        assertEq(token.balanceOf(address(0x1337)), 10 * 10 ** token.decimals());

        vm.expectRevert("MOVEFaucet: balance must be less than 1 MOVE");
        faucet.faucet{value: 10**17}();

        token.transfer(address(0xdead), token.balanceOf(address(0x1337)));

        vm.expectRevert("MOVEFaucet: rate limit exceeded");
        faucet.faucet{value: 10**17}();

        vm.warp(block.timestamp + 1 days);
        faucet.faucet{value: 10**17}();
        vm.stopPrank();
        vm.prank(address(this));
        uint256 balance = address(this).balance;
        faucet.withdraw();
        assertEq(address(faucet).balance, 0);
        assertEq(address(this).balance, balance + 2*10**17);
    }

    
}