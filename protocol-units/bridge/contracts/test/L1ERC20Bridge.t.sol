// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import {L1ERC20Bridge} from "../src/L1ERC20Bridge.sol";

contract L1ERC20BridgeTest is Test {
    L1ERC20Bridge public bridge;
    address public owner;
    address public nonOwner;

    function setUp() public {
        owner = address(this);
        nonOwner = address(0x1234);

        // Deploy the contract and initialize with the owner
        bridge = new L1ERC20Bridge();
        bridge.initialize(owner);
    }

    function test_Initialization() public view {
        assertEq(bridge.owner(), owner);
    }

    function test_SetValue() public {
        uint256 newValue = 42;
        bridge.setValue(newValue);
        assertEq(bridge.value(), newValue);
    }

    function testFuzz_SetValue(uint256 newValue) public {
        bridge.setValue(newValue);
        assertEq(bridge.value(), newValue);
    }
}
