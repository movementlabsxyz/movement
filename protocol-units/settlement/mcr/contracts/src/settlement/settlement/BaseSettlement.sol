// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract BaseSettlement is Initializable, AccessControlUpgradeable, UUPSUpgradeable {

    /**
     * @dev Initialize the contract
     */
    function initialize() public virtual initializer {
        __BaseSettlement_init();
    }

    function __BaseStaking_init() internal onlyInitializing {
        __BaseSettlement_init_unchained();
    }

    function __BaseSettlement_init_unchained() internal onlyInitializing {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    /**
     * @dev Authorize an upgrade
     * @param newImplementation The address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}

}