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
    string public moveSignature = "initialize(address,address)";
    address public multisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);
    address public anchorage = address(0xabc);

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
            address(moveTokenImplementation), address(timelock), abi.encodeWithSignature(moveSignature, multisig, anchorage)
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();

        admin = ProxyAdmin(entries[entries.length -2].emitter);

        token = MOVEToken(address(tokenProxy));
    }

    function testCannotInitializeTwice() public {
        // Initialize the contract
        vm.expectRevert(0xf92ee8a9);
        token.initialize(multisig, anchorage);
    }

    function testDecimals() public {
        assertEq(token.decimals(), 8);
    }

    function testTotalSupply() public {
        assertEq(token.totalSupply(), 10000000000 * 10 ** 8);
    }

    function testMultisigBalance() public {
        assertEq(token.balanceOf(anchorage), 10000000000 * 10 ** 8);
    }

    function testAdminRoleFuzz(address other) public {
        assertEq(token.hasRole(0x00, other), false);
        assertEq(token.hasRole(0x00, multisig), true);
        assertEq(token.hasRole(0x00, anchorage), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(this),
                0x00
            )
        );
        token.grantRole(0x00, other);
    }

    function testUpgradeFromTimelock() public {
        assertEq(admin.owner(), address(timelock));

        vm.prank(string2Address("Andy"));
        timelock.schedule(
            address(admin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(tokenProxy),
                address(moveTokenImplementation2),
                abi.encodeWithSignature("initialize(address)",multisig)
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
                "upgradeAndCall(address,address,bytes)",
                address(tokenProxy),
                address(moveTokenImplementation2),
                abi.encodeWithSignature("initialize(address)", multisig)
            ),
            bytes32(0),
            bytes32(0)
        );

        // Check the token details
        assertEq(token.decimals(), 8);
        assertEq(token.totalSupply(), 10000000000 * 10 ** 8);
        assertEq(token.balanceOf(anchorage), 10000000000 * 10 ** 8);
    }

    function testTransferToNewTimelock() public {
        assertEq(admin.owner(), address(timelock));

        uint256 minDelay = 1 days;
        address[] memory proposers = new address[](5);
        address[] memory executors = new address[](1);

        // Andy has been compromised, Albert will be the new proposer
        // we need to transfer the proxyAdmin ownership to a new timelock
        proposers[0] = string2Address("Albert");
        proposers[1] = string2Address("Bob");
        proposers[2] = string2Address("Charlie");
        proposers[3] = string2Address("David");
        proposers[4] = string2Address("Eve");

        executors[0] = multisig;

        TimelockController newTimelock = new TimelockController(minDelay, proposers, executors, address(0x0));
        vm.prank(string2Address("Bob"));
        timelock.schedule(
            address(admin),
            0,
            abi.encodeWithSignature(
                "transferOwnership(address)",
                address(newTimelock)
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
                "transferOwnership(address)",
                address(newTimelock)
            ),
            bytes32(0),
            bytes32(0)
        );

        assertEq(admin.owner(), address(newTimelock));
        
    }

    function testGrants() public {
        testUpgradeFromTimelock();

        // Check the token details
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), multisig), true);
    }

    function testMint() public {
        testUpgradeFromTimelock();
        uint256 intialBalance = MOVETokenV2(address(token)).balanceOf(address(0x1337));
        // Mint tokens
        vm.prank(multisig);
        MOVETokenV2(address(token)).mint(address(0x1337), 100);

        // Check the token details
        assertEq(MOVETokenV2(address(token)).balanceOf(address(0x1337)), intialBalance + 100);
    }

    function testRevokeMinterRole() public {
        testUpgradeFromTimelock();
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), multisig), true);
        
        vm.startPrank(multisig);
        MOVETokenV2(address(token)).mint(address(0x1337), 100);
        // Revoke minter role
        MOVETokenV2(address(token)).revokeMinterRole(multisig);

        // Check the token details
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenV2(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenV2(address(token)).mint(address(0x1337), 100);
        vm.stopPrank();
    }

    function testGrantRevokeMinterAdminRole() public {
        testUpgradeFromTimelock();
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), multisig), true);
        vm.startPrank(multisig);

        MOVETokenV2(address(token)).mint(address(0x1337), 100);
        // Revoke minter role
        MOVETokenV2(address(token)).revokeMinterRole(multisig);

        // Check the token details
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenV2(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenV2(address(token)).mint(address(0x1337), 100);

        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), address(0x1337)), false);
        // Grant minter role
        MOVETokenV2(address(token)).grantMinterRole(address(0x1337));
        vm.stopPrank();
        vm.prank(address(0x1337));
        MOVETokenV2(address(token)).mint(address(0x1337), 100);

        // Check the token details
        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), address(0x1337)), true);

        // Revoke minter role
        vm.prank(multisig);
        MOVETokenV2(address(token)).revokeMinterRole(address(0x1337));

        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ROLE(), address(0x1337)), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(0x1337),
                MOVETokenV2(address(token)).MINTER_ROLE()
            )
        );
        vm.prank(address(0x1337));
        MOVETokenV2(address(token)).mint(address(0x1337), 100);

        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ADMIN_ROLE(), multisig), true);
        // Revoke minter admin role
        vm.startPrank(multisig);
        MOVETokenV2(address(token)).revokeMinterAdminRole(multisig);

        assertEq(MOVETokenV2(address(token)).hasRole(MOVETokenV2(address(token)).MINTER_ADMIN_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenV2(address(token)).MINTER_ADMIN_ROLE()
            )
        );
        MOVETokenV2(address(token)).grantMinterRole(multisig);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenV2(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenV2(address(token)).mint(address(0x1337), 100);
        vm.stopPrank();
    }
}
