// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// Foundry
import "forge-std/Test.sol";
import "forge-std/console.sol";

// Project contracts
import {MOVETokenHyperliquid} from "../../src/token/MOVETokenHyperliquid.sol";
import {CREATE3Factory, ICREATE3Factory} from "../../script/helpers/Create3/CREATE3Factory.sol";

// OpenZeppelin
import {TransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";

// Safe
import {Enum} from "@safe-smart-account/contracts/common/Enum.sol";
import {Safe} from "@safe-smart-account/contracts/Safe.sol";

// LayerZero
import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {SetConfigParam} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLibManager.sol";
import {ExecutorConfig} from "@layerzerolabs/lz-evm-messagelib-v2/contracts/SendLibBase.sol";
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

interface IProxyFactory {
    function createProxyWithNonce(address masterCopy, bytes memory data, uint256 nonce)
        external
        returns (address proxy);
}

interface IDelegates {
    function delegates(address) external view returns (address);
}

contract DeployHyperliquidTest is Test {
    // Deployment addresses
    address constant DEPLOYER_ADDRESS = 0xB2105464215716e1445367BEA5668F581eF7d063;
    address constant ZERO = address(0x0);
    address constant MASTER_COPY_ADDRESS = 0xd9Db270c1B5E3Bd161E8c8503c55cEABeE709552;

    // LayerZero endpoint and libraries
    address public lzEndpoint = 0x3A73033C0b1407574C76BdBAc67f126f6b4a9AA9;
    address public receiveUln302 = 0x7cacBe439EaD55fa1c22790330b12835c6884a91;
    address public sendUln302 = 0xfd76d9CB0Bac839725aB79127E7411fe71b1e3CA;
    address public lzExecutor = 0x41Bdb4aa4A63a5b2Efc531858d3118392B1A1C3d;

    // Data Verification Networks (DVNs)
    address public p2pDVN = 0xC7423626016bc40375458bc0277F28681EC91C8e;
    address public horizenDVN = 0xBB83Ecf372CbB6daa629ea9A9A53BEC6d601F229;
    address public lzDVN = 0xc097ab8CD7b053326DFe9fB3E3a31a0CCe3B526f;

    // LayerZero config types
    uint32 public constant EXECUTOR_CONFIG_TYPE = 1;
    uint32 public constant ULN_CONFIG_TYPE = 2;
    uint32 public constant RECEIVE_CONFIG_TYPE = 2;

    // LayerZero parameters
    uint32 public movementEid = 30325;
    uint64 public confirmations = 0;
    bytes32 public movementOapp = 0x7e4fd97ef92302eea9b10f74be1d96fb1f1511cf7ed28867b0144ca89c6ebc3c;

    // Factory contracts
    IProxyFactory public proxyFactory = IProxyFactory(0xa6B71E26C5e0845f74c812102Ca7114b6a896AB2);
    CREATE3Factory public create3 = CREATE3Factory(0x2Dfcc7415D89af828cbef005F0d072D8b3F23183);

    // Token contracts
    MOVETokenHyperliquid public moveTokenImplementation;
    MOVETokenHyperliquid public move = MOVETokenHyperliquid(0x3073f7aAA4DB83f95e9FFf17424F71D4751a3073);

    // Deployment parameters
    bytes32 public salt = 0x6c0000000000000000000000018eddf77afc0a5c6d05a564a44fe37b068922c3;

    function setUp() public {}

    function testDeploy() public {
        vm.startPrank(DEPLOYER_ADDRESS);

        address expectedAddress = 0xd7E22951DE7aF453aAc5400d6E072E3b63BeB7E2;
        address expectedAddress2 = 0x7aE744e3b2816F660054EAbd1a1C4935DA34Ae28;

        bytes memory firstInitData =
            hex"b63e800d00000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000000000000b2105464215716e1445367bea5668f581ef7d06300000000000000000000000049f86aee2c2187870ece0e64570d0048eaf4c75100000000000000000000000012cbb2c9f072e955b6b95ad46213aaa984a4434d000000000000000000000000aff3deeb13bd2b480751189808c16e9809eebcce0000000000000000000000000eed12ca165a962cd12420dfb38407637bca42670000000000000000000000000000000000000000000000000000000000000000";
        bytes memory secondInitData =
            hex"b63e800d0000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000160000000000000000000000000f48f2b2d2a534e402487b3ee7c18c33aec0fe5e4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005afe7a11e70000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000b2105464215716e1445367bea5668f581ef7d0630000000000000000000000003eb69ef2dbedd5d58aa5e074131cd22d5e87ff530000000000000000000000000000000000000000000000000000000000000000";

        address multisigLabsOps = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, firstInitData, 0);
        address multisigDeployer = proxyFactory.createProxyWithNonce(MASTER_COPY_ADDRESS, secondInitData, 0);
        assertEq(multisigLabsOps, expectedAddress);
        assertEq(multisigDeployer, expectedAddress2);

        moveTokenImplementation = new MOVETokenHyperliquid(lzEndpoint);

        bytes memory create3Bytecode = abi.encodePacked(
            type(TransparentUpgradeableProxy).creationCode,
            abi.encode(
                address(moveTokenImplementation),
                address(multisigLabsOps),
                abi.encodeWithSignature("initialize(address)", DEPLOYER_ADDRESS)
            )
        );

        // Deploy MOVE token proxy using CREATE3
        bytes memory bytecode = abi.encodeWithSignature("deploy(bytes32,bytes)", salt, create3Bytecode);
        bytes32 digest = Safe(payable(multisigDeployer)).getTransactionHash(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), 0
        );

        assertEq(vm.addr(vm.envUint("PRIVATE_KEY")), DEPLOYER_ADDRESS);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(vm.envUint("PRIVATE_KEY"), digest);
        bytes memory signature = abi.encodePacked(r, s, v);
        require(signature.length >= 65);
        Safe(payable(multisigDeployer)).execTransaction(
            address(create3), 0, bytecode, Enum.Operation.Call, 0, 0, 0, ZERO, payable(ZERO), signature
        );

        vm.stopPrank();
    }

    function testVerifyMoveTokenDeployment() public {
        testDeploy();

        // Verify basic token properties
        assertEq(move.decimals(), 8);
        assertEq(move.name(), "Movement");
        assertEq(move.symbol(), "MOVE");
        assertEq(move.owner(), DEPLOYER_ADDRESS);
        assertEq(address(move.endpoint()), lzEndpoint);
        assertEq(move.totalSupply(), 0);
        assertTrue(move.DOMAIN_SEPARATOR() != bytes32(0));
        assertEq(IDelegates(address(move.endpoint())).delegates(address(move)), DEPLOYER_ADDRESS);

        // Verify EIP-712 domain
        (
            bytes1 fields,
            string memory name,
            string memory version,
            uint256 chainId,
            address verifyingContract,
            bytes32 domainSalt,
            uint256[] memory extensions
        ) = move.eip712Domain();

        assertEq(fields, hex"0f", "fields should be 0x0f (01111)");
        assertEq(name, "Movement", "EIP-712 name should be Movement");
        assertEq(version, "1", "EIP-712 version should be 1");
        assertEq(chainId, block.chainid, "chainId should match block.chainid");
        assertEq(verifyingContract, address(move), "verifyingContract should be the token address");
        assertEq(domainSalt, bytes32(0), "salt should be zero");
        assertEq(extensions.length, 0, "extensions array should be empty");

        // Verify finalizer storage slot
        address finalizer = address(uint160(uint256(vm.load(address(move), keccak256("HyperCore deployer")))));
        assertEq(finalizer, DEPLOYER_ADDRESS, "finalizer should be the deployer address");
    }

    /**
     * @dev Tests OFT configuration including DVN setup and enforced options
     *      Configures LayerZero infrastructure (DVNs, executor, ULN libraries)
     *      Sets and verifies enforced options for cross-chain messaging
     */
    function testConfigOFT() public {
        testDeploy();
        vm.startPrank(DEPLOYER_ADDRESS);

        configDVNExecutor(address(move));

        // Verify send library configuration
        address receivedSendLib = move.endpoint().getSendLibrary(address(move), movementEid);
        assertEq(receivedSendLib, sendUln302);

        // Verify receive library configuration
        (address receivedReceiveLib,) = move.endpoint().getReceiveLibrary(address(move), movementEid);
        assertEq(receivedReceiveLib, receiveUln302);

        // Set enforced options for gas and execution parameters
        bytes memory options = abi.encodePacked(uint176(0x00030100110100000000000000000000000000001388));
        setEnforcedParams(options);

        // Verify enforced options are set for both message types
        assertEq(move.enforcedOptions(movementEid, uint16(1)), options);
        assertEq(move.enforcedOptions(movementEid, uint16(2)), options);

        move.setPeer(movementEid, movementOapp);
        vm.stopPrank();
    }

    /**
     * @dev Tests cross-chain token sending functionality via LayerZero OFT
     *      Tests the complete flow from quote generation to token burning
     *      Verifies peer setup requirements, balance changes, and restrictions
     *      Tests both basic sends and sends with destination gas drops
     */
    function testSend() public {
        testConfigOFT();

        assertEq(move.totalSupply(), 0);
        uint256 amount = 1 * 10 ** 8;
        deal(address(move), DEPLOYER_ADDRESS, amount, true);
        vm.deal(DEPLOYER_ADDRESS, 1 ether);
        assertEq(move.balanceOf(DEPLOYER_ADDRESS), amount);
        // Movement Multisig address on destination chain
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

        vm.startPrank(DEPLOYER_ADDRESS);

        // Now quote and send should work
        MessagingFee memory fee = move.quoteSend(sendParam, false);
        uint256 totalSupplyBefore = move.totalSupply();
        uint256 balanceBefore = move.balanceOf(DEPLOYER_ADDRESS);
        move.send{value: fee.nativeFee}(sendParam, fee, DEPLOYER_ADDRESS);

        // Send burns tokens on source chain
        assertEq(move.balanceOf(DEPLOYER_ADDRESS), balanceBefore - amount);
        assertEq(move.totalSupply(), totalSupplyBefore - amount);

        vm.stopPrank();
    }

    /**
     * @dev Configures LayerZero DVN (Data Verification Network) and executor settings
     *      Sets up the required infrastructure for cross-chain message verification
     *      Configures 3 DVNs for security and the LayerZero executor
     * @param contractAddress Address of the OFT to configure
     */
    function configDVNExecutor(address contractAddress) public {
        setLibraries(contractAddress, movementEid, sendUln302, receiveUln302);

        // Configure required DVNs for message verification
        address[] memory dvnArray = new address[](3);
        dvnArray[0] = horizenDVN;
        dvnArray[1] = lzDVN;
        dvnArray[2] = p2pDVN;

        address[] memory emptyArray = new address[](0);
        UlnConfig memory ulnConfig = UlnConfig({
            confirmations: uint64(confirmations),
            requiredDVNCount: uint8(3),
            optionalDVNCount: uint8(0),
            optionalDVNThreshold: uint8(0),
            requiredDVNs: dvnArray,
            optionalDVNs: emptyArray
        });
        ExecutorConfig memory executorConfig = ExecutorConfig({maxMessageSize: 0, executor: lzExecutor});

        setConfigs(contractAddress, movementEid, sendUln302, receiveUln302, ulnConfig, executorConfig);
    }

    /**
     * @dev Sets LayerZero configuration parameters for send and receive libraries
     *      Configures executor and ULN settings for both sending and receiving messages
     * @param contractAddress Address of the contract to configure
     * @param remoteEid Remote endpoint ID to configure for
     * @param sendLibraryAddress Address of the send library
     * @param receiveLibraryAddress Address of the receive library
     * @param ulnConfig ULN configuration with DVN settings
     * @param executorConfig Executor configuration settings
     */
    function setConfigs(
        address contractAddress,
        uint32 remoteEid,
        address sendLibraryAddress,
        address receiveLibraryAddress,
        UlnConfig memory ulnConfig,
        ExecutorConfig memory executorConfig
    ) public {
        // Configure send library with executor and ULN configs
        SetConfigParam[] memory sendConfigParams = new SetConfigParam[](2);
        sendConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: EXECUTOR_CONFIG_TYPE, config: abi.encode(executorConfig)});
        sendConfigParams[1] =
            SetConfigParam({eid: remoteEid, configType: ULN_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        // Configure receive library with ULN config
        SetConfigParam[] memory receiveConfigParams = new SetConfigParam[](1);
        receiveConfigParams[0] =
            SetConfigParam({eid: remoteEid, configType: RECEIVE_CONFIG_TYPE, config: abi.encode(ulnConfig)});

        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, sendLibraryAddress, sendConfigParams);
        ILayerZeroEndpointV2(lzEndpoint).setConfig(contractAddress, receiveLibraryAddress, receiveConfigParams);
    }

    /**
     * @dev Sets the send and receive libraries for LayerZero messaging
     * @param _oapp Address of the OmniApp to configure
     * @param _eid Endpoint ID to configure for
     * @param _sendLib Address of the send library
     * @param _receiveLib Address of the receive library
     */
    function setLibraries(address _oapp, uint32 _eid, address _sendLib, address _receiveLib) public {
        ILayerZeroEndpointV2(lzEndpoint).setSendLibrary(_oapp, _eid, _sendLib);
        ILayerZeroEndpointV2(lzEndpoint).setReceiveLibrary(_oapp, _eid, _receiveLib, 0);
    }

    /**
     * @dev Sets enforced options for LayerZero messaging
     *      Applies the same options to both message types (1 and 2)
     * @param options Encoded options for gas and execution parameters
     */
    function setEnforcedParams(bytes memory options) public {
        EnforcedOptionParam[] memory enforcedParams = new EnforcedOptionParam[](2);
        enforcedParams[0] = EnforcedOptionParam({eid: movementEid, msgType: uint16(1), options: options});
        enforcedParams[1] = EnforcedOptionParam({eid: movementEid, msgType: uint16(2), options: options});
        move.setEnforcedOptions(enforcedParams);
    }
}
