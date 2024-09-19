// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {UUPSUpgradeable} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract BaseToken is Initializable, ERC20Upgradeable, AccessControlUpgradeable, UUPSUpgradeable {
    /**
     * @dev Initialize the contract
     * @param name The name of the token
     * @param symbol The symbol of the token
     */
    function initialize(string memory name, string memory symbol) public virtual initializer {
        __BaseToken_init(name, symbol);
    }
    /**
     * @dev Initialize the contract
     */

    function __BaseToken_init(string memory name, string memory symbol) internal onlyInitializing {
        __ERC20_init_unchained(name, symbol);
        __BaseToken_init_unchained();
    }

    function __BaseToken_init_unchained() internal onlyInitializing {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    /**
     * @dev Authorize an upgrade
     * @param newImplementation The address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}
}
