// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./base/MintableToken.sol";

contract MOVETokenDev is MintableToken {

    /**
     * @dev Initialize the contract
     */
    function initialize(address manager) public initializer {
        __MintableToken_init("Movement", "MOVE");
        _mint(manager, 10000000000 * 10 ** decimals());
        _grantRole(MINTER_ADMIN_ROLE, manager);
        _grantRole(MINTER_ROLE, manager);
    }

    function grantRoles(address account) public onlyRole(DEFAULT_ADMIN_ROLE) {
        _grantRole(MINTER_ADMIN_ROLE, account);
        _grantRole(MINTER_ROLE, account);

    }

    function decimals() public pure override returns (uint8) {
        return 8;
    }
}