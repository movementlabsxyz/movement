// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {IWETH9} from "./IWETH9.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";
import {IAtomicBridgeCounterparty} from "./IAtomicBridgeCounterparty.sol";

contract AtomicBridgeCounterparty is IAtomicBridgeCounterparty, Initializable {
    enum MessageState {
        PENDING,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransferDetails {
        bytes32 initiator; // move address
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
    }

    IERC20 public weth;
    mapping(bytes32 => BridgeTransferDetails) public pendingTransfers;
    mapping(bytes32 => BridgeTransferDetails) public completedTransfers;
    mapping(bytes32 => BridgeTransferDetails) public abortedTransfers;

    function initialize(address _weth) public initializer {
        if (_weth == address(0)) {
            revert ZeroAddress();
        }
        weth = IWETH9(_weth);
    }

    function lockBridgeTransferAssets(
        bytes32 initiator,
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        uint256 timeLock,
        address recipient,
        uint256 amount
    ) external returns (bool) {
        require(pendingTransfers[bridgeTransferId].recipient == address(0), "Transfer ID already exists");
        require(amount > 0, "Zero amount not allowed");

        weth.transferFrom(msg.sender, address(this), amount);

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
        require(details.recipient != address(0), "Transfer ID does not exist");

        bytes32 computedHash = keccak256(abi.encodePacked(preImage));
        require(computedHash == details.hashLock, "Invalid preImage");

        delete pendingTransfers[bridgeTransferId];
        completedTransfers[bridgeTransferId] = details;

        weth.transfer(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        require(details.recipient != address(0), "Transfer ID does not exist");
        require(block.timestamp > details.timeLock, "Timelock has not expired");

        delete pendingTransfers[bridgeTransferId];
        abortedTransfers[bridgeTransferId] = details;

        weth.transfer(details.recipient, details.amount);

        emit BridgeTransferCancelled(bridgeTransferId);
    }
}

