// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVETokenV2 is MintableToken {
    mapping(uint256 version => bool state) public versionInitialized;
    error AlreadyInitialized();
    /**
     * @dev Initialize the contract
     */
    function initialize() public {
        if (versionInitialized[2]) revert AlreadyInitialized();
        versionInitialized[2] = true;
        address multisig = address(0x00db70A9e12537495C359581b7b3Bc3a69379A00);
        _grantRole(MINTER_ADMIN_ROLE, multisig);
        _grantRole(MINTER_ROLE, multisig);
    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}
