// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { MOVEHyperLiquidComposer } from "../src/MOVEHyperLiquidComposer.sol";
import { IOFT } from "@layerzerolabs/oft/contracts/interfaces/IOFT.sol";

contract MOVEHyperLiquidComposerToken3073Test is Test {
    // OFT / token on Hyperliquid mainnet
    address internal constant MOVE =
        0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073;

    // Expected native config on mainnet
    address internal constant EXPECTED_NATIVE_ASSET_BRIDGE =
        0x2222222222222222222222222222222222222222;
    uint64 internal constant EXPECTED_NATIVE_CORE_INDEX_ID_MAINNET = 150;
    int8 internal constant EXPECTED_NATIVE_DECIMAL_DIFF = 10;

    // Expected ERC20 / OFT config for 0x3073...
    uint64 internal constant EXPECTED_ERC20_CORE_INDEX_ID = 395;
    int8 internal constant EXPECTED_ERC20_DECIMAL_DIFF = 0;

    function setUp() public {
        // Explicitly simulate Hyperliquid mainnet environment
        // (use the chainId your contracts use to branch mainnet/testnet logic)
        vm.chainId(999);
    }

    function test_config_for_token3073_on_mainnet() public {
        // Wrap the already-deployed OFT with the IOFT interface (no new OFT deployment)
        IOFT oft = IOFT(MOVE);

        // Deploy a fresh composer for this OFT with the expected config
        MOVEHyperLiquidComposer composer = new MOVEHyperLiquidComposer(
            MOVE,
            EXPECTED_ERC20_CORE_INDEX_ID,
            EXPECTED_ERC20_DECIMAL_DIFF
        );

        //
        // --- Native configuration checks (mainnet branch) ---
        //
        assertEq(
            composer.NATIVE_ASSET_BRIDGE(),
            EXPECTED_NATIVE_ASSET_BRIDGE,
            "wrong native asset bridge"
        );

        assertEq(
            composer.NATIVE_CORE_INDEX_ID(),
            EXPECTED_NATIVE_CORE_INDEX_ID_MAINNET,
            "wrong native core index id for mainnet"
        );

        assertEq(
            composer.NATIVE_DECIMAL_DIFF(),
            EXPECTED_NATIVE_DECIMAL_DIFF,
            "wrong native decimal diff"
        );

        //
        // --- ERC20 / OFT configuration checks for 0x3073... ---
        //
        // If ERC20_ASSET_BRIDGE is expected to be the OFT/token address, this enforces it.
        assertEq(
            composer.ERC20_ASSET_BRIDGE(),
            MOVE,
            "wrong ERC20 asset bridge (should be token 0x3073...)"
        );

        assertEq(
            composer.ERC20_CORE_INDEX_ID(),
            EXPECTED_ERC20_CORE_INDEX_ID,
            "wrong ERC20 core index id for token 0x3073..."
        );

        assertEq(
            composer.ERC20_DECIMAL_DIFF(),
            EXPECTED_ERC20_DECIMAL_DIFF,
            "wrong ERC20 decimal diff for token 0x3073..."
        );
    }
}
