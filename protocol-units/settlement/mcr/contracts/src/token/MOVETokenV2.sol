// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {MOVEToken} from "./MOVEToken.sol";
import {OFTUpgradeable, ERC20Upgradeable} from "@layerzerolabs/oft-evm-upgradeable/contracts/oft/OFTUpgradeable.sol";
import {PausableUpgradeable} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { SendParam, MessagingFee, MessagingReceipt, OFTReceipt } from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";


contract MOVETokenV2 is MOVEToken, OFTUpgradeable, PausableUpgradeable {

    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant UNPAUSER_ROLE = keccak256("UNPAUSER_ROLE");

    /**
     * @dev Disables potential implementation exploit
     */
    constructor(address _endpoint) OFTUpgradeable(_endpoint) {
        _disableInitializers();
    }

    /**
     * @dev Initializes the contract with initial parameters.
     * @param _delegate The address of the delegate.
     * @param _revoke The address of the address to revoke role.
     * @param _burned Burns circulation supply on Ethereum.
     */
    function initialize(address _delegate, address _revoke, address[] calldata _burned) external reinitializer(2) {
        __OFTCore_init(_delegate);
        __Ownable_init_unchained(_delegate);
        _grantRole(DEFAULT_ADMIN_ROLE, _delegate);
        _grantRole(PAUSER_ROLE, _delegate);
        _grantRole(UNPAUSER_ROLE, _delegate);
        _revokeRole(DEFAULT_ADMIN_ROLE, _revoke);
        for (uint256 i = 0; i < _burned.length; i++) {
            _burn(_burned[i], balanceOf(_burned[i]));
        }
    }

    /**
     * @dev Executes the send operation.
     * @param _sendParam The parameters for the send operation.
     * @param _fee The calculated fee for the send() operation.
     *      - nativeFee: The native fee.
     *      - lzTokenFee: The lzToken fee.
     * @param _refundAddress The address to receive any excess funds.
     * @return msgReceipt The receipt for the send operation.
     * @return oftReceipt The OFT receipt information.
     */
    function send(SendParam calldata _sendParam, MessagingFee calldata _fee, address _refundAddress)
        external
        payable
        override
        whenNotPaused
        returns (MessagingReceipt memory msgReceipt, OFTReceipt memory oftReceipt)
    {
        (uint256 amountSentLD, uint256 amountReceivedLD) = _debit(
            msg.sender,
            _sendParam.amountLD,
            _sendParam.minAmountLD,
            _sendParam.dstEid
        );

        // @dev Builds the options and OFT message to quote in the endpoint.
        (bytes memory message, bytes memory options) = _buildMsgAndOptions(_sendParam, amountReceivedLD);

        // @dev Sends the message to the LayerZero endpoint and returns the LayerZero msg receipt.
        msgReceipt = _lzSend(_sendParam.dstEid, message, options, _fee, _refundAddress);
        // @dev Formulate the OFT receipt.
        oftReceipt = OFTReceipt(amountSentLD, amountReceivedLD);

        emit OFTSent(msgReceipt.guid, _sendParam.dstEid, msg.sender, amountSentLD, amountReceivedLD);
    }

    /**
     * @dev Pauses all bridging.
     * Can only be called by an account with the PAUSER_ROLE.
     */
    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    /**
     * @dev Unpauses all bridging.
     * Can only be called by an account with the UNPAUSER_ROLE.
     */
    function unpause() external onlyRole(UNPAUSER_ROLE) {
        _unpause();
    }

    /**
     * @dev Returns the number of decimals
     * @notice decimals is set to 8, following the Movement network standard decimals
     */
    function decimals() public pure override(ERC20Upgradeable, MOVEToken) returns (uint8) {
        return 8;
    }

    /**
     * @dev Returns the number of shared decimals
     * @notice shared decimals is set to 8, following the Movement network standard decimals
     */
    function sharedDecimals() public pure virtual override returns (uint8) {
        return 8;
    }
}
