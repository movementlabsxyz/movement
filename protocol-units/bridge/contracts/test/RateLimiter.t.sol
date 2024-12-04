// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.22;
pragma abicoder v2;

import {Test, console} from "forge-std/Test.sol";
import {AtomicBridgeInitiatorMOVE, IAtomicBridgeInitiatorMOVE, OwnableUpgradeable} from "../src/AtomicBridgeInitiatorMOVE.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {MockMOVEToken} from "../src/MockMOVEToken.sol";  
import {RateLimiter} from "../src/RateLimiter.sol";

contract RateLimiterTest is Test {
    MockMOVEToken public moveToken;   
    RateLimiter public rateLimiterImplementation;
    RateLimiter public rateLimiter;
    ProxyAdmin public proxyAdmin;
    TransparentUpgradeableProxy public proxy;
    AtomicBridgeInitiatorMOVE public atomicBridgeInitiatorMOVE;

    address public originator = address(1);
    address public insuranceFund = address(4);
    address public rateLimiterOperator = address(5);
    bytes32 public recipient = keccak256(abi.encodePacked(address(2)));
    bytes32 public hashLock = keccak256(abi.encodePacked("secret"));
    uint256 public amount = 1 ether;
    uint256 public constant timeLockDuration = 48 * 60 * 60; // 48 hours in seconds

    function setUp() public {
        // Deploy the MOVEToken contract and mint some tokens to the deployer
        moveToken = new MockMOVEToken();
        moveToken.initialize(address(this)); // Contract will hold initial MOVE tokens
        moveToken.transfer(insuranceFund, moveToken.balanceOf(address(this)) / 10); // 10% of the total supply

        originator = vm.addr(uint256(keccak256(abi.encodePacked(block.timestamp, block.prevrandao))));

        rateLimiterImplementation = new RateLimiter();
        proxy = new TransparentUpgradeableProxy(
            address(rateLimiterImplementation),
            address(this),
            abi.encodeWithSignature(
                "initialize(address,address,address,address,address)",
                address(moveToken),
                address(this),
                address(rateLimiterOperator),
                address(0x1337), // just a mock address
                insuranceFund
            )
        );

        rateLimiter = RateLimiter(address(proxy));
    }

   function testSetRateLimitFuzz(uint256 _numerator, uint256 _denominator, uint256 _perTransfer) public {

        _numerator = _numerator % 1000;
        _denominator = _denominator % 1000;
        uint256 _perTransfer = 1 ether * (_perTransfer % 1000);

        vm.prank(rateLimiterOperator);
        if (_numerator == 0) {
            // should fail on division error
        } else if ((_denominator/_numerator) >= 4) {
            rateLimiter.setRateLimiterCoefficients(_numerator, _denominator);

            if (_perTransfer > 0) {
                // rate limit on both inbound and outbound until we exceed the limit for the period
                uint256 totalTransferred = 0;
                // number of iterations should be the total balance of the insurance fund divided by the _perTransfer divided by 2 to check reverts are applied consistently on higher values
                uint256 numberOfIterations = moveToken.balanceOf(insuranceFund) / (_perTransfer / 2);
                uint256 periodMax = moveToken.balanceOf(insuranceFund) * _numerator / _denominator;
                for (uint256 i = 0; i < numberOfIterations; i++) {

                    if (totalTransferred + _perTransfer > periodMax) {
                        vm.expectRevert();
                        rateLimiter.rateLimitOutbound(_perTransfer);
                    } else {
                        rateLimiter.rateLimitOutbound(_perTransfer);
                    }

                    if (totalTransferred + _perTransfer > periodMax) {
                        vm.expectRevert();
                        rateLimiter.rateLimitInbound(_perTransfer);
                    } else {
                        rateLimiter.rateLimitInbound(_perTransfer);
                    }

                    totalTransferred += _perTransfer;
                }
            }

        } else {
            vm.expectRevert("INSURANCE_FUND_MUST_BE_4X_RATE_LIMITER");
            rateLimiter.setRateLimiterCoefficients(_numerator, _denominator);
        }

    }
}

