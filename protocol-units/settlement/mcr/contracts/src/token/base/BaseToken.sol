// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {UUPSUpgradeable} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

abstract contract BaseToken is Initializable, ERC20Upgradeable, AccessControlUpgradeable, UUPSUpgradeable {
    /**
     * @dev Initialize the contract
     */
    function __BaseToken_init(string memory name, string memory symbol) internal onlyInitializing {
        __ERC20_init_unchained(name, symbol);
        __AccessControl_init_unchained();
        __UUPSUpgradeable_init_unchained();
        __BaseToken_init_unchained();
        // __GovernorTimelockControl_init();
    }

    function __BaseToken_init_unchained() internal onlyInitializing {
        grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _mint(address(this), 1000000 * 10 ** decimals());
    }

    /**
     * @dev Authorize an upgrade
     * @param newImplementation The address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}
}
