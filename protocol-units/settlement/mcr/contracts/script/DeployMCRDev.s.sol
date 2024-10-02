pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/token/MOVEToken.sol";
import "../src/staking/MovementStaking.sol";
import "../src/settlement/MCR.sol";
import {IMintableToken, MintableToken} from "../src/token/base/MintableToken.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

contract DeployMCRDev is Script {
    function run() external {
        vm.startBroadcast();

        MintableToken moveTokenImplementation = new MintableToken();
        MovementStaking stakingImplementation = new MovementStaking();
        MCR mcrImplementation = new MCR();

        // Deploy the Move Token
        bytes memory moveTokenData = abi.encodeCall(MintableToken.initialize, ("Move Token", "MOVE"));
        address moveTokenProxy = address(new ERC1967Proxy(address(moveTokenImplementation), moveTokenData));

        // Deploy the Movement Staking
        bytes memory movementStakingData =
            abi.encodeCall(MovementStaking.initialize, IMintableToken(address(moveTokenProxy)));
        address movementStakingProxy = address(new ERC1967Proxy(address(stakingImplementation), movementStakingData));

        // Deploy the MCR
        address[] memory custodians = new address[](1);
        custodians[0] = address(moveTokenProxy);
        bytes memory mcrData = abi.encodeCall(
            MCR.initialize, (IMovementStaking(address(movementStakingProxy)), 0, 10, 600 seconds, custodians)
        );
        address mcrProxy = address(new ERC1967Proxy(address(mcrImplementation), mcrData));

        console.log("Move Token Proxy: %s", moveTokenProxy);
        console.log("MCR Proxy: %s", mcrProxy);
        console.log("MCR custodian: %s", MovementStaking(movementStakingProxy).epochDurationByDomain(mcrProxy));
        MintableToken moveToken = MintableToken(moveTokenProxy);
        moveToken.mint(msg.sender, 100000 ether);

        moveToken.grantMinterRole(msg.sender);
        moveToken.grantMinterRole(address(movementStakingProxy));

        vm.stopBroadcast();
    }
}
