// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ERC20PermitUpgradeable} from
    "@openzeppelin/contracts-upgradeable/token/ERC20/extensions/ERC20PermitUpgradeable.sol";
import {OFTUpgradeable} from "@layerzerolabs/oft-evm-upgradeable/contracts/oft/OFTUpgradeable.sol";

contract MOVETokenHyperliquid is ERC20PermitUpgradeable, OFTUpgradeable {

    /// keccak256("HyperCore deployer")
    bytes32 internal constant FINALIZER_SLOT = 0x8c306a6a12fff1951878e8621be6674add1102cd359dd968efbbe797629ef84f;

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
        _setFinalizer(_delegate);
    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to 8, following the Movement network standard decimals
     */
    function decimals() public pure virtual override returns (uint8) {
        return 8;
    }

    /**
     * @dev Sets the finalizer address.
     * @param _finalizer The address of the finalizer.
     */
    function setFinalizer(address _finalizer) external onlyOwner {
        _setFinalizer(_finalizer);
    }

    function _setFinalizer(address _finalizer) internal {
        assembly {
            sstore(FINALIZER_SLOT, _finalizer)
        }
    }
}
