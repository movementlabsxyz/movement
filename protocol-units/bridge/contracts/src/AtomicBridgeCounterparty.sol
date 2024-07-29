// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {IAtomicBridgeCounterparty} from "./IAtomicBridgeCounterparty.sol";
import {AtomicBridgeInitiator} from "./AtomicBridgeInitator.sol";

contract AtomicBridgeCounterparty is IAtomicBridgeCounterparty, Initializable {
    enum MessageState {
        PENDING,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransferDetails {
        bytes32 initiator; // address of the initiator
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
    }

    AtomicBridgeInitiator public atomicBridgeInitiator;
    mapping(bytes32 => BridgeTransferDetails) public pendingTransfers;
    mapping(bytes32 => BridgeTransferDetails) public completedTransfers;
    mapping(bytes32 => BridgeTransferDetails) public abortedTransfers;

    function initialize(address _atomicBridgeInitiator) public initializer {
        if (_atomicBridgeInitiator == address(0)) {
            revert ZeroAddress();
        }
        atomicBridgeInitiator = AtomicBridgeInitiator(_atomicBridgeInitiator);
    }

    function lockBridgeTransferAssets(
        bytes32 initiator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        uint256 timeLock,
        address recipient,
        uint256 amount
    ) external returns (bool) {
        if (pendingTransfers[bridgeTransferId].recipient != address(0)) {
            revert BridgeTransferInvalid();
        }
        if (amount == 0) {
            revert ZeroAmount();
        }

        pendingTransfers[bridgeTransferId] = BridgeTransferDetails({
            recipient: recipient,
            initiator: initiator,
            amount: amount,
            hashLock: hashLock,
            timeLock: block.timestamp + timeLock
        });

        emit BridgeTransferAssetsLocked(bridgeTransferId, recipient, amount, hashLock, timeLock);

        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        if (details.recipient == address(0)) {
            revert BridgeTransferInvalid();
        }

        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        if (computedHash != details.hashLock) {
            revert InvalidSecret();
        }

        delete pendingTransfers[bridgeTransferId];
        completedTransfers[bridgeTransferId] = details;

        // Call withdrawWETH on AtomicBridgeInitiator to transfer funds to the recipient
        atomicBridgeInitiator.withdrawWETH(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        if (details.recipient == address(0)) {
            revert BridgeTransferInvalid();
        }
        if (block.timestamp <= details.timeLock) {
            revert TimeLockNotExpired();
        }

        delete pendingTransfers[bridgeTransferId];
        abortedTransfers[bridgeTransferId] = details;

        // Call withdrawWETH on AtomicBridgeInitiator to refund the initiator
        atomicBridgeInitiator.withdrawWETH(details.recipient, details.amount);

        emit BridgeTransferCancelled(bridgeTransferId);
    }
}
