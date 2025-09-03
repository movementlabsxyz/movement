// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {MOVEToken} from "./MOVEToken.sol";
import {OFTUpgradeable, ERC20Upgradeable} from "@layerzerolabs/oft-evm-upgradeable/contracts/oft/OFTUpgradeable.sol";

contract MOVETokenV2 is MOVEToken, OFTUpgradeable {

    /**
     * @dev Disables potential implementation exploit
     */
    constructor(address _endpoint) OFTUpgradeable(_endpoint) {_disableInitializers();}

    /**
     * @dev Initializes the contract with initial parameters.
     * @param _delegate The address of the delegate.
     * @param _revoke The address of the address to revoke role.
     * @param _burned Burns circulation supply on Ethereum.
     */
    function initialize(address _delegate, address _revoke, address[] calldata _burned) external reinitializer(2) {
        __OFTCore_init(_delegate);
        __Ownable_init_unchained(_delegate);
        _grantRole(DEFAULT_ADMIN_ROLE, _delegate);
        _revokeRole(DEFAULT_ADMIN_ROLE, _revoke);
        for (uint256 i = 0; i < _burned.length; i++) {
            _burn(_burned[i], balanceOf(_burned[i]));
        }

    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to 8, following the Movement network standard decimals
     */
    function decimals() public pure override(ERC20Upgradeable, MOVEToken) returns (uint8) {
        return 8;
    }
}