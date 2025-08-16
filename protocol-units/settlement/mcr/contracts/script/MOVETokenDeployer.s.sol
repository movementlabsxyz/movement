pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {MOVEToken} from "../src/token/MOVEToken.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import { Helper, ProxyAdmin } from "./helpers/Helper.sol";
import {ICREATE3Factory} from "./helpers/Create3/ICREATE3Factory.sol";

// Script intended to be used for deploying the MOVE token from an EOA
// Utilizies existing safes and sets them as proposers and executors.
// The MOVEToken contract takes in the Movement Foundation address and sets it as its own admin for future upgrades.
// The whole supply is minted to the Movement Foundation Safe.
// The script also verifies that the token has the correct balances, decimals and permissions.
contract MOVETokenDeployer is Helper {
    // COMMANDS
    // mainnet
    // forge script MOVETokenDeployer --fork-url https://eth.llamarpc.com --verify --etherscan-api-key ETHERSCAN_API_KEY
    // testnet
    // forge script MOVETokenDeployer --fork-url https://eth-sepolia.api.onfinality.io/public
    // Safes should be already deployed
    bytes32 public salt = 0x0;
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function run() external virtual {

        // load config and deployments data
        _loadExternalData();

        uint256 signer = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(signer);
        
        // Deploy CREATE3Factory, Safes and Timelock if not deployed
        _deployDependencies();

        deployment.moveAdmin == ZERO && deployment.move == ZERO ?
            _deployMove() : deployment.moveAdmin != ZERO && deployment.move != ZERO ?
                // if move is already deployed, upgrade it
                _upgradeMove() : revert("MOVE: both admin and proxy should be registered");
        
        require(MOVEToken(deployment.move).balanceOf(address(deployment.movementAnchorage)) == 999999998000000000, "Movement Anchorage Safe balance is wrong");
        require(MOVEToken(deployment.move).decimals() == 8, "Decimals are expected to be 8"); 
        require(MOVEToken(deployment.move).totalSupply() == 1000000000000000000,"Total supply is wrong");
        require(MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementFoundationSafe)),"Movement Foundation expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(deployment.movementLabsSafe)),"Movement Labs not expected to have token admin role");
        require(!MOVEToken(deployment.move).hasRole(DEFAULT_ADMIN_ROLE, address(timelock)),"Timelock not expected to have token admin role");
        vm.stopBroadcast();

        if (vm.isContext(VmSafe.ForgeContext.ScriptBroadcast)) {
                _writeDeployments();
            }
    }

    // •☽────✧˖°˖DANGER ZONE˖°˖✧────☾•
// Modifications to the following functions have to be throughly tested

    function _deployMove() internal {
        console.log("MOVE: deploying");
        MOVEToken moveImplementation = new MOVEToken();
        // genetares bytecode for CREATE3 deployment
        bytes memory bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(address(moveImplementation), address(timelock), abi.encodeWithSignature(moveSignature, deployment.movementFoundationSafe, deployment.movementAnchorage))
        );
        vm.recordLogs();
        // deploys the MOVE token proxy using CREATE3
        moveProxy = TransparentUpgradeableProxy(payable(ICREATE3Factory(create3).deploy(salt, bytecode)));
        console.log("MOVEToken deployment records:");
        console.log("proxy", address(moveProxy));
        deployment.move = address(moveProxy);
        deployment.moveAdmin = _storeAdminDeployment();
    }

    function _upgradeMove() internal {
        console.log("MOVE: upgrading");
        MOVEToken newMoveImplementation = new MOVEToken();
        _checkBytecodeDifference(address(newMoveImplementation), deployment.move);
        // Prepare the data for the upgrade
        bytes memory data = abi.encodeWithSignature(
            "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
            address(deployment.moveAdmin),
            0,
            abi.encodeWithSignature(
                "upgradeAndCall(address,address,bytes)",
                address(deployment.move),
                address(newMoveImplementation),
                ""
            ),
            bytes32(0),
            bytes32(0),
            config.minDelay
        );

        _proposeUpgrade(data, "movetoken.json");
    }
}
