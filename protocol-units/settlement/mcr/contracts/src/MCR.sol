// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console2.sol";

contract MCR {

    // Use an address set here
    using EnumerableSet for EnumerableSet.AddressSet;

    uint256 public epochDuration;

    // track the last accepted block height, so that we can require blocks are submitted in order and handle staking effectively
    uint256 public lastAcceptedBlockHeight;

    // track the current epoch for staking and unstaking
    uint256 public curentEpoch;

    struct BlockCommitment {
        // currently, to simplify the api, we'll say 0 is uncommitted all other numbers are legitimate heights
        uint256 height;
        bytes commitment;
        bytes blockId;
    }

    // ! ledger for staking and unstaking
    EnumerableSet.AddressSet public validators;
    // preserved records of stake by address per epoch
    mapping(uint256 => mapping( address => uint256)) public epochStakes;
    // preserved records of unstake by address per epoch
    mapping(uint256 => mapping( address => uint256)) public epochUnstakes;

    // map each block height 
    mapping(uint256 => uint256) public blockHeightEpochAssignments;

    // track each commitment from each validator for each block height
    mapping(uint256 => mapping(address => BlockCommitment)) public commitments;

    // track the total stake accumulate for each commitment for each block height
    mapping(uint256 => mapping(bytes => uint256)) public commitmentStakes;

    // map block height to accepted block hash 
    mapping(uint256 => BlockCommitment) public acceptedBlocks;

    event ValidatorStaked(address indexed validator, uint256 stake, uint256 epoch);
    event ValidatorUnstaked(address indexed validator, uint256 stake, uint256 epoch);
    event BlockAccepted(bytes32 indexed blockHash, bytes stateCommitment);
    event BlockCommitmentSubmitted(bytes32 indexed blockHash, bytes stateCommitment, uint256 validatorStake);

    constructor(
        uint256 _epochDurationSecs
    ) {
        epochDuration = _epochDurationSecs;
    }

    // gets the would be epoch for the current block time
    function getEpochByBlockTime() public view returns (uint256) {
        return block.timestamp / epochDuration;
    }

    // gets the current epoch up to which blocks have been accepted
    function getCurrentEpoch() public view returns (uint256) {
        return curentEpoch;
    }

    // gets the next epoch
    function getNextEpoch() public view returns (uint256) {
        return curentEpoch + 1;
    }

    // gets the stake for a given validator at a given epoch
    function getStakeAtEpoch(address validatorAddress, uint256 epoch) public view returns (uint256) {
        return epochStakes[epoch][validatorAddress];
    }

    // gets the stake for a given validator at the current epoch
    function getCurrentEpochStake(address validatorAddress) public view returns (uint256) {
        return getStakeAtEpoch(validatorAddress, getCurrentEpoch());
    }

    // gets the unstake for a given validator at a given epoch
    function getUnstakeAtEpoch(address validatorAddress, uint256 epoch) public view returns (uint256) {
        return epochUnstakes[epoch][validatorAddress];
    }

    // gets the unstake for a given validator at the current epoch
    function getCurrentEpochUnstake(address validatorAddress) public view returns (uint256) {
        return getUnstakeAtEpoch(validatorAddress, getCurrentEpoch());
    }

    // gets the total stake for a given epoch
    function getTotalStakeForEpoch(uint256 epoch) public view returns (uint256) {
        
        uint256 totalStake = 0;
        for (uint256 i = 0; i < validators.length(); i++){
            totalStake += getCurrentEpochStake(validators[i]);
        }
        return totalStake;
    }

    // gets the total stake for the current epoch
    function getTotalStakeForCurrentEpoch() public view returns (uint256) {
        return getTotalStakeForEpoch(getCurrentEpoch());
    }

    // stakes for the next epoch
    function stake() external payable {

        validators.add(msg.sender);
        epochStakes[getNextEpoch()][msg.sender] += msg.value;
        emit ValidatorStaked(msg.sender, msg.value, getNextEpoch());

    }

    // unstakes an amount for the next epoch
    function unstake(uint256 amount) external {

        require(
            epochStakes[getCurrentEpoch()][msg.sender] >= amount,
            "Insufficient stake."
        );

        // indicate that we are going to unstake this amount in the next epoch
        // ! this doesn't actually happen until we roll over the epoch
        // note: by tracking in the next epoch we need to make sure when we roll over an epoch we check the amount rolled over from stake by the unstake in the next epoch
        epochUnstakes[getNextEpoch()][msg.sender] += amount;

        emit ValidatorUnstaked(
            msg.sender,
            amount,
            getNextEpoch()
        );

    }
    
    // rolls over the stake and unstake for a given validator
    function rollOverValidator(address validatorAddress) {

        // the amount of stake rolled over is stake[currentEpoch] - unstake[nextEpoch]
        epochStakes[getNextEpoch()][validatorAddress] = epochStakes[getCurrentEpoch()][validatorAddress] - epochUnstakes[getNextEpoch()][validatorAddress];

        // the unstake is then paid out
        payable(validatorAddress).transfer(epochUnstakes[getNextEpoch()][validatorAddress]);

    }

    // commits a validator to a particular block
    function submitBlockCommitmentForValidator(
        address validatorAddress, 
        BlockCommitment memory blockCommitment
    ) external {

        require(blockCommitment.height == lastAcceptedBlockHeight + 1, "Validator must commit to one greater than the last accepted block height");

        require(commitments[comitment.blockHeight].height != 0, "Validator has already committed to a block at this height");

        // assign the block height to the current epoch if it hasn't been assigned yet
        if (!blockHeightEpochAssignments[blockCommitment.height].isAssigned) {
            blockHeightEpochAssignments[blockCommitment.height].epoch = getCurrentEpoch();
            blockHeightEpochAssignments[blockCommitment.height].isAssigned = true;
        }

        // register the validator's commitment
        commitments[blockCommitment.height][validatorAddress] = blockCommitment;

        // increment the commitment count by stake
        commitmentStakes[blockCommitment.height][blockCommitment.commitment] += getCurrentEpochStake(validatorAddress);

        // if the commitment count is greater than the supermajority stake, accept the block
        uint256 totalStake = getTotalStakeForCurrentEpoch();
        uint256 totalStakeOnCommitment = commitmentStake[blockCommitment.height][blockCommitment.commitment];
        if (totalStakeOnCommitment > (2 * totalStake)/3 ) {
            acceptBlockCommitment(blockCommitment);
        }
      
    }

    function submitBlockCommitment(
        BlockCommitment memory blockCommitment
    ) public {

        submitBlockCommitmentForValidator(msg.sender, blockCommitment);

    }


    function acceptBlockCommitment(BlockCommitment memory blockCommitment) internal {
      
        // set accepted block commitment
        acceptedBlocks[blockCommitment.height] = blockCommitment;

        // set last accepted block height
        lastAcceptedBlockHeight = blockCommitment.height;

        // slash minority validators w.r.t. to the accepted block commitment
        slashMinority(blockCommitment);

        // emit the block accepted event
        emit BlockAccepted(blockCommitment.blockId, blockCommitment.commitment);

        // if the timestamp epoch is greater than the current epoch, roll over the epoch
        if ( getEpochByBlockTime() > getCurrentEpoch() ) {
            rollOverEpoch();
        }
       
    }

    function slashMinority(BlockCommitment memory blockCommitment) internal {


    }

    function rollOverEpoch() internal {

        // iterate over the validator set
        for (uint256 i = 0; i < validators.length(); i++){
            address validatorAddress = validators.at(i);
            rollOverValidator(validatorAddress);
        }

        // increment the current epoch
        curentEpoch += 1;

    }

}