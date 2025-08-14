// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

// =============================================================================
// IMPORTS
// =============================================================================

// Forge testing framework
import "forge-std/Test.sol";

// Local contracts
import {MOVEToken} from "../../src/token/MOVEToken.sol";
import {MOVETokenV2} from "../../src/token/MOVETokenV2.sol";

// OpenZeppelin contracts
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {TimelockController} from "@openzeppelin/contracts/governance/TimelockController.sol";
import {ERC1967Utils} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

// LayerZero contracts
import {EndpointV2Mock} from "@layerzerolabs/test-devtools-evm-foundry/contracts/mocks/EndpointV2Mock.sol";
import {ExecutorConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/SendLibBase.sol";
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {SetConfigParam} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";
import {UlnConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/uln/UlnBase.sol";
import {EnforcedOptionParam} from "@layerzerolabs/oapp-evm/contracts/oapp/interfaces/IOAppOptionsType3.sol";
import {
    IOFT,
    SendParam,
    OFTLimit,
    OFTReceipt,
    OFTFeeDetail,
    MessagingReceipt,
    MessagingFee
} from "@layerzerolabs/oft-evm/contracts/interfaces/IOFT.sol";
import {MessagingFee} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {OFTAdapter} from "lib/LayerZero-v2/packages/layerzero-v2/evm/oapp/contracts/oft/OFTAdapter.sol";

// Safe contracts
import {CompatibilityFallbackHandler} from "@safe-smart-account/contracts/handler/CompatibilityFallbackHandler.sol";

contract MOVETokenV2Test is Test {
    // =============================================================================
    // STATE VARIABLES - CONTRACT INSTANCES
    // =============================================================================

    MOVEToken public move;
    MOVETokenV2 public move2;
    MOVETokenV2 public moveTokenImplementation2;

    bytes32 public moveOftAdapterBytes32 = 0x7e4fd97ef92302eea9b10f74be1d96fb1f1511cf7ed28867b0144ca89c6ebc3c;

    ProxyAdmin public admin = ProxyAdmin(payable(0x8365AA031806A1ac2b31a5d3b8323020FC85DfEc));
    TransparentUpgradeableProxy public moveProxy =
        TransparentUpgradeableProxy(payable(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073));
    TimelockController public timelock = TimelockController(payable(0x25a5A3FA61cba5Fd5fb1D75D0AcfEB81370778Eb));
    ILayerZeroEndpointV2 public endpoint = ILayerZeroEndpointV2(payable(0x1a44076050125825900e736c501f859c50fE728c));

    // =============================================================================
    // STATE VARIABLES - ADDRESSES
    // =============================================================================

    address public anchorage = 0xe3e86E126fcCd071Af39a0899734Ca5C8E5F4F25;
    address public labs = 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2;
    address public foundation = 0xB304C899EcB46DD91F31Ef0d177fF9dAf8C17edf;
    address public bridge = 0xf1dF43A3053cd18E477233B59a25fC483C2cBe0f;
    address public oldFoundation = 0x074C155f09cE5fC3B65b4a9Bbb01739459C7AD63;

    // LayerZero infrastructure addresses
    address public sendUln302 = 0xbB2Ea70C9E858123480642Cf96acbcCE1372dCe1;
    address public receiveUln302 = 0xc02Ab410f0734EFa3F14628780e6e695156024C2;
    address public lzExecutor = 0x173272739Bd7Aa6e4e214714048a9fE699453059;

    // DVN (Data Verification Network) addresses
    address public p2pDVN = 0x06559EE34D85a88317Bf0bfE307444116c631b67;
    address public horizenDVN = 0x380275805876Ff19055EA900CDb2B46a94ecF20D;
    address public lzDVN = 0x589dEDbD617e0CBcB916A9223F4d1300c294236b;
    address public nethermindDVN = 0xa59BA433ac34D2927232918Ef5B2eaAfcF130BA5;

    // =============================================================================
    // STATE VARIABLES - CONFIGURATION
    // =============================================================================

    string public moveSignature = "initialize(address,address)";
    uint64 public confirmations = 0;
    uint32 public movementEid = 30325;
    uint256 public minDelay = 2 days;

    // =============================================================================
    // STATE VARIABLES - CONSTANTS
    // =============================================================================

    uint32 public constant EXECUTOR_CONFIG_TYPE = 1;
    uint32 public constant ULN_CONFIG_TYPE = 2;
    uint32 public constant RECEIVE_CONFIG_TYPE = 2;

    // =============================================================================
    // SETUP
    // =============================================================================

    function setUp() public {
        moveTokenImplementation2 = new MOVETokenV2(address(endpoint));

        move = MOVEToken(address(moveProxy));
        move2 = MOVETokenV2(address(moveProxy));
    }

    // =============================================================================
    // BASIC TOKEN TESTS
    // =============================================================================

    function testCannotInitializeTwice() public {
        vm.expectRevert(0xf92ee8a9);
        move.initialize(labs, anchorage);
    }

    function testDecimals() public {
        assertEq(move.decimals(), 8);
    }

    function testTotalSupply() public {
        assertEq(move.totalSupply(), 10000000000 * 10 ** 8);
    }

    function testAdminRoleFuzz(address other) public {
        assertEq(move.hasRole(0x00, other), false);

        vm.expectRevert(
            abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, address(this), 0x00)
        );
        move.grantRole(0x00, other);
    }

    // =============================================================================
    // UPGRADE TESTS
    // =============================================================================

    function testUpgradeFromTimelock() public {
        assertEq(admin.owner(), address(timelock));
        assertEq(move.hasRole(0x00, oldFoundation), true);
        assertEq(move.hasRole(0x00, anchorage), false);

        // TODO: define balances to burn
        address[] memory deprecated = new address[](1);
        deprecated[0] = bridge;

        bytes memory initializeData =
            abi.encodeWithSignature("initialize(address,address,address[])", labs, oldFoundation, deprecated);

        bytes memory upgradeData = abi.encodeWithSignature(
            "upgradeAndCall(address,address,bytes)",
            address(moveProxy),
            address(moveTokenImplementation2),
            initializeData
        );

        vm.prank(labs);
        timelock.schedule(address(admin), 0, upgradeData, bytes32(0), bytes32(0), minDelay);

        // while scheduled, labs pauses the existing bridge, this can occur simultaneously with the schedule call
        vm.startPrank(labs);
        OFTAdapter(payable(bridge)).setPeer(30325, 0x0);

        vm.expectRevert();
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.warp(block.timestamp + minDelay + 1);

        uint256 bridgeBalance = move.balanceOf(bridge);

        vm.prank(foundation);
        timelock.execute(address(admin), 0, upgradeData, bytes32(0), bytes32(0));

        assertEq(move.decimals(), 8);
        assertEq(move.totalSupply(), 10000000000 * 10 ** 8 - bridgeBalance);
        assertEq(move.balanceOf(bridge), 0);
        assertEq(move.hasRole(0x00, oldFoundation), false);
        assertEq(move.hasRole(0x00, foundation), false);
        assertEq(move.hasRole(0x00, labs), true);
        assertEq(move.hasRole(0x00, anchorage), false);
    }

    // =============================================================================
    // OFT (OMNICHAIN FUNGIBLE TOKEN) TESTS
    // =============================================================================

    function testOFTFeatures() public {
        testUpgradeFromTimelock();

        assertEq(address(move2.endpoint()), address(endpoint));
        assertEq(move2.decimals(), 8);
    }

    function testConfigOFT() public {
        testUpgradeFromTimelock();

        vm.startPrank(labs);
        configDVNExecutor(address(move2));

        address receivedSendLib = move2.endpoint().getSendLibrary(address(move2), movementEid);
        assertEq(receivedSendLib, sendUln302);

        (address receivedReceiveLib,) = move2.endpoint().getReceiveLibrary(address(move2), movementEid);
        assertEq(receivedReceiveLib, receiveUln302);

        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000001388));
        setEnforcedParams(options);
        vm.stopPrank();
    }

    function testSend() public {
        testConfigOFT();

        console.log("anchorage balance");
        console.log(move2.balanceOf(anchorage));

        uint256 amount = 1 * 10 ** 8;
        vm.prank(anchorage);

        // Movement Multisig
        bytes32 moveAddress = 0x98ebb7985c84a89972022edf391bdaa7d95f061d9742efb3703de368413431e1;
        SendParam memory sendParam = SendParam({
            dstEid: movementEid,
            to: moveAddress,
            amountLD: amount,
            minAmountLD: amount,
            extraOptions: bytes(""),
            composeMsg: bytes(""),
            oftCmd: bytes("")
        });

        vm.expectRevert();
        move2.quoteSend(sendParam, false);

        vm.prank(anchorage);
        vm.expectRevert();
        move2.send(sendParam, MessagingFee({nativeFee: 100000000000000, lzTokenFee: 0}), bridge);

        vm.prank(labs);
        move2.setPeer(movementEid, moveOftAdapterBytes32);

        MessagingFee memory fee = move2.quoteSend(sendParam, false);

        vm.prank(anchorage);
        move2.send{value: fee.nativeFee}(sendParam, fee, bridge);
    }

    // =============================================================================
    // HELPER FUNCTIONS
    // =============================================================================

    function string2Address(bytes memory str) public pure returns (address addr) {
        bytes32 data = keccak256(str);
        assembly {
            mstore(0, data)
            addr := mload(0)
        }
    }

    function configDVNExecutor(address adapter) public {
        setLibraries(adapter, movementEid, sendUln302, receiveUln302);

        address[] memory dvnArray = new address[](3);
        dvnArray[0] = p2pDVN;
        dvnArray[1] = horizenDVN;
        dvnArray[2] = lzDVN;

        address[] memory emptyArray = new address[](0);
        UlnConfig memory ulnConfig =
            UlnConfig(uint64(confirmations), uint8(3), uint8(0), uint8(0), dvnArray, emptyArray);
        ExecutorConfig memory executorConfig = ExecutorConfig(0, lzExecutor);
        setConfigs(adapter, movementEid, sendUln302, receiveUln302, ulnConfig, executorConfig);
    }

    function setConfigs(
        address contractAddress,
        uint32 remoteEid,
        address sendLibraryAddress,
        address receiveLibraryAddress,
        UlnConfig memory ulnConfig,
        ExecutorConfig memory executorConfig
    ) public {
        SetConfigParam[] memory sendConfigParams = new SetConfigParam[](2);

        sendConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: EXECUTOR_CONFIG_TYPE, config: abi.encode(executorConfig)});

        sendConfigParams[1] =
            SetConfigParam({eid: remoteEid, configType: ULN_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        SetConfigParam[] memory receiveConfigParams = new SetConfigParam[](1);
        receiveConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: RECEIVE_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        endpoint.setConfig(contractAddress, sendLibraryAddress, sendConfigParams);
        endpoint.setConfig(contractAddress, receiveLibraryAddress, receiveConfigParams);
    }

    function setLibraries(address _oapp, uint32 _eid, address _sendLib, address _receiveLib) public {
        endpoint.setSendLibrary(_oapp, _eid, _sendLib);
        endpoint.setReceiveLibrary(_oapp, _eid, _receiveLib, 0);
    }

    function setEnforcedParams(bytes memory options) public {
        EnforcedOptionParam[] memory enforcedParams = new EnforcedOptionParam[](2);
        enforcedParams[0] = EnforcedOptionParam({eid: movementEid, msgType: uint16(1), options: options});
        enforcedParams[1] = EnforcedOptionParam({eid: movementEid, msgType: uint16(2), options: options});
        move2.setEnforcedOptions(enforcedParams);
    }
}
