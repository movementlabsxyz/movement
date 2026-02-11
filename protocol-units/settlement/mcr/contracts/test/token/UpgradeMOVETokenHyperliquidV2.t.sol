// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Test.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquid} from "../../src/token/MOVETokenHyperliquid.sol";
import {MOVETokenHyperliquidV2} from "../../src/token/MOVETokenHyperliquidV2.sol";

// OpenZeppelin
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

// LayerZero
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";

/**
 * @title UpgradeMOVETokenHyperliquidV2Test
 * @notice Test suite for upgrading MOVETokenHyperliquid to MOVETokenHyperliquidV2 via timelock
 * @dev Tests the complete upgrade process including:
 *      - Scheduling the upgrade through timelock with proper delay
 *      - Executing the upgrade after timelock delay
 *      - Verifying all state transitions and access controls
 */
contract UpgradeMOVETokenHyperliquidV2Test is Test {
    // =============================================================================
    // STATE VARIABLES - CONTRACT INSTANCES
    // =============================================================================

    MOVETokenHyperliquid public moveV1;
    MOVETokenHyperliquidV2 public moveV2;
    MOVETokenHyperliquidV2 public moveTokenImplementationV2 = MOVETokenHyperliquidV2(0x2ee631fA49F90a98c7210b2220dcA16dA19147D8);
    TransparentUpgradeableProxy public moveProxy = TransparentUpgradeableProxy(payable(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073));
    ProxyAdmin public admin = ProxyAdmin(0x8365AA031806A1ac2b31a5d3b8323020FC85DfEc);

    TimelockController public timelock = TimelockController(payable(0xA649f6335828f070dDDd7A8c4F5bef2b6FF7Bd51));

    // =============================================================================
    // STATE VARIABLES - ADDRESSES
    // =============================================================================

    /// @dev Timelock controller address
    address constant TIMELOCK_ADDRESS = 0xA649f6335828f070dDDd7A8c4F5bef2b6FF7Bd51;
    /// @dev Proposer address (multisig)
    address constant PROPOSER_ADDRESS = 0x5cD71FFf9947486fD6F5c193a722191c22728887;
    /// @dev Executor address (multisig)
    address constant EXECUTOR_ADDRESS = 0x443513664Eab95280360Dc1c33FDe1ad4ED5C7bB;
    /// @dev Deployer address
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;

    /// @dev Expected proxy address
    address constant EXPECTED_MOVE_TOKEN_PROXY = 0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;
    /// @dev LayerZero endpoint address (Hyperliquid Mainnet)
    address constant LZ_ENDPOINT = 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9;

    // =============================================================================
    // STATE VARIABLES - CONFIGURATION
    // =============================================================================

    /// @dev Minimum delay for timelock operations (48 hours)
    uint256 public constant MIN_DELAY = 48 hours;

    /// @dev MOVE token decimals
    uint8 public constant MOVE_DECIMALS = 8;

    // =============================================================================
    // SETUP
    // =============================================================================

    /**
     * @dev Sets up the test environment by:
     *      1. Creating a mock LayerZero endpoint
     *      2. Deploying the V1 implementation and proxy
     *      3. Deploying the ProxyAdmin
     *      4. Deploying the TimelockController
     *      5. Deploying the V2 implementation
     */
    function setUp() public {
        moveV1 = MOVETokenHyperliquid(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);
        moveV2 = MOVETokenHyperliquidV2(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);

    }

    // =============================================================================
    // UPGRADE TESTS
    // =============================================================================

    /**
     * @dev Tests the complete upgrade process from MOVETokenHyperliquid to MOVETokenHyperliquidV2
     *      This test simulates the real upgrade scenario including:
     *      1. Scheduling the upgrade through timelock with proper delay
     *      2. Verifying that upgrade cannot be executed before timelock delay
     *      3. Executing the upgrade after timelock delay
     *      4. Verifying all state transitions
     */
    function testUpgradeViaTimelock() public {
        // Verify initial state
        assertEq(moveV1.decimals(), MOVE_DECIMALS);
        assertEq(moveV1.sharedDecimals(), 6);
        assertEq(moveV1.name(), "Movement");
        assertEq(moveV1.symbol(), "MOVE");
        assertEq(moveV1.owner(), DEPLOYER_ADDRESS);
        assertEq(address(moveV1.endpoint()), LZ_ENDPOINT);
        assertEq(moveV1.totalSupply(), 0);
        assertEq(moveV1.sharedDecimals(), 6);

        // Verify admin owner is timelock
        assertEq(admin.owner(), address(timelock));

        // Prepare upgrade data - NO initialize call per requirements
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            0x2ee631fA49F90a98c7210b2220dcA16dA19147D8,
            bytes("")
        );

        console.log(address(moveTokenImplementationV2));
        console.logBytes(upgradeData);

        // Schedule upgrade via proposer
        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        // Verify that upgrade cannot be executed before timelock delay
        vm.prank(EXECUTOR_ADDRESS);
        vm.expectRevert();
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Fast forward time past the delay
        vm.warp(block.timestamp + MIN_DELAY + 1);

        // Execute upgrade via executor
        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify V1 token state after upgrade (old interface should still work)
        assertEq(moveV1.decimals(), MOVE_DECIMALS);
        assertEq(moveV1.sharedDecimals(), MOVE_DECIMALS); // Despite using the V1 interface, it should also return 8
        assertEq(moveV1.name(), "Movement");
        assertEq(moveV1.symbol(), "MOVE");
        assertEq(moveV1.owner(), DEPLOYER_ADDRESS);
        assertEq(address(moveV1.endpoint()), LZ_ENDPOINT);

        assertEq(address(moveV2), EXPECTED_MOVE_TOKEN_PROXY);

        // Verify V2 token state after upgrade
        assertEq(moveV2.decimals(), MOVE_DECIMALS);
        assertEq(moveV2.sharedDecimals(), MOVE_DECIMALS); // New function in V2
        assertEq(moveV2.name(), "Movement");
        assertEq(moveV2.symbol(), "MOVE");
        assertEq(moveV2.owner(), DEPLOYER_ADDRESS);
        assertEq(address(moveV2.endpoint()), LZ_ENDPOINT);

        // Verify proxy implementation was updated correctly
        bytes32 implementation = vm.load(address(moveProxy), ERC1967Utils.IMPLEMENTATION_SLOT);
        address implAddr;
        assembly {
            implAddr := and(implementation, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }
        assertEq(implAddr, address(moveTokenImplementationV2));

        // Verify EIP-712 domain
        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            bytes32 domainSalt,
            uint256[] memory extensions
        ) = moveV2.eip712Domain();

        assertEq(fields, hex"0f", "EIP-712 fields should be 0x0f");
        assertEq(name, "Movement", "EIP-712 name should be Movement");
        assertEq(version, "1", "EIP-712 version should be 1");
        assertEq(chainId, 999, "EIP-712 chainId should be 999");
        assertEq(verifyingContract, address(moveV2), "EIP-712 verifying contract should be the token address");
        assertEq(domainSalt, bytes32(0), "EIP-712 salt should be zero");
        assertEq(extensions.length, 0, "EIP-712 extensions array should be empty");

        // Verify finalizer storage slot
        address finalizer = address(uint160(uint256(vm.load(address(moveV2), keccak256("HyperCore deployer")))));
        assertEq(finalizer, DEPLOYER_ADDRESS, "Finalizer should be the deployer address");

        console.log("Upgrade successful!");
        console.log("V2 Implementation:", address(moveTokenImplementationV2));
        console.log("Proxy address:", address(moveProxy));
        console.log("Chain ID:", chainId);
    }

    /**
     * @dev Tests that the upgrade can only be scheduled by the proposer
     */
    function testOnlyProposerCanSchedule() public {
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(admin),
            address(moveTokenImplementationV2),
            bytes("")
        );


        // Try to schedule from unauthorized address
        vm.prank(DEPLOYER_ADDRESS);
        vm.expectRevert();
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        // Verify proposer can schedule
        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);
    }

    /**
     * @dev Tests that the upgrade can only be executed by the executor
     */
    function testOnlyExecutorCanExecute() public {
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementationV2),
            bytes("")
        );

        // Schedule upgrade
        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        // Fast forward time
        vm.warp(block.timestamp + MIN_DELAY + 1);

        // Try to execute from unauthorized address
        vm.prank(DEPLOYER_ADDRESS);
        vm.expectRevert();
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify executor can execute
        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));
    }

    /**
     * @dev Tests that the upgrade respects the 48-hour delay
     */
    function testUpgradeRespects48HourDelay() public {
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementationV2),
            bytes("")
        );

        // Schedule upgrade
        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        uint256 scheduleTime = block.timestamp;

        // Try to execute before delay
        vm.warp(scheduleTime + MIN_DELAY - 1);
        vm.prank(EXECUTOR_ADDRESS);
        vm.expectRevert();
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Execute exactly at delay
        vm.warp(scheduleTime + MIN_DELAY);
        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify upgrade succeeded
        bytes32 implementation = vm.load(address(moveProxy), ERC1967Utils.IMPLEMENTATION_SLOT);
        address implAddr;
        assembly {
            implAddr := and(implementation, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }
        assertEq(implAddr, address(moveTokenImplementationV2));
    }

    /**
     * @dev Tests that token state is preserved across the upgrade
     */
    function testStatePreservation() public {
        // Mint some tokens to test state preservation
        deal(address(moveV1), DEPLOYER_ADDRESS, 1000 * 10 ** MOVE_DECIMALS, true);
        uint256 balanceBefore = moveV1.balanceOf(DEPLOYER_ADDRESS);
        uint256 totalSupplyBefore = moveV1.totalSupply();

        // Perform upgrade
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementationV2),
            bytes("")
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify state is preserved
        assertEq(moveV2.balanceOf(DEPLOYER_ADDRESS), balanceBefore);
        assertEq(moveV2.totalSupply(), totalSupplyBefore);
        assertEq(moveV2.owner(), DEPLOYER_ADDRESS);
        assertEq(moveV2.decimals(), MOVE_DECIMALS);
        assertEq(moveV2.sharedDecimals(), MOVE_DECIMALS);
    }

    /**
     * @dev Tests that V2-specific functionality works after upgrade
     */
    function testV2Functionality() public {
        // Perform upgrade
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementationV2),
            bytes("")
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Test V2-specific functionality (sharedDecimals)
        assertEq(moveV2.sharedDecimals(), MOVE_DECIMALS);

        // Verify that both decimals and sharedDecimals return the same value
        assertEq(moveV2.decimals(), moveV2.sharedDecimals());
    }

    /**
     * @dev Tests that initialize cannot be called on the proxy after upgrade
     */
    function testCannotReinitializeProxyAfterUpgrade() public {
        // Perform upgrade
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementationV2),
            bytes("")
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Attempt to reinitialize the proxy - should revert
        vm.expectRevert(abi.encodeWithSignature("InvalidInitialization()"));
        moveV2.initialize(DEPLOYER_ADDRESS);
    }

    /**
     * @dev Tests that initialize cannot be called on the V2 implementation contract
     */
    function testCannotInitializeV2Implementation() public {
        // Attempt to initialize the V2 implementation directly - should revert
        // The constructor calls _disableInitializers() which prevents initialization
        vm.expectRevert(abi.encodeWithSignature("InvalidInitialization()"));
        moveTokenImplementationV2.initialize(DEPLOYER_ADDRESS);
    }

    /**
     * @dev Tests that initialize cannot be called on proxy before upgrade
     */
    function testCannotReinitializeProxyBeforeUpgrade() public {
        // Attempt to reinitialize the proxy before upgrade - should revert
        // The initialize was already called during initial deployment
        vm.expectRevert(abi.encodeWithSignature("InvalidInitialization()"));
        moveV1.initialize(DEPLOYER_ADDRESS);
    }
}
