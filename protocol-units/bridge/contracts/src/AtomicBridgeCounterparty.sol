// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/Keccak256.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";
import "./IAtomicBridgeCounterparty.sol";

contract AtomicBridgeCounterparty is Ownable, IAtomicBridgeCounterparty {
    using EnumerableMap for EnumerableMap.Bytes32ToAddressMap;

    struct BridgeTransferDetails {
        address initiator; // Ethereum address
        address recipient;
        uint256 amount;
        bytes32 hashLock;
        uint256 timeLock;
    }

    IERC20 public weth;
    mapping(bytes32 => BridgeTransferDetails) public pendingTransfers;
    mapping(bytes32 => BridgeTransferDetails) public completedTransfers;
    mapping(bytes32 => BridgeTransferDetails) public abortedTransfers;

    constructor(IERC20 _weth) {
        weth = _weth;
    }

    function lockBridgeTransferAssets(
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        uint256 timeLock,
        address recipient,
        uint256 amount
    ) external override returns (bool) {
        require(pendingTransfers[bridgeTransferId].initiator == address(0), "Transfer ID already exists");
        require(amount > 0, "Zero amount not allowed");

        weth.transferFrom(msg.sender, address(this), amount);

        pendingTransfers[bridgeTransferId] = BridgeTransferDetails({
            initiator: msg.sender,
            recipient: recipient,
            amount: amount,
            hashLock: hashLock,
            timeLock: block.timestamp + timeLock
        });

        emit BridgeTransferAssetsLocked(bridgeTransferId, recipient, amount, hashLock, timeLock);

        return true;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes memory preImage) external override {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        require(details.initiator != address(0), "Transfer ID does not exist");

        bytes32 computedHash = keccak256(preImage);
        require(computedHash == details.hashLock, "Invalid preImage");

        delete pendingTransfers[bridgeTransferId];
        completedTransfers[bridgeTransferId] = details;

        weth.transfer(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(bytes32 bridgeTransferId) external override onlyOwner {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        require(details.initiator != address(0), "Transfer ID does not exist");
        require(block.timestamp > details.timeLock, "Timelock has not expired");

        delete pendingTransfers[bridgeTransferId];
        abortedTransfers[bridgeTransferId] = details;

        weth.transfer(details.initiator, details.amount);

        emit BridgeTransferCancelled(bridgeTransferId);
    }

    function getPendingTransferDetails(bytes32 bridgeTransferId) external view returns (BridgeTransferDetails memory) {
        return pendingTransfers[bridgeTransferId];
    }

    function getCompletedTransferDetails(bytes32 bridgeTransferId) external view returns (BridgeTransferDetails memory) {
        return completedTransfers[bridgeTransferId];
    }

    function getAbortedTransferDetails(bytes32 bridgeTransferId) external view returns (BridgeTransferDetails memory) {
        return abortedTransfers[bridgeTransferId];
    }
}

