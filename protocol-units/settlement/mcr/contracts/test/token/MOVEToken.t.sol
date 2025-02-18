// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "forge-std/console2.sol";
import {MOVEToken} from "../../src/token/MOVEToken.sol";
import {MOVETokenDev} from "../../src/token/MOVETokenDev.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

function string2Address(bytes memory str) pure returns (address addr) {
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
    MOVETokenDev public moveTokenImplementation2;
    TimelockController public timelock;
    string public moveSignature = "initialize(address,address)";
    address public multisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);
    address public anchorage = address(0xabc);
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function setUp() public {
        moveTokenImplementation = new MOVEToken();
        moveTokenImplementation2 = new MOVETokenDev();

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
            address(moveTokenImplementation),
            address(timelock),
            abi.encodeWithSignature(moveSignature, multisig, anchorage)
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();

        admin = ProxyAdmin(entries[entries.length - 2].emitter);

        token = MOVEToken(address(tokenProxy));
    }

    function testCannotInitializeTwice() public {
        // Initialize the contract
        vm.expectRevert(0xf92ee8a9);
        token.initialize(multisig, anchorage);
    }

    function testDecimals() public view {
        assertEq(token.decimals(), 8);
    }

    function testTotalSupply() public view {
        assertEq(token.totalSupply(), 10000000000 * 10 ** 8);
    }

    function testMultisigBalance() public view {
        assertEq(token.balanceOf(anchorage), 10000000000 * 10 ** 8);
    }

    /// @notice Fuzzing test to verify admin role permissions
    /// @param other Any address to test against
    function testAdminRoleFuzz(address other) public {
        // Verify multisig has admin role (primary admin)
        assertEq(token.hasRole(DEFAULT_ADMIN_ROLE, multisig), true);
        
        // Verify other addresses only have admin if they are the multisig
        assertEq(token.hasRole(DEFAULT_ADMIN_ROLE, other), other == multisig);
        
        // Verify custody address (anchorage) does not have admin role
        assertEq(token.hasRole(DEFAULT_ADMIN_ROLE, anchorage), false);

        // Test role granting permissions (skip multisig since it should succeed)
        vm.assume(other != multisig);
        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), DEFAULT_ADMIN_ROLE)
        );
        token.grantRole(DEFAULT_ADMIN_ROLE, other);
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
                ""
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
                ""
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
            abi.encodeWithSignature("transferOwnership(address)", address(newTimelock)),
            bytes32(0),
            bytes32(0),
            block.timestamp + 1 days
        );

        vm.warp(block.timestamp + 1 days + 1);
        vm.prank(multisig);
        timelock.execute(
            address(admin),
            0,
            abi.encodeWithSignature("transferOwnership(address)", address(newTimelock)),
            bytes32(0),
            bytes32(0)
        );

        assertEq(admin.owner(), address(newTimelock));
    }

    function testGrants() public {
        testUpgradeFromTimelock();

        vm.prank(multisig);
        MOVETokenDev(address(token)).grantRoles(multisig);

        // Check the token details
        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), multisig), true);
    }

    function testMint() public {
        testUpgradeFromTimelock();

        vm.prank(multisig);
        MOVETokenDev(address(token)).grantRoles(multisig);
        uint256 intialBalance = MOVETokenDev(address(token)).balanceOf(address(0x1337));
        // Mint tokens
        vm.prank(multisig);
        MOVETokenDev(address(token)).mint(address(0x1337), 100);

        // Check the token details
        assertEq(MOVETokenDev(address(token)).balanceOf(address(0x1337)), intialBalance + 100);
    }

    function testRevokeMinterRole() public {
        testUpgradeFromTimelock();

        vm.prank(multisig);
        MOVETokenDev(address(token)).grantRoles(multisig);
        
        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), multisig), true);

        vm.startPrank(multisig);
        MOVETokenDev(address(token)).mint(address(0x1337), 100);
        // Revoke minter role
        MOVETokenDev(address(token)).revokeMinterRole(multisig);

        // Check the token details
        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenDev(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenDev(address(token)).mint(address(0x1337), 100);
        vm.stopPrank();
    }

    function testGrantRevokeMinterAdminRole() public {
        testUpgradeFromTimelock();
        vm.prank(multisig);
        MOVETokenDev(address(token)).grantRoles(multisig);
        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), multisig), true);
        vm.startPrank(multisig);

        MOVETokenDev(address(token)).mint(address(0x1337), 100);
        // Revoke minter role
        MOVETokenDev(address(token)).revokeMinterRole(multisig);

        // Check the token details
        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), multisig), false);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenDev(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenDev(address(token)).mint(address(0x1337), 100);

        assertEq(
            MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), address(0x1337)), false
        );
        // Grant minter role
        MOVETokenDev(address(token)).grantMinterRole(address(0x1337));
        vm.stopPrank();
        vm.prank(address(0x1337));
        MOVETokenDev(address(token)).mint(address(0x1337), 100);

        // Check the token details
        assertEq(
            MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), address(0x1337)), true
        );

        // Revoke minter role
        vm.prank(multisig);
        MOVETokenDev(address(token)).revokeMinterRole(address(0x1337));

        assertEq(
            MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ROLE(), address(0x1337)), false
        );

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(0x1337),
                MOVETokenDev(address(token)).MINTER_ROLE()
            )
        );
        vm.prank(address(0x1337));
        MOVETokenDev(address(token)).mint(address(0x1337), 100);

        assertEq(MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ADMIN_ROLE(), multisig), true);
        // Revoke minter admin role
        vm.startPrank(multisig);
        MOVETokenDev(address(token)).revokeMinterAdminRole(multisig);

        assertEq(
            MOVETokenDev(address(token)).hasRole(MOVETokenDev(address(token)).MINTER_ADMIN_ROLE(), multisig), false
        );

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenDev(address(token)).MINTER_ADMIN_ROLE()
            )
        );
        MOVETokenDev(address(token)).grantMinterRole(multisig);

        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                multisig,
                MOVETokenDev(address(token)).MINTER_ROLE()
            )
        );
        MOVETokenDev(address(token)).mint(address(0x1337), 100);
        vm.stopPrank();
    }
}
