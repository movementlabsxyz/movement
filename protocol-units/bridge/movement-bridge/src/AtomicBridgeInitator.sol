import "./IAtomicBridgeInitiator.sol";
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

contract AtomicBridgeInitiator is IAtomicBridgeInitiator {
    struct BridgeTransfer {
        uint amount;
        address originator;
        address recipient;
        bytes32 hashLock;
        uint timeLock;
        bool exists;
    }

    mapping(bytes32 => BridgeTransfer) public bridgeTransfers;
    mapping(bytes32 => BridgeTransfer) public completedBridgeTransfers;

    function initiateBridgeTransfer(
        uint amount, 
        address _originator, 
        address _recipient, 
        bytes32 _hashLock, 
        uint _timeLock
    ) external payable override returns (bytes32 _bridgeTransferId) {
        require(msg.value == amount, "Amount mismatch with the sent value");

        _bridgeTransferId = keccak256(abi.encodePacked(_originator, _recipient, _hashLock, _timeLock, block.timestamp));
        require(!bridgeTransfers[_bridgeTransferId].exists, "Bridge transfer already exists");

        bridgeTransfers[_bridgeTransferId] = BridgeTransfer({
            amount: amount,
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

