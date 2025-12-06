// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Script.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquidV3} from "../src/token/MOVETokenHyperliquidV3.sol";

// OpenZeppelin
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";

/**
 * @title UpgradeMOVETokenHyperliquidV3
 * @notice Script for upgrading MOVETokenHyperliquidV2 to MOVETokenHyperliquidV3 via timelock
 * @dev This script:
 *      1. Deploys the V3 implementation contract
 *      2. Schedules the upgrade through the timelock (requires proposer role)
 *      3. After MIN_DELAY (48 hours), executes the upgrade (requires executor role)
 *      4. Calls initialize on V3 to set up AccessControl roles
 *      5. Verifies the upgrade was successful
 */
contract UpgradeMOVETokenHyperliquidV3 is Script {
    // =============================================================================
    // CONSTANTS - ADDRESSES
    // =============================================================================

    /// @dev Timelock controller address
    address constant TIMELOCK_ADDRESS = 0xA649f6335828f070dDDd7A8c4F5bef2b6FF7Bd51;
    /// @dev Proposer address (multisig)
    address constant PROPOSER_ADDRESS = 0x5cD71FFf9947486fD6F5c193a722191c22728887;
    /// @dev Executor address (multisig)
    address constant EXECUTOR_ADDRESS = 0x443513664Eab95280360Dc1c33FDe1ad4ED5C7bB;
    /// @dev Deployer address
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    /// @dev Proxy Admin address
    address constant PROXY_ADMIN_ADDRESS = 0x8365AA031806A1ac2b31a5d3b8323020FC85DfEc;
    /// @dev MOVE Token Proxy address
    address constant MOVE_TOKEN_PROXY = 0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;
    /// @dev LayerZero endpoint address (Hyperliquid Mainnet)
    address constant LZ_ENDPOINT = 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9;

    // =============================================================================
    // CONSTANTS - CONFIGURATION
    // =============================================================================

    /// @dev Minimum delay for timelock operations (48 hours)
    uint256 public constant MIN_DELAY = 48 hours;
    /// @dev MOVE token decimals
    uint8 public constant MOVE_DECIMALS = 8;

    // =============================================================================
    // STATE VARIABLES
    // =============================================================================

    MOVETokenHyperliquidV3 public moveTokenImplementationV3;
    MOVETokenHyperliquidV3 public moveV3;
    ProxyAdmin public admin;
    TimelockController public timelock;

    // =============================================================================
    // MAIN FUNCTIONS
    // =============================================================================

    /**
     * @dev Deploys the V3 implementation and logs the schedule upgrade parameters for multisig UI
     * Run with: forge script UpgradeMOVETokenHyperliquidV3 --rpc-url <RPC_URL> --broadcast
     *
     */
    function run() public {

        vm.startBroadcast();

        // Deploy new V3 implementation
        moveTokenImplementationV3 = new MOVETokenHyperliquidV3(LZ_ENDPOINT);

        vm.stopBroadcast();

        console.log("===========================================");
        console.log("V3 Implementation Deployed");
        console.log("===========================================");
        console.log("Implementation Address:", address(moveTokenImplementationV3));
        console.log("LayerZero Endpoint:", LZ_ENDPOINT);
        console.log("===========================================");
        console.log("");

        // Prepare upgrade data with initialize call to set up AccessControl
        bytes memory initData = abi.encodeWithSignature("initialize(address)", PROPOSER_ADDRESS);
        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            MOVE_TOKEN_PROXY,
            address(moveTokenImplementationV3),
            initData
        );

        console.log("===========================================");
        console.log("SCHEDULE UPGRADE - MULTISIG PARAMETERS");
        console.log("===========================================");
        console.log("To:", TIMELOCK_ADDRESS);
        console.log("Function: schedule(address,uint256,bytes,bytes32,bytes32,uint256)");
        console.log("");
        console.log("Parameters:");
        console.log("  target:", PROXY_ADMIN_ADDRESS);
        console.log("  value:", vm.toString(uint256(0)));
        console.log("  data:", vm.toString(upgradeData));
        console.log("  predecessor:", vm.toString(bytes32(0)));
        console.log("  salt:", vm.toString(bytes32(0)));
        console.log("  delay:", vm.toString(MIN_DELAY));
        console.log("");
        console.log("Context:");
        console.log("  Implementation:", address(moveTokenImplementationV3));
        console.log("  Proxy:", MOVE_TOKEN_PROXY);
        console.log("  Ready for execution at:", block.timestamp + MIN_DELAY);
        console.log("===========================================");
    }

    /**
     * @dev Verifies the upgrade was successful
     * Run with: forge script UpgradeMOVETokenHyperliquidV3 --sig "verifyUpgrade(address,address)" <IMPLEMENTATION_ADDRESS> <ADMIN_ADDRESS> --rpc-url <RPC_URL>
     *
     * @param implementationAddress Address of the deployed V3 implementation
     */
    function verifyUpgrade(address implementationAddress) public {
        moveV3 = MOVETokenHyperliquidV3(MOVE_TOKEN_PROXY);

        console.log("===========================================");
        console.log("Verifying V3 Upgrade");
        console.log("===========================================");

        // Verify basic token properties
        require(moveV3.decimals() == MOVE_DECIMALS, "Decimals verification failed");
        require(moveV3.sharedDecimals() == MOVE_DECIMALS, "SharedDecimals verification failed");
        require(keccak256(bytes(moveV3.name())) == keccak256(bytes("Movement")), "Name verification failed");
        require(keccak256(bytes(moveV3.symbol())) == keccak256(bytes("MOVE")), "Symbol verification failed");
        require(address(moveV3.endpoint()) == LZ_ENDPOINT, "Endpoint verification failed");
        console.log("Basic token properties verified");

        // Verify implementation address
        bytes32 implementationSlot = vm.load(MOVE_TOKEN_PROXY, bytes32(uint256(keccak256("eip1967.proxy.implementation")) - 1));
        address currentImpl;
        assembly {
            currentImpl := and(implementationSlot, 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF)
        }
        require(currentImpl == implementationAddress, "Implementation verification failed");
        console.log("Implementation updated correctly");
        console.log("  Current Implementation:", currentImpl);

        // Verify AccessControl roles
        bytes32 DEFAULT_ADMIN_ROLE = moveV3.DEFAULT_ADMIN_ROLE();
        bytes32 PAUSER_ROLE = moveV3.PAUSER_ROLE();
        bytes32 UNPAUSER_ROLE = moveV3.UNPAUSER_ROLE();

        require(moveV3.hasRole(DEFAULT_ADMIN_ROLE, PROPOSER_ADDRESS), "DEFAULT_ADMIN_ROLE verification failed");
        require(moveV3.hasRole(PAUSER_ROLE, PROPOSER_ADDRESS), "PAUSER_ROLE verification failed");
        require(moveV3.hasRole(UNPAUSER_ROLE, PROPOSER_ADDRESS), "UNPAUSER_ROLE verification failed");
        console.log("AccessControl roles verified");

        // Verify pausing functionality
        require(!moveV3.paused(), "Contract should not be paused initially");
        console.log("Contract is not paused");

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

        require(fields == hex"0f", "EIP-712 fields verification failed");
        require(keccak256(bytes(name)) == keccak256(bytes("Movement")), "EIP-712 name verification failed");
        require(keccak256(bytes(version)) == keccak256(bytes("1")), "EIP-712 version verification failed");
        require(chainId == block.chainid, "EIP-712 chainId verification failed");
        require(verifyingContract == MOVE_TOKEN_PROXY, "EIP-712 verifying contract verification failed");
        require(domainSalt == bytes32(0), "EIP-712 salt verification failed");
        require(extensions.length == 0, "EIP-712 extensions verification failed");
        console.log("EIP-712 domain verified");

        // Verify finalizer storage slot (should remain from original deployment)
        address finalizer = address(uint160(uint256(vm.load(MOVE_TOKEN_PROXY, keccak256("HyperCore deployer")))));
        require(finalizer == DEPLOYER_ADDRESS, "Finalizer verification failed");
        console.log("Finalizer storage preserved");
        console.log("  Finalizer:", finalizer);

        console.log("===========================================");
        console.log("All Verifications Passed!");
        console.log("===========================================");
    }
}
