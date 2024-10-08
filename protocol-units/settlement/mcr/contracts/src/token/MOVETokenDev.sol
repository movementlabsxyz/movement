// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVETokenDev is MintableToken {
    mapping(uint256 version => bool state) public versionInitialized;
    error AlreadyInitialized();
    /**
     * @dev Initialize the contract
     */
    function initialize(address multisig) public {
        if (versionInitialized[2]) revert AlreadyInitialized();
        versionInitialized[2] = true;
        _grantRole(MINTER_ADMIN_ROLE, multisig);
        _grantRole(MINTER_ROLE, multisig);
    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}