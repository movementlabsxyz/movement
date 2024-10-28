// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 value) external returns (bool);
    function decimals() external view returns (uint8);
}

contract MOVEFaucet {

    IERC20 public move;
    uint256 public rateLimit = 1 days;
    uint256 public amount = 10;
    uint256 public maxBalance = 1;
    address public owner;
    mapping(address => uint256) public lastFaucetClaim;

    constructor(IERC20 _move) {
        move = _move;
        owner = msg.sender;
    }

    function faucet() external payable {
        require(msg.value == 10 ** 17, "MOVEFaucet: eth invalid amount");
        require(move.balanceOf(msg.sender) < maxBalance * 10 ** move.decimals(), "MOVEFaucet: balance must be less than determine amount of MOVE");
        require(block.timestamp - lastFaucetClaim[msg.sender] >= rateLimit, "MOVEFaucet: rate limit exceeded");
        lastFaucetClaim[msg.sender] = block.timestamp;
        require(move.transfer(msg.sender, amount * 10 ** move.decimals()), "MOVEFaucet: transfer failed");
    }

    function setConfig(uint256 _rateLimit, uint256 _amount, uint256 _maxBalance, address _owner) external {
        require(msg.sender == owner, "MOVEFaucet: only owner can set config");
        rateLimit = _rateLimit;
        amount = _amount;
        maxBalance = _maxBalance;
        owner = _owner;

    }

    function withdraw() external {
        require(msg.sender == owner, "MOVEFaucet: only owner can retrieve funds");
        (bool status,) = owner.call{value: address(this).balance}("");
        require(status == true, "error during transaction");
    }
}