// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MockToken is ERC20 {
    uint256 public amount;
    uint256 public timeLimit;
    uint8 internal decimals_;
    mapping(address => uint256) public requests;

    constructor(string memory name, string memory symbol, uint8 _decimals, uint256 _amount, uint256 _timeLimit)
        public
        ERC20(name, symbol)
    {
        amount = _amount;
        timeLimit = _timeLimit;
        decimals_ = _decimals;
    }

    function decimals() public view override returns (uint8) {
        return decimals_;
    }

    function mint() public {
        require(requests[msg.sender] + timeLimit < block.timestamp, "Request is too soon");
        requests[msg.sender] = block.timestamp;
        _mint(msg.sender, amount);
    }
}
