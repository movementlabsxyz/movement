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
     * @dev Initializes the contract with initial parameters.
     * @param _owner The address of the owner who receives default admin role.
     * @param _custody The address of the custody account.
     * @notice The ERC20 token is named "Movement" with symbol "MOVE".
     * @notice EIP712 domain version is set to "1" for signatures.
     * @notice The owner is granted the `DEFAULT_ADMIN_ROLE`.
     * @notice 10 billion MOVE tokens are minted to the owner's address.
     */
    function initialize(address _owner, address _custody) public initializer {
        require(_owner != address(0) && _custody != address(0));
        __ERC20_init("Movement", "MOVE");
        __EIP712_init_unchained("Movement", "1");
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _mint(_custody, 10000000000 * 10 ** decimals());
    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to 8, following the Movement network standard decimals
     */
    function decimals() public pure override returns (uint8) {
        return 8;
    }
}