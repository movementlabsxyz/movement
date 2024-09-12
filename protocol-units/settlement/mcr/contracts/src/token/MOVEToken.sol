// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20PermitUpgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/extensions/ERC20PermitUpgradeable.sol";
import {AccessControlUpgradeable} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract MOVEToken is ERC20PermitUpgradeable, AccessControlUpgradeable {

    /**
     * @dev Disables potential implementation exploit
     */
    constructor() {_disableInitializers();}

    /**
     * @dev Initializes the contract
     * @param _owner The onwer of the initial supply
     * @notice __ERC20_init params: name and symbol are set to "Movement" and "MOVE" respectively
     * @notice __EIP712_init_unchained: name and version are set to "Movement" and "1" respectively
     * @notice _setupRole: DEFAULT_ADMIN_ROLE is set to the owner
     * @notice _mint: 10,000,000,000 MOVE tokens are minted to the owner
     */
    function initialize(address _owner) public initializer {
        __ERC20_init("Movement", "MOVE");
        __EIP712_init_unchained("Movement", "1");
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _mint(address(_owner), 10000000000 * 10 ** decimals());
    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to the Movement network standard decimals
     */
    function decimals() public pure override returns (uint8) {
        return 8;
    }
}