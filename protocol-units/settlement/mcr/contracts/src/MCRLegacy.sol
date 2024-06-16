// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import "forge-std/console.sol";

contract MCRLegacy {
    // Use an address set here
    using EnumerableSet for EnumerableSet.AddressSet;

    uint256 public genesisStakeRequired;
    uint256 public maxGenesisStakePerValidator;
    uint256 public genesisStakeAccumulated;

    uint256 public epochDuration;

    // the number of blocks that can be submitted ahead of the lastAcceptedBlockHeight
    // this allows for things like batching to take place without some validators locking down the validator set by pushing too far ahead
    // ? this could be replaced by a 2/3 stake vote on the block height to epoch assignment
    // ? however, this protocol becomes more complex as you to take steps to ensure that...
    // ? 1. Block heights have a non-decreasing mapping to epochs
    // ? 2. Votes get accumulated reasonable near the end of the epoch (i.e., your vote is cast for the epoch you vote fore and the next)
    // ? if howevever, you simply allow a race with the tolerance below, both of these are satisfied without the added complexity
    uint256 public leadingBlockTolerance;

    // track the last accepted block height, so that we can require blocks are submitted in order and handle staking effectively
    uint256 public lastAcceptedBlockHeight;

    // track the current epoch for staking and unstaking
    uint256 public currentEpoch;

    struct BlockCommitment {
        // currently, to simplify the api, we'll say 0 is uncommitted all other numbers are legitimate heights
        uint256 height;
        bytes32 commitment;
        bytes32 blockId;
    }

    // ! ledger for staking and unstaking
    EnumerableSet.AddressSet internal validators;
    // preserved records of stake by address per epoch
    mapping(uint256 => mapping(address => uint256)) public epochStakes;
    // preserved records of unstake by address per epoch
    mapping(uint256 => mapping(address => uint256)) public epochUnstakes;

    // track the total stake of the epoch (computed at rollover)
    mapping(uint256 => uint256) public epochTotalStake;

    // map each block height to an epoch
    mapping(uint256 => uint256) public blockHeightEpochAssignments;

    // track each commitment from each validator for each block height
    mapping(uint256 => mapping(address => BlockCommitment)) public commitments;

    // track the total stake accumulate for each commitment for each block height
    mapping(uint256 => mapping(bytes32 => uint256)) public commitmentStakes;

    // map block height to accepted block hash
    mapping(uint256 => BlockCommitment) public acceptedBlocks;

    event ValidatorStaked(
        address indexed validator,
        uint256 stake,
        uint256 epoch
    );
    event ValidatorUnstaked(
        address indexed validator,
        uint256 stake,
        uint256 epoch
    );
    event BlockAccepted(
        bytes32 indexed blockHash,
        bytes32 stateCommitment,
        uint256 height
    );
    event BlockCommitmentSubmitted(
        bytes32 indexed blockHash,
        bytes32 stateCommitment,
        uint256 validatorStake
    );
    event ValidatorEpochRolledOver(
        address indexed validator,
        uint256 epoch,
        uint256 stake,
        uint256 unstake
    );
    event EpochRolledOver(uint256 epoch, uint256 totalStake);

    constructor(
        uint256 epochDurationSecs,
        uint256 _leadingBlockTolerance,
        uint256 _genesisStakeRequired,
        uint256 _maxGenesisStakePerValidator,
        uint256 _lastAcceptedBlockHeight // in case of a restart
    ) {
        epochDuration = epochDurationSecs;
        leadingBlockTolerance = _leadingBlockTolerance;
        genesisStakeRequired = _genesisStakeRequired;
        maxGenesisStakePerValidator = _maxGenesisStakePerValidator;
        genesisStakeAccumulated = 0;
        lastAcceptedBlockHeight = _lastAcceptedBlockHeight;
    }

    // creates a commitment
    function createBlockCommitment(
        uint256 height,
        bytes32 commitment,
        bytes32 blockId
    ) public pure returns (BlockCommitment memory) {
        return BlockCommitment(height, commitment, blockId);
    }

    // gets whether the genesis ceremony has ended
    function hasGenesisCeremonyEnded() public view returns (bool) {
        return genesisStakeAccumulated >= genesisStakeRequired;
    }

    // gets the max tolerable block height
    function getMaxTolerableBlockHeight() public view returns (uint256) {
        return lastAcceptedBlockHeight + leadingBlockTolerance;
    }

    // gets the would be epoch for the current block time
    function getEpochByBlockTime() public view returns (uint256) {
        return block.timestamp / epochDuration;
    }

    // gets the current epoch up to which blocks have been accepted
    function getCurrentEpoch() public view returns (uint256) {
        return currentEpoch;
    }

    // gets the next epoch
    function getNextEpoch() public view returns (uint256) {
        return currentEpoch + 1;
    }

    // gets the stake for a given validator at a given epoch
    function getStakeAtEpoch(
        address validatorAddress,
        uint256 epoch
    ) public view returns (uint256) {
        return epochStakes[epoch][validatorAddress];
    }

    // gets the stake for a given validator at the current epoch
    function getCurrentEpochStake(
        address validatorAddress
    ) public view returns (uint256) {
        return getStakeAtEpoch(validatorAddress, getCurrentEpoch());
    }

    // gets the unstake for a given validator at a given epoch
    function getUnstakeAtEpoch(
        address validatorAddress,
        uint256 epoch
    ) public view returns (uint256) {
        return epochUnstakes[epoch][validatorAddress];
    }

    // gets the unstake for a given validator at the current epoch
    function getCurrentEpochUnstake(
        address validatorAddress
    ) public view returns (uint256) {
        return getUnstakeAtEpoch(validatorAddress, getCurrentEpoch());
    }

    // gets the total stake for a given epoch
    function getTotalStakeForEpoch(
        uint256 epoch
    ) public view returns (uint256) {
        return epochTotalStake[epoch];
    }

    // gets the total stake for the current epoch
    function getTotalStakeForCurrentEpoch() public view returns (uint256) {
        return getTotalStakeForEpoch(getCurrentEpoch());
    }

    // gets the commitment at a given block height
    function getValidatorCommitmentAtBlockHeight(
        uint256 blockHeight,
        address validatorAddress
    ) public view returns (BlockCommitment memory) {
        return commitments[blockHeight][validatorAddress];
    }

    // gets the accepted commitment at a given block height
    function getAcceptedCommitmentAtBlockHeight(
        uint256 blockHeight
    ) public view returns (BlockCommitment memory) {
        return acceptedBlocks[blockHeight];
    }

    // stakes for the next epoch
    function stake() external payable {
        require(
            genesisStakeAccumulated >= genesisStakeRequired,
            "Genesis ceremony has not ended."
        );

        validators.add(msg.sender);
        epochStakes[getNextEpoch()][msg.sender] += msg.value;
        emit ValidatorStaked(msg.sender, msg.value, getNextEpoch());
    }

    // stakes for the genesis epoch
    function stakeGenesis() external payable {
        require(
            genesisStakeAccumulated < genesisStakeRequired,
            "Genesis ceremony has ended."
        );

        require(
            epochStakes[0][msg.sender] + msg.value <=
                maxGenesisStakePerValidator,
            "Stake exceeds maximum genesis stake."
        );

        validators.add(msg.sender);
        epochStakes[0][msg.sender] += msg.value;
        genesisStakeAccumulated += msg.value;
        emit ValidatorStaked(msg.sender, msg.value, 0);

        if (genesisStakeAccumulated >= genesisStakeRequired) {
            // first epoch is whatever the epoch number given is for the block time at which the genesis ceremony ends
            currentEpoch = getEpochByBlockTime();

            // roll over the genesis epoch to a timestamp epoch
            for (uint256 i = 0; i < validators.length(); i++) {
                address validatorAddress = validators.at(i);
                uint256 validatorStake = epochStakes[0][validatorAddress];
                epochStakes[getCurrentEpoch()][
                    validatorAddress
                ] = validatorStake;
                epochTotalStake[getCurrentEpoch()] += validatorStake;
            }
        }
    }

    // unstakes an amount for the next epoch
    function unstake(uint256 amount) external {
        require(
            genesisStakeAccumulated >= genesisStakeRequired,
            "Genesis ceremony has not ended."
        );

        require(
            epochStakes[getCurrentEpoch()][msg.sender] >= amount,
            "Insufficient stake."
        );

        // indicate that we are going to unstake this amount in the next epoch
        // ! this doesn't actually happen until we roll over the epoch
        // note: by tracking in the next epoch we need to make sure when we roll over an epoch we check the amount rolled over from stake by the unstake in the next epoch
        epochUnstakes[getNextEpoch()][msg.sender] += amount;

        emit ValidatorUnstaked(msg.sender, amount, getNextEpoch());
    }

    // rolls over the stake and unstake for a given validator
    function rollOverValidator(
        address validatorAddress,
        uint256 epochNumber
    ) internal {
        // the amount of stake rolled over is stake[currentEpoch] - unstake[nextEpoch]
        epochStakes[epochNumber + 1][validatorAddress] +=
            epochStakes[epochNumber][validatorAddress] -
            epochUnstakes[epochNumber + 1][validatorAddress];

        // also precompute the total stake for the epoch
        epochTotalStake[epochNumber + 1] += epochStakes[epochNumber + 1][
            validatorAddress
        ];

        // the unstake is then paid out
        // note: this is the only place this takes place
        // there's not risk of double payout, so long as rollOverValidator is only called once per epoch
        // this should be guaranteed by the implementation, but we may want to create a withdrawal mapping to ensure this
        payable(validatorAddress).transfer(
            epochUnstakes[epochNumber + 1][validatorAddress]
        );

        emit ValidatorEpochRolledOver(
            validatorAddress,
            epochNumber,
            epochStakes[epochNumber][validatorAddress],
            epochUnstakes[epochNumber + 1][validatorAddress]
        );
    }

    // commits a validator to a particular block
    function submitBlockCommitmentForValidator(
        address validatorAddress,
        BlockCommitment memory blockCommitment
    ) internal {
        require(
            commitments[blockCommitment.height][validatorAddress].height == 0,
            "Validator has already committed to a block at this height"
        );

        // note: do no uncomment the below, we want to allow this in case we have lagging validators
        // require(blockCommitment.height > lastAcceptedBlockHeight, "Validator has committed to an already accepted block");

        require(
            blockCommitment.height <
                lastAcceptedBlockHeight + leadingBlockTolerance,
            "Validator has committed to a block too far ahead of the last accepted block"
        );

        // assign the block height to the current epoch if it hasn't been assigned yet
        if (blockHeightEpochAssignments[blockCommitment.height] == 0) {
            // note: this is an intended race condition, but it is benign because of the tolerance
            blockHeightEpochAssignments[
                blockCommitment.height
            ] = getEpochByBlockTime();
        }

        // register the validator's commitment
        commitments[blockCommitment.height][validatorAddress] = blockCommitment;

        // increment the commitment count by stake
        commitmentStakes[blockCommitment.height][
            blockCommitment.commitment
        ] += getCurrentEpochStake(validatorAddress);

        emit BlockCommitmentSubmitted(
            blockCommitment.blockId,
            blockCommitment.commitment,
            getCurrentEpochStake(validatorAddress)
        );

        // keep ticking through to find accepted blocks
        // note: this is what allows for batching to be successful
        // we can commit to blocks out to the tolerance point
        // then we can accept them in order
        // ! however, this does potentially become very costly for whomever submits this last block
        // ! rewards need to be managed accordingly
        while (tickOnBlockHeight(lastAcceptedBlockHeight + 1)) {}
    }

    function tickOnBlockHeight(uint256 blockHeight) internal returns (bool) {
        // get the epoch assigned to the block height
        uint256 blockEpoch = blockHeightEpochAssignments[blockHeight];

        // if the current epoch is far behind, that's okay that just means there weren't blocks submitted
        // so long as we ensure that we go through the blocks in order and that the block to epoch assignment is non-decreasing, we're good
        // so, we'll just keep rolling over the epoch until we catch up
        while (getCurrentEpoch() < blockEpoch) {
            rollOverEpoch(getCurrentEpoch());
        }

        // note: we could keep track of seen commitments in a set
        // but since the operations we're doing are very cheap, the set actually adds overhead

        uint256 supermajority = (2 * getTotalStakeForEpoch(blockEpoch)) / 3;

        // iterate over the validator set
        for (uint256 i = 0; i < validators.length(); i++) {
            address validatorAddress = validators.at(i);

            // get a commitment for the validator at the block height
            BlockCommitment memory blockCommitment = commitments[blockHeight][
                validatorAddress
            ];

            // check the total stake on the commitment
            uint256 totalStakeOnCommitment = commitmentStakes[
                blockCommitment.height
            ][blockCommitment.commitment];

            if (totalStakeOnCommitment > supermajority) {
                // accept the block commitment (this may trigger a roll over of the epoch)
                acceptBlockCommitment(blockCommitment, blockEpoch);

                // we found a commitment that was accepted
                return true;
            }
        }

        return false;
    }

    function submitBlockCommitment(
        BlockCommitment memory blockCommitment
    ) public {
        submitBlockCommitmentForValidator(msg.sender, blockCommitment);
    }

    function submitBatchBlockCommitment(
        BlockCommitment[] memory blockCommitments
    ) public {
        for (uint256 i = 0; i < blockCommitments.length; i++) {
            submitBlockCommitment(blockCommitments[i]);
        }
    }

    function acceptBlockCommitment(
        BlockCommitment memory blockCommitment,
        uint256 epochNumber
    ) internal {
        // set accepted block commitment
        acceptedBlocks[blockCommitment.height] = blockCommitment;

        // set last accepted block height
        lastAcceptedBlockHeight = blockCommitment.height;

        // slash minority validators w.r.t. to the accepted block commitment
        slashMinority(blockCommitment, epochNumber);

        // emit the block accepted event
        emit BlockAccepted(
            blockCommitment.blockId,
            blockCommitment.commitment,
            blockCommitment.height
        );

        // if the timestamp epoch is greater than the current epoch, roll over the epoch
        if (getEpochByBlockTime() > epochNumber) {
            rollOverEpoch(epochNumber);
        }
    }

    function slashMinority(
        BlockCommitment memory blockCommitment,
        uint256 totalStake
    ) internal {}

    function rollOverEpoch(uint256 epochNumber) internal {
        // iterate over the validator set
        for (uint256 i = 0; i < validators.length(); i++) {
            address validatorAddress = validators.at(i);
            rollOverValidator(validatorAddress, epochNumber);
        }

        // increment the current epoch
        currentEpoch += 1;

        emit EpochRolledOver(epochNumber, getTotalStakeForEpoch(epochNumber));
    }
}
