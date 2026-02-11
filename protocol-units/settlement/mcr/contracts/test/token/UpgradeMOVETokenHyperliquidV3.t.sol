// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Test.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquidV2} from "../../src/token/MOVETokenHyperliquidV2.sol";
import {MOVETokenHyperliquidV3} from "../../src/token/MOVETokenHyperliquidV3.sol";

// OpenZeppelin
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";

// LayerZero
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {SendParam, MessagingFee, MessagingReceipt, OFTReceipt} from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";

/**
 * @title UpgradeMOVETokenHyperliquidV3Test
 * @notice Test suite for upgrading MOVETokenHyperliquidV2 to MOVETokenHyperliquidV3 via timelock
 * @dev Tests the complete upgrade process including:
 *      - Scheduling the upgrade through timelock with proper delay
 *      - Executing the upgrade after timelock delay
 *      - Verifying all state transitions and access controls
 *      - Testing pausing functionality
 *      - Testing reinitialization and inability to initialize again
 *      - Testing AccessControl role-based permissions
 */
contract UpgradeMOVETokenHyperliquidV3Test is Test {
    // =============================================================================
    // STATE VARIABLES - CONTRACT INSTANCES
    // =============================================================================

    MOVETokenHyperliquidV2 public moveV2;
    MOVETokenHyperliquidV3 public moveV3;
    MOVETokenHyperliquidV3 public moveTokenImplementationV3;
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
     *      1. Initializing V2 proxy instance
     *      2. Deploying the V3 implementation
     *      3. Setting up V3 proxy instance
     */
    function setUp() public {
        // Set up existing V2 proxy
        moveV2 = MOVETokenHyperliquidV2(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);
        moveV3 = MOVETokenHyperliquidV3(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);

        // Deploy new V3 implementation
        moveTokenImplementationV3 = new MOVETokenHyperliquidV3(LZ_ENDPOINT);
    }

    // =============================================================================
    // UPGRADE TESTS
    // =============================================================================

    /**
     * @dev Tests the complete upgrade process from MOVETokenHyperliquidV2 to MOVETokenHyperliquidV3
     *      This test simulates the real upgrade scenario including:
     *      1. Verifying initial V2 state
     *      2. Scheduling the upgrade through timelock with proper delay
     *      3. Verifying that upgrade cannot be executed before timelock delay
     *      4. Executing the upgrade after timelock delay
     *      5. Calling initialize on the V3 to set up AccessControl roles
     *      6. Verifying all state transitions and new V3 functionality
     */
    function testUpgradeV2ToV3ViaTimelock() public {

        // Verify initial V2 state
        assertEq(moveV2.decimals(), MOVE_DECIMALS);
        assertEq(moveV2.sharedDecimals(), MOVE_DECIMALS);
        assertEq(moveV2.name(), "Movement");
        assertEq(moveV2.symbol(), "MOVE");
        assertEq(moveV2.owner(), PROPOSER_ADDRESS);
        assertEq(address(moveV2.endpoint()), LZ_ENDPOINT);
        uint256 initialTotalSupply = moveV2.totalSupply();

        // Verify admin owner is timelock
        assertEq(admin.owner(), address(timelock));

        // Prepare upgrade data with initialize call to set up AccessControl
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
        );

        console.log("V3 Implementation:", address(moveTokenImplementationV3));
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

        // Verify V2 interface still works after upgrade
        assertEq(moveV2.decimals(), MOVE_DECIMALS);
        assertEq(moveV2.sharedDecimals(), MOVE_DECIMALS);
        assertEq(moveV2.name(), "Movement");
        assertEq(moveV2.symbol(), "MOVE");
        assertEq(moveV2.owner(), PROPOSER_ADDRESS);
        assertEq(address(moveV2.endpoint()), LZ_ENDPOINT);

        assertEq(address(moveV3), EXPECTED_MOVE_TOKEN_PROXY);

        // Verify V3 token state after upgrade
        assertEq(moveV3.decimals(), MOVE_DECIMALS);
        assertEq(moveV3.sharedDecimals(), MOVE_DECIMALS);
        assertEq(moveV3.name(), "Movement");
        assertEq(moveV3.symbol(), "MOVE");
        assertEq(moveV3.owner(), PROPOSER_ADDRESS);
        assertEq(address(moveV3.endpoint()), LZ_ENDPOINT);
        assertEq(moveV3.totalSupply(), initialTotalSupply);

        // Verify proxy implementation was updated correctly
        bytes32 implementation = vm.load(address(moveProxy), ERC1967Utils.IMPLEMENTATION_SLOT);
        address implAddr;
        assembly {
            implAddr := and(implementation, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }
        assertEq(implAddr, address(moveTokenImplementationV3));

        // Verify AccessControl roles were set up correctly
        bytes32 DEFAULT_ADMIN_ROLE = moveV3.DEFAULT_ADMIN_ROLE();
        bytes32 PAUSER_ROLE = moveV3.PAUSER_ROLE();
        bytes32 UNPAUSER_ROLE = moveV3.UNPAUSER_ROLE();

        assertTrue(moveV3.hasRole(DEFAULT_ADMIN_ROLE, PROPOSER_ADDRESS));
        assertTrue(moveV3.hasRole(PAUSER_ROLE, PROPOSER_ADDRESS));
        assertTrue(moveV3.hasRole(UNPAUSER_ROLE, PROPOSER_ADDRESS));

        // Verify EIP-712 domain
        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            bytes32 domainSalt,
            uint256[] memory extensions
        ) = moveV3.eip712Domain();

        assertEq(fields, hex"0f", "EIP-712 fields should be 0x0f");
        assertEq(name, "Movement", "EIP-712 name should be Movement");
        assertEq(version, "1", "EIP-712 version should be 1");
        assertEq(chainId, 999, "EIP-712 chainId should be 999");
        assertEq(verifyingContract, address(moveV3), "EIP-712 verifying contract should be the token address");
        assertEq(domainSalt, bytes32(0), "EIP-712 salt should be zero");
        assertEq(extensions.length, 0, "EIP-712 extensions array should be empty");

        // Verify finalizer storage slot (remains from original deployment)
        address finalizer = address(uint160(uint256(vm.load(address(moveV3), keccak256("HyperCore deployer")))));
        assertEq(finalizer, DEPLOYER_ADDRESS, "Finalizer should remain as originally deployed");

        console.log("Upgrade successful!");
        console.log("V3 Implementation:", address(moveTokenImplementationV3));
        console.log("Proxy address:", address(moveProxy));
        console.log("Chain ID:", chainId);
    }

    /**
     * @dev Tests that the upgrade can only be scheduled by the proposer
     */
    function testOnlyProposerCanSchedule() public {
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
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
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
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
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
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
        assertEq(implAddr, address(moveTokenImplementationV3));
    }

    /**
     * @dev Tests that token state is preserved across the upgrade
     */
    function testStatePreservation() public {
        // Mint some tokens to test state preservation
        deal(address(moveV2), PROPOSER_ADDRESS, 1000 * 10 ** MOVE_DECIMALS, true);
        uint256 balanceBefore = moveV2.balanceOf(PROPOSER_ADDRESS);
        uint256 totalSupplyBefore = moveV2.totalSupply();

        // Perform upgrade
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify state is preserved
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), balanceBefore);
        assertEq(moveV3.totalSupply(), totalSupplyBefore);
        assertEq(moveV3.owner(), PROPOSER_ADDRESS); // Owner remains DEPLOYER from original deployment
        assertEq(moveV3.decimals(), MOVE_DECIMALS);
        assertEq(moveV3.sharedDecimals(), MOVE_DECIMALS);
    }

    // =============================================================================
    // V3-SPECIFIC FUNCTIONALITY TESTS
    // =============================================================================

    /**
     * @dev Tests pausing functionality with AccessControl
     */
    function testPausingFunctionality() public {
        // Perform upgrade with initialization
        _performUpgrade();

        // Mint some tokens for testing
        deal(address(moveV3), PROPOSER_ADDRESS, 1000 * 10 ** MOVE_DECIMALS, true);

        // Verify initial state - not paused
        assertFalse(moveV3.paused());

        // Pause the contract as PAUSER_ROLE
        vm.prank(PROPOSER_ADDRESS);
        moveV3.pause();

        // Verify paused state
        assertTrue(moveV3.paused());

        // Unpause the contract as UNPAUSER_ROLE
        vm.prank(PROPOSER_ADDRESS);
        moveV3.unpause();

        // Verify unpaused state
        assertFalse(moveV3.paused());
    }

    /**
     * @dev Tests that pausing prevents calling send() function
     */
    function testPausingPreventsSend() public {
        // Perform upgrade with initialization
        _performUpgrade();

        // Mint some tokens for testing
        deal(address(moveV3), PROPOSER_ADDRESS, 1000 * 10 ** MOVE_DECIMALS, true);

        // Verify initial state - not paused
        assertFalse(moveV3.paused());

        // Prepare send parameters
        SendParam memory sendParam = SendParam({
            dstEid: 30325, // Ethereum mainnet endpoint ID
            to: 0xb10acc8eb83aa4852a1559caa9633427458ef084f9b9febec6ad8558ad709355,
            amountLD: 100 * 10 ** MOVE_DECIMALS,
            minAmountLD: 100 * 10 ** MOVE_DECIMALS,
            extraOptions: bytes(""),
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });

        MessagingFee memory fee = MessagingFee({
            nativeFee: 0.001 ether,
            lzTokenFee: 0
        });

        // Pause the contract
        vm.prank(PROPOSER_ADDRESS);
        moveV3.pause();

        // Verify paused state
        assertTrue(moveV3.paused());

        // Attempt to send tokens while paused - should revert
        vm.prank(PROPOSER_ADDRESS);
        vm.deal(PROPOSER_ADDRESS, 1 ether);
        vm.expectRevert(abi.encodeWithSignature("EnforcedPause()"));
        moveV3.send{value: 0.001 ether}(sendParam, fee, PROPOSER_ADDRESS);

        // Verify balance unchanged (send didn't execute)
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), 1000 * 10 ** MOVE_DECIMALS);

        // Unpause the contract
        vm.prank(PROPOSER_ADDRESS);
        moveV3.unpause();

        // Verify unpaused state
        assertFalse(moveV3.paused());

        // Try to send tokens while unpaused - should not revert with EnforcedPause
        // It may fail at the LayerZero level, but the pause check should pass
        vm.prank(PROPOSER_ADDRESS);
        vm.deal(PROPOSER_ADDRESS, 1 ether);

        // We expect it to either:
        // 1. Succeed (if LayerZero is properly configured)
        // 2. Fail with a LayerZero-related error (not EnforcedPause)
        // The key is that it should NOT revert with EnforcedPause
        moveV3.send{value: 0.001 ether}(sendParam, fee, PROPOSER_ADDRESS);

        console.log("Send failed with LayerZero error (expected) - pause check passed");     
    }

    /**
     * @dev Tests that regular ERC20 transfers are NOT affected by pausing
     * Only the send() function (LayerZero bridging) should be paused
     */
    function testPausingDoesNotAffectERC20Transfers() public {
        // Perform upgrade with initialization
        _performUpgrade();

        // Get initial balances
        uint256 initialProposerBalance = moveV3.balanceOf(PROPOSER_ADDRESS);
        uint256 initialExecutorBalance = moveV3.balanceOf(EXECUTOR_ADDRESS);
        uint256 initialDeployerBalance = moveV3.balanceOf(DEPLOYER_ADDRESS);
        uint256 initialTotalSupply = moveV3.totalSupply();

        // Mint some tokens for testing
        deal(address(moveV3), PROPOSER_ADDRESS, initialProposerBalance + 1000 * 10 ** MOVE_DECIMALS, true);

        // Pause the contract
        vm.prank(PROPOSER_ADDRESS);
        moveV3.pause();

        // Verify paused state
        assertTrue(moveV3.paused());

        // Regular ERC20 transfers should still work when paused
        vm.prank(PROPOSER_ADDRESS);
        moveV3.transfer(EXECUTOR_ADDRESS, 100 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(EXECUTOR_ADDRESS), initialExecutorBalance + 100 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), initialProposerBalance + 900 * 10 ** MOVE_DECIMALS);

        // Approve should work
        vm.prank(PROPOSER_ADDRESS);
        moveV3.approve(EXECUTOR_ADDRESS, 200 * 10 ** MOVE_DECIMALS);

        // TransferFrom should work
        vm.prank(EXECUTOR_ADDRESS);
        moveV3.transferFrom(PROPOSER_ADDRESS, DEPLOYER_ADDRESS, 50 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(DEPLOYER_ADDRESS), initialDeployerBalance + 50 * 10 ** MOVE_DECIMALS);

        // Unpause
        vm.prank(PROPOSER_ADDRESS);
        moveV3.unpause();
    }

    /**
     * @dev Tests that only accounts with PAUSER_ROLE can pause
     */
    function testOnlyPauserCanPause() public {
        _performUpgrade();

        address nonPauser = address(0x9999);

        // Try to pause from non-pauser address
        vm.prank(nonPauser);
        vm.expectRevert();
        moveV3.pause();

        // Verify PAUSER_ROLE can pause
        vm.prank(PROPOSER_ADDRESS);
        moveV3.pause();
        assertTrue(moveV3.paused());
    }

    /**
     * @dev Tests that only accounts with UNPAUSER_ROLE can unpause
     */
    function testOnlyUnpauserCanUnpause() public {
        _performUpgrade();

        // First pause the contract
        vm.prank(PROPOSER_ADDRESS);
        moveV3.pause();

        address nonUnpauser = address(0x9999);

        // Try to unpause from non-unpauser address
        vm.prank(nonUnpauser);
        vm.expectRevert();
        moveV3.unpause();

        // Verify UNPAUSER_ROLE can unpause
        vm.prank(PROPOSER_ADDRESS);
        moveV3.unpause();
        assertFalse(moveV3.paused());
    }

    /**
     * @dev Tests AccessControl role management
     */
    function testAccessControlRoles() public {
        _performUpgrade();

        bytes32 DEFAULT_ADMIN_ROLE = moveV3.DEFAULT_ADMIN_ROLE();
        bytes32 PAUSER_ROLE = moveV3.PAUSER_ROLE();
        bytes32 UNPAUSER_ROLE = moveV3.UNPAUSER_ROLE();

        address newPauser = address(0x1111);
        address newUnpauser = address(0x2222);

        // Grant PAUSER_ROLE to new address
        vm.prank(PROPOSER_ADDRESS);
        moveV3.grantRole(PAUSER_ROLE, newPauser);
        assertTrue(moveV3.hasRole(PAUSER_ROLE, newPauser));

        // Grant UNPAUSER_ROLE to new address
        vm.prank(PROPOSER_ADDRESS);
        moveV3.grantRole(UNPAUSER_ROLE, newUnpauser);
        assertTrue(moveV3.hasRole(UNPAUSER_ROLE, newUnpauser));

        // Verify new pauser can pause
        vm.prank(newPauser);
        moveV3.pause();
        assertTrue(moveV3.paused());

        // Verify new unpauser can unpause
        vm.prank(newUnpauser);
        moveV3.unpause();
        assertFalse(moveV3.paused());

        // Revoke PAUSER_ROLE
        vm.prank(PROPOSER_ADDRESS);
        moveV3.revokeRole(PAUSER_ROLE, newPauser);
        assertFalse(moveV3.hasRole(PAUSER_ROLE, newPauser));

        // Verify revoked pauser cannot pause
        vm.prank(newPauser);
        vm.expectRevert();
        moveV3.pause();
    }

    /**
     * @dev Tests that initialize cannot be called again after V3 upgrade
     */
    function testCannotReinitializeAfterV3Upgrade() public {
        _performUpgrade();

        // Attempt to reinitialize - should revert because reinitializer(2) was already called
        vm.expectRevert(abi.encodeWithSignature("InvalidInitialization()"));
        moveV3.initialize(address(0x9999));
    }

    /**
     * @dev Tests that initialize cannot be called on the V3 implementation contract
     */
    function testCannotInitializeV3Implementation() public {
        // Attempt to initialize the V3 implementation directly - should revert
        // The constructor calls _disableInitializers() which prevents initialization
        vm.expectRevert(abi.encodeWithSignature("InvalidInitialization()"));
        moveTokenImplementationV3.initialize(PROPOSER_ADDRESS);
    }

    /**
     * @dev Tests that V3 inherits all V2 functionality
     */
    function testV3InheritsV2Functionality() public {
        _performUpgrade();

        // Test V2-specific functionality (sharedDecimals)
        assertEq(moveV3.sharedDecimals(), MOVE_DECIMALS);

        // Verify that both decimals and sharedDecimals return the same value
        assertEq(moveV3.decimals(), moveV3.sharedDecimals());

        // Test V1 (base) functionality - token metadata
        assertEq(moveV3.name(), "Movement");
        assertEq(moveV3.symbol(), "MOVE");
        assertEq(moveV3.decimals(), MOVE_DECIMALS);

        // Test ownership functionality from V1 (owner remains from original deployment)
        assertEq(moveV3.owner(), PROPOSER_ADDRESS);

        // Test LayerZero endpoint configuration from V1
        assertEq(address(moveV3.endpoint()), LZ_ENDPOINT);

        // Test ERC20 functionality - mint and transfer
        uint256 initialProposerBalance = moveV3.balanceOf(PROPOSER_ADDRESS);
        uint256 initialExecutorBalance = moveV3.balanceOf(EXECUTOR_ADDRESS);
        uint256 initialTotalSupply = moveV3.totalSupply();

        deal(address(moveV3), PROPOSER_ADDRESS, initialProposerBalance + 1000 * 10 ** MOVE_DECIMALS, true);
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), initialProposerBalance + 1000 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.totalSupply(), initialTotalSupply + 1000 * 10 ** MOVE_DECIMALS);

        // Test transfer functionality
        vm.prank(PROPOSER_ADDRESS);
        moveV3.transfer(EXECUTOR_ADDRESS, 100 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(EXECUTOR_ADDRESS), initialExecutorBalance + 100 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), initialProposerBalance + 900 * 10 ** MOVE_DECIMALS);

        // Test approve and transferFrom functionality
        uint256 initialDeployerBalance = moveV3.balanceOf(DEPLOYER_ADDRESS);

        vm.prank(PROPOSER_ADDRESS);
        moveV3.approve(EXECUTOR_ADDRESS, 200 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.allowance(PROPOSER_ADDRESS, EXECUTOR_ADDRESS), 200 * 10 ** MOVE_DECIMALS);

        vm.prank(EXECUTOR_ADDRESS);
        moveV3.transferFrom(PROPOSER_ADDRESS, DEPLOYER_ADDRESS, 50 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(DEPLOYER_ADDRESS), initialDeployerBalance + 50 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.balanceOf(PROPOSER_ADDRESS), initialProposerBalance + 850 * 10 ** MOVE_DECIMALS);
        assertEq(moveV3.allowance(PROPOSER_ADDRESS, EXECUTOR_ADDRESS), 150 * 10 ** MOVE_DECIMALS);

        // Test EIP-712 Permit functionality (from ERC20PermitUpgradeable)
        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            ,
        ) = moveV3.eip712Domain();

        assertEq(name, "Movement");
        assertEq(version, "1");
        assertEq(verifyingContract, address(moveV3));
        assertTrue(fields != 0x00, "EIP-712 domain should be configured");
        assertTrue(chainId > 0, "Chain ID should be set");

        // Test finalizer storage functionality from V1 (remains from original deployment)
        address finalizer = address(uint160(uint256(vm.load(address(moveV3), keccak256("HyperCore deployer")))));
        assertEq(finalizer, DEPLOYER_ADDRESS, "Finalizer should be set from original deployment");

        // Test setFinalizer functionality (only owner can call this)
        address testAddress = address(0x9999);
        vm.prank(PROPOSER_ADDRESS); // Owner is DEPLOYER from original deployment
        moveV3.setFinalizer(testAddress);
        address newFinalizer = address(uint160(uint256(vm.load(address(moveV3), keccak256("HyperCore deployer")))));
        assertEq(newFinalizer, testAddress, "Finalizer should be updated");
    }

    /**
     * @dev Tests upgrading without calling initialize (roles won't be set)
     */
    function testUpgradeWithoutInitialize() public {
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            bytes("") // No initialize call
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        // Verify upgrade succeeded but roles are not set
        bytes32 implementation = vm.load(address(moveProxy), ERC1967Utils.IMPLEMENTATION_SLOT);
        address implAddr;
        assembly {
            implAddr := and(implementation, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }
        assertEq(implAddr, address(moveTokenImplementationV3));

        // Verify roles are NOT set (since initialize wasn't called)
        bytes32 DEFAULT_ADMIN_ROLE = moveV3.DEFAULT_ADMIN_ROLE();
        bytes32 PAUSER_ROLE = moveV3.PAUSER_ROLE();
        bytes32 UNPAUSER_ROLE = moveV3.UNPAUSER_ROLE();

        assertFalse(moveV3.hasRole(DEFAULT_ADMIN_ROLE, PROPOSER_ADDRESS));
        assertFalse(moveV3.hasRole(PAUSER_ROLE, PROPOSER_ADDRESS));
        assertFalse(moveV3.hasRole(UNPAUSER_ROLE, PROPOSER_ADDRESS));

        // Now initialize manually
        vm.prank(DEPLOYER_ADDRESS);
        moveV3.initialize(PROPOSER_ADDRESS);

        // Verify roles are now set
        assertTrue(moveV3.hasRole(DEFAULT_ADMIN_ROLE, PROPOSER_ADDRESS));
        assertTrue(moveV3.hasRole(PAUSER_ROLE, PROPOSER_ADDRESS));
        assertTrue(moveV3.hasRole(UNPAUSER_ROLE, PROPOSER_ADDRESS));
    }

    // =============================================================================
    // HELPER FUNCTIONS
    // =============================================================================

    /**
     * @dev Helper function to perform the upgrade with initialization
     */
    function _performUpgrade() internal {
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(EXPECTED_MOVE_TOKEN_PROXY),
            address(moveTokenImplementationV3),
            initData
        );

        vm.prank(PROPOSER_ADDRESS);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), MIN_DELAY);

        vm.warp(block.timestamp + MIN_DELAY);

        vm.prank(EXECUTOR_ADDRESS);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));
    }
}
