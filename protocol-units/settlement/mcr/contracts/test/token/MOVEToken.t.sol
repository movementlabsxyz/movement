// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import {MOVEToken} from "../../src/token/MOVEToken.sol";
import {MOVETokenV2} from "../../src/token/MOVETokenV2.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

function string2Address(bytes memory str) returns (address addr) {
    bytes32 data = keccak256(str);
    assembly {
        mstore(0, data)
        addr := mload(0)
    }
}

contract MOVETokenTest is Test {
    MOVEToken public token;
    TransparentUpgradeableProxy public tokenProxy;
    ProxyAdmin public admin;
    MOVEToken public moveTokenImplementation;
    MOVETokenV2 public moveTokenImplementation2;
    TimelockController public timelock;
    string public moveSignature = "initialize(address)";
    address public multisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);

    function setUp() public {
        moveTokenImplementation = new MOVEToken();
        moveTokenImplementation2 = new MOVETokenV2();

        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        proposers[0] = string2Address("Andy");
        proposers[1] = string2Address("Bob");
        proposers[2] = string2Address("Charlie");
        proposers[3] = string2Address("David");
        proposers[4] = string2Address("Eve");
        executors[0] = multisig;

        timelock = new TimelockController(minDelay, proposers, executors, address(0x0));


        vm.recordLogs();
        // Deploy proxy
        tokenProxy = new TransparentUpgradeableProxy(
            address(moveTokenImplementation), address(timelock), abi.encodeWithSignature(moveSignature, multisig)
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();
        assertEq(entries.length, 5);

        admin = ProxyAdmin(entries[3].emitter);

        token = MOVEToken(address(tokenProxy));
    }

    function testCannotInitializeTwice() public {
        // Initialize the contract
        vm.expectRevert(0xf92ee8a9);
        token.initialize(multisig);
    }

    function testDecimals() public {
        assertEq(token.decimals(), 8);
    }

    function testTotalSupply() public {
        assertEq(token.totalSupply(), 10000000000 * 10 ** 8);
    }

    function testMultisigBalance() public {
        assertEq(token.balanceOf(multisig), 10000000000 * 10 ** 8);
    }

    function testUpgradeFromTimelock() public {

        assertEq(admin.owner(), address(timelock));

        vm.prank(string2Address("Andy"));
        timelock.schedule(
            address(admin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)", address(tokenProxy), address(moveTokenImplementation2), ""
            ),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );

        vm.warp(block.timestamp + 1 days + 1);

        vm.prank(multisig);
        timelock.execute(
            address(admin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)", address(tokenProxy), address(moveTokenImplementation2), ""
            ),
            bytes32(0),
            bytes32(0)
        );

        // Check the token details
        assertEq(token.decimals(), 8);
        assertEq(token.totalSupply(), 10000000000 * 10 ** 8);
        assertEq(token.balanceOf(multisig), 10000000000 * 10 ** 8);
    }

    
}
