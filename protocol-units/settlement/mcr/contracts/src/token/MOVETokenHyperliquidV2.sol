// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {MOVETokenHyperliquid} from "./MOVETokenHyperliquid.sol";

contract MOVETokenHyperliquidV2 is MOVETokenHyperliquid {

    /**
     * @dev Disables potential implementation exploit
     */
    constructor(address _endpoint) MOVETokenHyperliquid(_endpoint) {_disableInitializers();}

    /**
     * @dev Returns the number of shared decimals
     * @notice shared decimals is set to 8, following the Movement network standard decimals
     */
    function sharedDecimals() public pure virtual override returns (uint8) {
        return 8;
    }
}
