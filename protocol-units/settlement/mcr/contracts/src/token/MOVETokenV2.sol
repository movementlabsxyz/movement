// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVETokenV2 is MintableToken {
    mapping(uint256 version => bool state) public versionInitialized;
    /**
     * @dev Initialize the contract
     */
    function initializeV2() public {
        if (versionInitialized[2]) revert AlreadyInitializedV2();
        versionInitialized[2] = true;
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}
