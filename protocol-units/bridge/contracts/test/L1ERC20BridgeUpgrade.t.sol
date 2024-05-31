// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import {L1ERC20Bridge} from "../src/L1ERC20Bridge.sol";
import {L1ERC20BridgeUpgrade} from "./mock/L1ERC20BridgeUpgrade.sol";

import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract L1ERC20BridgeTest is Test {
    ERC1967Proxy public proxy;

    L1ERC20Bridge public bridge;
    L1ERC20BridgeUpgrade public bridgeUpgrade;

    address public owner;
    address public nonOwner;

    function setUp() public {
        owner = address(this);
        nonOwner = address(0x1234);

        // Deploy the contract and initialize with the owner
        bridge = new L1ERC20Bridge();
        bridge.initialize(owner);

        // Deploy the proxy with the implementation address and initialization data
        proxy = new ERC1967Proxy(
            address(bridge),
            abi.encodeWithSelector(L1ERC20Bridge.initialize.selector, owner)
        );

        // Cast the proxy to the L1ERC20Bridge type
        bridge = L1ERC20Bridge(address(proxy));
    }

    function test_UpgradeToMockL1ERC20Bridge() public {
        // Deploy the new implementation contract
        L1ERC20BridgeUpgrade bridge_upgrade = new L1ERC20BridgeUpgrade();

        // Upgrade the proxy to the new implementation
        vm.prank(owner);
        bridge.upgradeToAndCall(address(bridge_upgrade), new bytes(0x0));

        // Cast the proxy to the MockL1ERC20Bridge type
        L1ERC20BridgeUpgrade upgradedProxy = L1ERC20BridgeUpgrade(
            address(proxy)
        );

        // Verify that the storage is maintained and new functionality works
        assertEq(upgradedProxy.upgraded(), true);
    }

    function test_UpgradeToMockL1ERC20BridgeNonOwner() public {
        // Deploy the new implementation contract
        L1ERC20BridgeUpgrade bridge_upgrade = new L1ERC20BridgeUpgrade();

        // Upgrade the proxy to the new implementation
        vm.prank(nonOwner);

        bytes4 selector = bytes4(
            keccak256("OwnableUnauthorizedAccount(address)")
        );
        vm.expectRevert(abi.encodeWithSelector(selector, nonOwner));
        bridge.upgradeToAndCall(address(bridge_upgrade), new bytes(0x0));
    }
}
