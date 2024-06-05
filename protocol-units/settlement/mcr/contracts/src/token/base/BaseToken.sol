// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "openzeppelin-contracts-upgradeable/contracts/token/ERC20/ERC20Upgradeable.sol";
import "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";
import "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import "openzeppelin-contracts-upgradeable/contracts/access/AccessControlUpgradeable.sol";
import "openzeppelin-contracts-upgradeable/contracts/governance/extensions/GovernorTimelockControlUpgradeable.sol";

contract BaseToken is Initializable, ERC20Upgradeable, GovernorTimelockControlUpgradeable, AccessControlUpgradeable, UUPSUpgradeable {

    /**
     * @dev Initialize the contract
     */
    function initialize(
        string memory name, 
        string memory symbol
    ) initializer public {
        __ERC20_init(name, symbol);
        __AccessControl_init();
        __UUPSUpgradeable_init();
        __GovernorTimelockControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);

        _mint(msg.sender, 1000000 * 10 ** decimals());
    }

    /**
     * @dev Authorize an upgrade
     * @param newImplementation The address of the new implementation
     */
    function _authorizeUpgrade(address newImplementation) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}

}