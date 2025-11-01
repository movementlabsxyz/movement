// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ERC20PermitUpgradeable} from
    "@openzeppelin/contracts-upgradeable/token/ERC20/extensions/ERC20PermitUpgradeable.sol";
import {OFTUpgradeable} from "@layerzerolabs/oft-evm-upgradeable/contracts/oft/OFTUpgradeable.sol";

contract MOVETokenL2 is ERC20PermitUpgradeable, OFTUpgradeable {

    /**
     * @dev Disables potential implementation exploit
     */
    constructor(address _endpoint) OFTUpgradeable(_endpoint) {_disableInitializers();}

    /**
     * @dev Initializes the contract with initial parameters.
     * @param _delegate The address of the delegate.
     */
    function initialize(address _delegate) external initializer {
        __OFT_init("Movement", "MOVE", _delegate);
        __EIP712_init_unchained("Movement", "1");
        __Ownable_init_unchained(_delegate);
    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to 8, following the Movement network standard decimals
     */
    function decimals() public pure virtual override returns (uint8) {
        return 8;
    }
}
