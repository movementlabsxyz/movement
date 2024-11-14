// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IAtomicBridgeInitiatorMOVE} from "./IAtomicBridgeInitiatorMOVE.sol";
import {MockMOVEToken} from "./MockMOVEToken.sol";
import {OwnableUpgradeable} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {ERC20Upgradeable} from "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";

contract AtomicBridgeInitiatorMOVE is IAtomicBridgeInitiatorMOVE, OwnableUpgradeable {
    enum MessageState {
        INITIALIZED,
        COMPLETED,
        REFUNDED
    }

    struct BridgeTransfer {
        uint256 amount;
        address originator;
        bytes32 recipient;
        bytes32 hashLock;
        uint256 timeLock; // in seconds (timestamp)
        MessageState state;
    }

    // Mapping of bridge transfer ids to BridgeTransfer structs
    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;

    address public counterpartyAddress;
    ERC20Upgradeable public moveToken;
    uint256 private nonce;

    uint256 public initiatorTimeLockDuration;

    function initialize(
        address _moveToken,
        address owner,
        uint256 _timeLockDuration
    ) public initializer {
        require(_moveToken != address(0) && owner != address(0), "ZeroAddress");
        moveToken = ERC20Upgradeable(_moveToken);
        __Ownable_init(owner);

        initiatorTimeLockDuration = _timeLockDuration;
    }

    function setCounterpartyAddress(address _counterpartyAddress) external onlyOwner {
        require(_counterpartyAddress != address(0), "ZeroAddress");
        counterpartyAddress = _counterpartyAddress;
    }

    function initiateBridgeTransfer(uint256 moveAmount, bytes32 recipient, bytes32 hashLock)
        external
        returns (bytes32 bridgeTransferId)
    {
        address originator = msg.sender;
            
        require(moveAmount > 0, "ZeroAmount");

        if (!moveToken.transferFrom(originator, address(this), moveAmount)) {
            revert("MOVETransferFailed");
        }

        bridgeTransferId = keccak256(abi.encodePacked(originator, recipient, hashLock, initiatorTimeLockDuration, block.timestamp, nonce++));
        bridgeTransfers[bridgeTransferId] = BridgeTransfer({
            amount: moveAmount,
            originator: originator,
            recipient: recipient,
            hashLock: hashLock,
            timeLock: block.timestamp + initiatorTimeLockDuration,
            state: MessageState.INITIALIZED
        });

        emit BridgeTransferInitiated(bridgeTransferId, originator, recipient, moveAmount, hashLock, initiatorTimeLockDuration);
        return bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 bridgeTransferId, bytes32 preImage) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];

        require(bridgeTransfer.state == MessageState.INITIALIZED, "BridgeTransferHasBeenCompleted");
        require(keccak256(abi.encodePacked(preImage)) == bridgeTransfer.hashLock, "InvalidSecret");
        require(block.timestamp <= bridgeTransfer.timeLock, "TimelockExpired");

        bridgeTransfer.state = MessageState.COMPLETED;

        emit BridgeTransferCompleted(bridgeTransferId, preImage);
    }

    function refundBridgeTransfer(bytes32 bridgeTransferId) external onlyOwner {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[bridgeTransferId];
        require(bridgeTransfer.state == MessageState.INITIALIZED, "BridgeTransferStateNotInitialized");
        require(block.timestamp >= bridgeTransfer.timeLock, "TimeLockNotExpired");

        bridgeTransfer.state = MessageState.REFUNDED;

        if (!moveToken.transfer(bridgeTransfer.originator, bridgeTransfer.amount)) revert("MOVETransferFailed");

        emit BridgeTransferRefunded(bridgeTransferId);
    }
}
