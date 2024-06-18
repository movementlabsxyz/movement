// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {console} from "forge-std/Test.sol";
import {IAtomicBridgeInitiator} from "./interfaces/IAtomicBridgeInitiator.sol";
import {IWETH10} from "./WETH/interfaces/IWETH10.sol";

contract AtomicBridgeInitiator is IAtomicBridgeInitiator {
    struct BridgeTransfer {
        uint256 amount;
        address originator;
        address recipient;
        bytes32 hashLock;
        uint256 timeLock;
        bool completed;
    }

    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;
    IWETH10 public weth;

    constructor(address _weth) {
        weth = IWETH10(_weth);
    }

    function initiateBridgeTransfer(
        uint256 _wethAmount,
        address _originator,
        address _recipient,
        bytes32 _hashLock,
        uint256 _timeLock
    ) external payable override returns (bytes32 _bridgeTransferId) {
        console.log("initiateBridgeTransfer");
        uint256 ethAmount = msg.value;
        uint256 totalAmount = _wethAmount + ethAmount;
        _bridgeTransferId = keccak256(abi.encodePacked(_originator, _recipient, _hashLock, _timeLock, block.timestamp));
        // Check if the bridge transfer already exists
        if (bridgeTransfers[_bridgeTransferId].amount != 0) revert BridgeTransferExists();
        // Ensure there is a valid total amount
        if (totalAmount == 0) revert ZeroAmount();
        //Transfer WETH to this contract, revert if transfer fails
        if (!weth.transfer(address(this), wethAmount)) revert WETHTransferFailed();
        // If msg.value is greater than 0, convert ETH to WETH
        if (ethAmount > 0) weth.deposit{value: ethAmount}();

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: totalAmount,
            originator: _originator,
            recipient: _recipient,
            hashLock: _hashLock,
            timeLock: block.timestamp + _timeLock
        });

        emit BridgeTransferInitiated(_bridgeTransferId, _originator, _recipient, totalAmount, _hashLock, _timeLock);
        return _bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 _secret) external override {
        // Retrieve the bridge transfer
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        uint256 amount = bridgeTransfer.amount;
        if (amount == 0) revert NonExistentBridgeTransfer();
        if (bridgeTransfer.completed) revert BridgeTransferCompleted();
        if (keccak256(abi.encodePacked(_secret)) != bridgeTransfer.hashLock) revert InvalidSecret();

        bridgeTransfer.completed = true;

        // todo: we need to verify if transfers are cheaper in weth or eth
        // then decide on which transfer to use.
        if (!weth.transfer(bridgeTransfer.recipient, amount)) revert WETHTransferFailed();
        // payable(bridgeTransfer.recipient).transfer(amount);

        emit BridgeTransferCompleted(_bridgeTransferId, _secret);
    }

    function refundBridgeTransfer(bytes32 _bridgeTransferId) external override {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        uint256 amount = bridgeTransfer.amount;
        if (amount == 0) revert NonExistentBridgeTransfer();
        if (bridgeTransfer.completed) revert BridgeTransferCompleted();
        if (block.timestamp < bridgeTransfer.timeLock) revert TimeLockNotExpired();

        bridgeTransfer.completed = true;

        // todo: we need to verify if transfers are cheaper in weth or eth
        // then decide on which transfer to use.
        if (!weth.transfer(bridgeTransfer.originator, amount)) revert WETHTransferFailed();
        // payable(bridgeTransfer.originator).transfer(amount);

        emit BridgeTransferRefunded(_bridgeTransferId);
    }
}
