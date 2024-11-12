pragma solidity ^0.8.13;

import {IERC20} from "@openzeppelin/contracts/interfaces/IERC20.sol";

// canonical order: domain, epoch, custodian, attester, stake =? decas
interface IMovementStaking {
    function registerDomain(
        uint256 epochDuration,
        address[] calldata custodians
    ) external;
    function acceptGenesisCeremony() external;
    function setGenesisCeremony(
        address[] calldata,
        address[] calldata,
        uint256[] calldata
    ) external;
    function getEpochByL1BlockTime(address) external view returns (uint256);
    function getCurrentEpoch(address) external view returns (uint256);
    function getNextEpoch(address) external view returns (uint256);
    function getNextEpochByBlockTime(address) external view returns (uint256);
    function getStakeAtEpoch(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) external view returns (uint256);
    function getCurrentEpochStake(
        address domain,
        address custodian,
        address attester
    ) external view returns (uint256);
    function getUnstakeAtEpoch(
        address domain,
        uint256 epoch,
        address custodian,
        address attester
    ) external view returns (uint256);
    function getCurrentEpochUnstake(
        address domain,
        address custodian,
        address attester
    ) external view returns (uint256);
    function getTotalStakeForEpoch(
        address domain,
        uint256 epoch,
        address custodian
    ) external view returns (uint256);
    function getTotalStakeForCurrentEpoch(
        address domain,
        address custodian
    ) external view returns (uint256);
    function stake(address domain, IERC20 custodian, uint256 amount) external;
    function unstake(
        address domain,
        address custodian,
        uint256 amount
    ) external;
    function getCustodiansByDomain(
        address domain
    ) external view returns (address[] memory);
    function getAttestersByDomain(
        address domain
    ) external view returns (address[] memory);
    function rollOverEpoch() external;
    function slash(
        address[] calldata custodians,
        address[] calldata attesters,
        uint256[] calldata amounts,
        uint256[] calldata refundAmounts
    ) external;

    function whitelistAddress(address addr) external;
    function removeAddressFromWhitelist(address addr) external;

    event AttesterStaked(
        address indexed domain,
        uint256 indexed epoch,
        address indexed custodian,
        address attester,
        uint256 stake
    );

    event AttesterUnstaked(
        address indexed domain,
        uint256 indexed epoch,
        address indexed custodian,
        address attester,
        uint256 stake
    );

    event AttesterEpochRolledOver(
        address indexed attester,
        uint256 indexed epoch,
        address indexed custodian,
        uint256 stake,
        uint256 unstake
    );

    event EpochRolledOver(address indexed domain, uint256 epoch);
    
    error StakeExceedsGenesisStake();
    error CustodianTransferAmountMismatch();
    error GenesisAlreadyAccepted();
}
