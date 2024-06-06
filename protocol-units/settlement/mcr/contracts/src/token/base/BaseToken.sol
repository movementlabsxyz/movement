// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract BaseToken is Initializable, ERC20Upgradeable, AccessControlUpgradeable, UUPSUpgradeable {

    /**
     * @dev Initialize the contract
     */
    function initialize(
        string memory name, 
        string memory symbol
    ) initializer public virtual {
        __ERC20_init(name, symbol);
        __AccessControl_init();
        __UUPSUpgradeable_init();
        // __GovernorTimelockControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);

        _mint(address(this), 1000000 * 10 ** decimals());
    }

    /**
     * @dev Authorize an upgrade
     * @param newImplementation The address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}

}