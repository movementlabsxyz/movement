// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import {MOVEFaucet, IERC20} from '../src/MOVEFaucet.sol';
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

contract MOVETokenDev is ERC20, AccessControl {

    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");
    bytes32 public constant MINTER_ADMIN_ROLE = keccak256("MINTER_ADMIN_ROLE");

    /**
     * @dev Initialize the contract
     */
    constructor(address manager) ERC20("Movement", "MOVE") {
        _mint(manager, 10000000000 * 10 ** decimals());
        _grantRole(MINTER_ADMIN_ROLE, manager);
        _grantRole(MINTER_ROLE, manager);
    }

    function grantRoles(address account) public onlyRole(DEFAULT_ADMIN_ROLE) {
        _grantRole(MINTER_ADMIN_ROLE, account);
        _grantRole(MINTER_ROLE, account);

    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}

contract MOVEFaucetTest is Test {
    MOVEFaucet public faucet;
    MOVETokenDev public token;

    fallback() external payable {}

    function setUp() public {
        token = new MOVETokenDev(address(this));
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

        vm.expectRevert("MOVEFaucet: balance must be less than determined amount of MOVE");
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