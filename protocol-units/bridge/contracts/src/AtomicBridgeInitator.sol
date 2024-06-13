import "./IAtomicBridgeInitiator.sol";
import "./WETH/interfaces/IWETH10.sol";
// SPDX-License-Identifier: MIT
pragma solidity ^0.7.6;

contract AtomicBridgeInitiator is IAtomicBridgeInitiator {
    struct BridgeTransfer {
        uint256 amount;
        address originator;
        address recipient;
        bytes32 hashLock;
        uint timeLock;
        bool exists;
    }

    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;
    mapping(bytes32 => BridgeTransfer) public completedBridgeTransfers;
    IWETH10 public weth;

    constructor(address _weth) {
        weth = IWETH10(_weth);
    }

    function initiateBridgeTransfer(
        uint256 _wethAmount,
        address _originator, 
        address _recipient, 
        bytes32 _hashLock, 
        uint _timeLock
    ) external payable override returns (bytes32 _bridgeTransferId) {
        uint256 totalAmount = _wethAmount;

        // If msg.value is greater than 0, convert ETH to WETH and add to total amount
        if (msg.value > 0) {
            weth.deposit{value: msg.value}();
            require(weth.transfer(address(this), msg.value), "WETH transfer failed");
            totalAmount += msg.value;
        }

        // Ensure there is a valid total amount
        require(totalAmount > 0, "Total amount must be greater than 0");

        _bridgeTransferId = keccak256(
            abi.encodePacked(
                _originator, 
                _recipient, 
                _hashLock, 
                _timeLock, 
                block.timestamp
        ));

        // Check if the bridge transfer already exists
        require(bridgeTransfers[_bridgeTransferId].amount == 0, "Bridge transfer already exists");

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: totalAmount,
            originator: _originator,
            recipient: _recipient,
            hashLock: _hashLock,
            timeLock: block.timestamp + _timeLock,
            exists: true
        });

        emit BridgeTransferInitiated(_bridgeTransferId, _originator, _recipient, _hashLock, _timeLock);
        return _bridgeTransferId;
    }

    function completeBridgeTransfer(bytes32 _bridgeTransferId, bytes32 _secret) external override {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        require(bridgeTransfer.exists, "Bridge transfer does not exist");
        require(keccak256(abi.encodePacked(_secret)) == bridgeTransfer.hashLock, "Invalid secret");

        // Move the bridge transfer to completed
        completedBridgeTransfers[_bridgeTransferId] = bridgeTransfer;

        // Delete from active bridgeTransfers
        delete bridgeTransfers[_bridgeTransferId];

        payable(bridgeTransfer.recipient).transfer(bridgeTransfer.amount);

        emit BridgeTransferCompleted(_bridgeTransferId, _secret);
    }

    function refundBridgeTransfer(bytes32 _bridgeTransferId) external override {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        require(bridgeTransfer.exists, "Bridge transfer does not exist");
        require(block.timestamp > bridgeTransfer.timeLock, "Timelock has not expired");

        delete bridgeTransfers[_bridgeTransferId];

        payable(bridgeTransfer.originator).transfer(bridgeTransfer.amount);

        emit BridgeTransferRefunded(_bridgeTransferId);
    }

    function getBridgeTransferDetail(bytes32 _bridgeTransferId) external view override returns (
        bool exists, 
        uint amount, 
        address originator, 
        address recipient, 
        bytes32 hashLock, 
        uint timeLock
    ) {
        BridgeTransfer storage bridgeTransfer = bridgeTransfers[_bridgeTransferId];
        return (
            bridgeTransfer.exists, 
            bridgeTransfer.amount,
            bridgeTransfer.originator,
            bridgeTransfer.recipient,
            bridgeTransfer.hashLock,
            bridgeTransfer.timeLock
        );
    }

    function getCompletedBridgeTransferDetail(bytes32 _bridgeTransferId) external view returns (
        bool exists, 
        uint amount, 
        address originator, 
        address recipient, 
        bytes32 hashLock, 
        uint timeLock
    ) {
        BridgeTransfer storage bridgeTransfer = completedBridgeTransfers[_bridgeTransferId];
        return (
            bridgeTransfer.exists, 
            bridgeTransfer.amount, 
            bridgeTransfer.originator, 
            bridgeTransfer.recipient, 
            bridgeTransfer.hashLock, 
            bridgeTransfer.timeLock
        );
    }
}

