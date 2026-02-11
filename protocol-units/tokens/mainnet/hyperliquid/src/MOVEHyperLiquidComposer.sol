// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;

import { HyperLiquidComposer } from "@layerzerolabs/hyperliquid-composer/contracts/HyperLiquidComposer.sol";

/// @dev This contract is a composer that allows transfers of ERC20 and HYPE tokens to a target address on hypercore.
///
/// @dev This contract does NOT refund dust because we do not expect any due to truncation of sharedDecimals.
///      In the off-chance that you have dust you would have to implement dust refunds to the receiver in:
///      `_transferERC20ToHyperCore` and `_transferNativeToHyperCore`
contract MOVEHyperLiquidComposer is HyperLiquidComposer {
    /// @notice Constructor for the HyperLiquidComposer
    ///
    /// @param _oft The address of the OFT
    /// @param _hlIndexId The HyperLiquid core spot's index value
    /// @param _assetDecimalDiff The difference in decimals between the HyperEVM's ERC20 and the HyperLiquid HIP-1 token
    constructor(
        address _oft,
        uint64 _hlIndexId,
        int8 _assetDecimalDiff
    ) HyperLiquidComposer(_oft, _hlIndexId, _assetDecimalDiff) {}
}