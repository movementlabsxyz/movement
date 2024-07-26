// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/keccak256.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";

contract AtomicBridgeCounterparty is Ownable {
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

    event BridgeTransferAssetsLocked(
        bytes32 bridgeTransferId,
        address recipient,
        uint256 amount,
        bytes32 hashLock,
        uint256 timeLock
    );

    event BridgeTransferCompleted(
        bytes32 bridgeTransferId,
        bytes preImage
    );

    event BridgeTransferCancelled(
        bytes32 bridgeTransferId
    );

    constructor(IERC20 _weth) {
        weth = _weth;
    }

    function lockBridgeTransferAssets(
        bytes32 bridgeTransferId,
        bytes32 hashLock,
        uint256 timeLock,
        address recipient,
        uint256 amount
    ) external returns (bool) {
        require(pendingTransfers[bridgeTransferId].initiator == address(0), "Transfer ID already exists");

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

    function completeBridgeTransfer(
        bytes32 bridgeTransferId,
        bytes memory preImage
    ) external {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        require(details.initiator != address(0), "Transfer ID does not exist");

        bytes32 computedHash = keccak256(preImage);
        require(computedHash == details.hashLock, "Invalid preImage");

        delete pendingTransfers[bridgeTransferId];
        completedTransfers[bridgeTransferId] = details;

        weth.transfer(details.recipient, details.amount);

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function abortBridgeTransfer(
        bytes32 bridgeTransferId
    ) external onlyOwner {
        BridgeTransferDetails memory details = pendingTransfers[bridgeTransferId];
        require(details.initiator != address(0), "Transfer ID does not exist");
        require(block.timestamp > details.timeLock, "Timelock has not expired");

        delete pendingTransfers[bridgeTransferId];
        abortedTransfers[bridgeTransferId] = details;

        weth.transfer(details.initiator, details.amount);

        emit BridgeTransferCancelled(bridgeTransferId);
    }
}

