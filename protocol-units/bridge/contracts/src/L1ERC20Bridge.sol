// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract L1ERC20Bridge is Initializable, UUPSUpgradeable, OwnableUpgradeable {
    uint256 public value;

    function initialize(address initialOwner) public initializer {
        __Ownable_init(initialOwner);
    }

    function setValue(uint256 _value) public {
        value = _value;
    }

    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyOwner {}
}
