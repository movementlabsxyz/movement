// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import {IAtomicBridgeInitiator} from "./IAtomicBridgeInitiator.sol";
import {IWETH10} from "./WETH/interfaces/IWETH10.sol";
import {Initializable} from "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract AtomicBridgeInitiator is IAtomicBridgeInitiator, Initializable {
    struct BridgeTransfer {
        uint256 amount;
        address originator;
        bytes32 recipient;
        bytes32 hashLock;
        uint256 timeLock;
        bool completed;
    }

    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;
    IWETH10 public weth;
    address public authority;

    modifier onlyAuthorized() {
        if (msg.sender != authority) revert Unauthorized();
        _;
    }

    function initialize(address _weth) public initializer {
        if (_weth == address(0)) {
            revert ZeroAddress();
        }
        authority = msg.sender;
        weth = IWETH10(_weth);
    }

    function initiateBridgeTransfer(uint256 _wethAmount, bytes32 _recipient, bytes32 _hashLock, uint256 _timeLock)
        external
        payable
        returns (bytes32 _bridgeTransferId)
    {
        address originator = msg.sender;
        uint256 ethAmount = msg.value;
        uint256 totalAmount = _wethAmount + ethAmount;
        _bridgeTransferId = keccak256(abi.encodePacked(originator, _recipient, _hashLock, _timeLock, block.timestamp));
        // Check if the bridge transfer already exists
        if (bridgeTransfers[_bridgeTransferId].amount != 0) revert BridgeTransferExists();
        // Ensure there is a valid total amount
        if (totalAmount == 0) {
            revert ZeroAmount();
        }
        // If msg.value is greater than 0, convert ETH to WETH
        if (ethAmount > 0) weth.deposit{value: ethAmount}();

        //Transfer WETH to this contract, revert if transfer fails
        if (_wethAmount > 0) {
            if (!weth.transferFrom(originator, address(this), _wethAmount)) revert WETHTransferFailed();
        }
        // If msg.value is greater than 0, convert ETH to WETH

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: totalAmount,
            originator: originator,
            recipient: _recipient,
            hashLock: _hashLock,
            timeLock: block.timestamp + _timeLock,
            completed: false
        });

        emit BridgeTransferInitiated(_bridgeTransferId, originator, _recipient, totalAmount, _hashLock, _timeLock);
        return _bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 _secret) external onlyAuthorized {
        // Retrieve the bridge transfer
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        uint256 amount = bridgeTransfer.amount;
        if (amount == 0) revert NonExistentBridgeTransfer();
        if (bridgeTransfer.completed) revert BridgeTransferHasBeenCompleted();
        if (keccak256(abi.encodePacked(_secret)) != bridgeTransfer.hashLock) revert InvalidSecret();

        bridgeTransfer.completed = true;

        // Mock transfer to recipient by serializing the address
        // this is supposed to be removed in production
        address recip;
        bytes32 recipient = bridgeTransfer.recipient;
        assembly {
            recip := shr(96, recipient)
        }
        if (!weth.transfer(recip, amount)) revert WETHTransferFailed();
        // payable(bridgeTransfer.recipient).transfer(amount);

        emit BridgeTransferCompleted(_bridgeTransferId, _secret);
    }

    function refundBridgeTransfer(bytes32 _bridgeTransferId) external {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        uint256 amount = bridgeTransfer.amount;
        if (amount == 0) revert NonExistentBridgeTransfer();
        if (bridgeTransfer.completed) revert BridgeTransferHasBeenCompleted();
        if (block.timestamp < bridgeTransfer.timeLock) revert TimeLockNotExpired();

        bridgeTransfer.completed = true;

        // todo: we need to verify if transfers are cheaper in weth or eth
        // then decide on which transfer to use.
        if (!weth.transfer(bridgeTransfer.originator, amount)) revert WETHTransferFailed();
        // payable(bridgeTransfer.originator).transfer(amount);

        emit BridgeTransferRefunded(_bridgeTransferId);
    }

    function receiveFunds(bytes32 _secret) external {
        // This function is used to receive ETH from Moveth Bridge
    }
}
