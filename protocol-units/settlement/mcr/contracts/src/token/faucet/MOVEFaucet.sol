// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract MOVEFaucet {

    IERC20 public move;
    uint256 public rateLimit = 1 days;
    uint256 public amount = 10;
    address receiver;
    mapping(address => uint256) public lastFaucetClaim;

    constructor(IERC20 _move) {
        move = _move;
        receiver = msg.sender;
    }

    function faucet() external payable {
        require(msg.value == 1 * 10 ** 17, "MOVEFaucet: invalid amount");
        payable(receiver).transfer(msg.value);
        require(block.timestamp - lastFaucetClaim[msg.sender] >= rateLimit, "MOVEFaucet: rate limit exceeded");
        lastFaucetClaim[msg.sender] = block.timestamp;
        move.transfer(msg.sender, amount * 10 ** move.decimals());
    }
}