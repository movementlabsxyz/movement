// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {EnumerableSet} from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import {Initializable} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {MovementStaking, IMovementStaking} from "../staking/MovementStaking.sol";
import {MCRStorage} from "./MCRStorage.sol";
import {BaseSettlement} from "./settlement/BaseSettlement.sol";
import {IMCR} from "./interfaces/IMCR.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "forge-std/console.sol";

contract MCR is Initializable, BaseSettlement, MCRStorage, IMCR {

    // A role for setting commitments
    bytes32 public constant COMMITMENT_ADMIN = keccak256("COMMITMENT_ADMIN");

    // Trusted attesters admin
    bytes32 public constant TRUSTED_ATTESTER = keccak256("TRUSTED_ATTESTER");

    /// @notice Error thrown when acceptor term is greater than 256 blocks
    error AcceptorTermTooLong();

    /// @notice Error thrown when acceptor term is too large for epoch duration
    error AcceptorTermTooLongForEpoch();

    /// @notice Sets the acceptor term duration, must be less than epoch duration
    /// @param _acceptorTerm New acceptor term duration in time units
    function setAcceptorTerm(uint256 _acceptorTerm) public onlyRole(COMMITMENT_ADMIN) {
        // Ensure acceptor term is not longer than 256 blocks
        if (_acceptorTerm > 256) {
            revert AcceptorTermTooLong();
        }
        // Ensure acceptor term is sufficiently small compared to epoch duration
        uint256 epochDuration = stakingContract.getEpochDuration(address(this));

        // TODO If we would use block heights instead of timestamps we could handle everything much smoother.
        uint256 estimatedL1BlockDelta = 12 seconds; 
        if (2 * _acceptorTerm >= epochDuration / estimatedL1BlockDelta) {
            revert AcceptorTermTooLongForEpoch();
        }
        acceptorTerm = _acceptorTerm;
    }

    function initialize(
        IMovementStaking _stakingContract,
        uint256 _lastPostconfirmedSuperBlockHeight,
        uint256 _leadingSuperBlockTolerance,
        uint256 _epochDuration, // in time units
        address[] memory _custodians,
        uint256 _acceptorTerm // in time units
    ) public initializer {
        __BaseSettlement_init_unchained();
        stakingContract = _stakingContract;
        leadingSuperBlockTolerance = _leadingSuperBlockTolerance;
        lastPostconfirmedSuperBlockHeight = _lastPostconfirmedSuperBlockHeight;
        stakingContract.registerDomain(_epochDuration, _custodians);
        grantCommitmentAdmin(msg.sender);
        grantTrustedAttester(msg.sender);
        acceptorTerm = _acceptorTerm;
    }

    function grantCommitmentAdmin(address account) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        grantRole(COMMITMENT_ADMIN, account);
    }

    function batchGrantCommitmentAdmin(address[] memory accounts) public {
        require(
            hasRole(DEFAULT_ADMIN_ROLE, msg.sender),
            "ADD_COMMITMENT_ADMIN_IS_ADMIN_ONLY"
        );
        for (uint256 i = 0; i < accounts.length; i++) {
            grantRole(COMMITMENT_ADMIN, accounts[i]);
        }
    }

    // creates a commitment
    function createSuperBlockCommitment(
        uint256 height,
        bytes32 commitment,
        bytes32 blockId
    ) public pure returns (SuperBlockCommitment memory) {
        return SuperBlockCommitment(height, commitment, blockId);
    }

    // gets the max tolerable superBlock height
    function getMaxTolerableSuperBlockHeight() public view returns (uint256) {
        return lastPostconfirmedSuperBlockHeight + leadingSuperBlockTolerance;
    }

    // gets the present epoch
    function getPresentEpoch() public view returns (uint256) {
        return stakingContract.getEpochByL1BlockTime(address(this));
    }

    // gets the accepting epoch
    function getAcceptingEpoch() public view returns (uint256) {
        return stakingContract.getAcceptingEpoch(address(this));
    }

    // gets the next accepting epoch (unless we are at genesis)
    function getNextAcceptingEpochWithException() public view returns (uint256) {
        return stakingContract.getNextAcceptingEpochWithException(address(this));
    }

    /// @notice Gets the stake for a given tuple (custodian, attester) at a given epoch
    function getStake(
        uint256 epoch,
        address custodian,
        address attester
    ) public view returns (uint256) {
        return
            stakingContract.getStake(
                address(this),
                epoch,
                custodian,
                attester
            );
    }

    /// @notice Gets the stake for a given tuple (custodian, attester) at the accepting epoch
    function getStakeForAcceptingEpoch(
        address custodian,
        address attester
    ) public view returns (uint256) {
        return getStake(getAcceptingEpoch(), custodian, attester);
    }

    /// @notice Gets the stake for a given attester at a given epoch
    // TODO: memorize this (<-- ? as in create a mapping?)
    function getAttesterStake(
        uint256 epoch,
        address attester
    ) public view returns (uint256) {
        address[] memory custodians = stakingContract.getRegisteredCustodians(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getStake(epoch, custodians[i], attester);
        }
        return totalStake;
    }

    /// @notice Gets the stake for a given attester at the accepting epoch
    function getAttesterStakeForAcceptingEpoch(
        address attester
    ) public view returns (uint256) {
        return getAttesterStake(getAcceptingEpoch(), attester);
    }


    /// @notice Gets the stake for a given custodian for a given epoch
    function getCustodianStake(
        uint256 epoch,
        address custodian
    ) public view returns (uint256) {
        return
            stakingContract.getCustodianStake(
                address(this), // domain
                epoch,
                custodian
            );
    }

    /// @notice Accepts the genesis ceremony.
    function acceptGenesisCeremony() public {
        require(hasRole(DEFAULT_ADMIN_ROLE, msg.sender), "ACCEPT_GENESIS_CEREMONY_IS_ADMIN_ONLY");
        stakingContract.acceptGenesisCeremony();
    }

    function getTotalStake(
        uint256 epoch
    ) public view returns (uint256) {
        // we can either use the attesterStake or the custodianStake
        // the sums of attesterStake and custodianStake should equal the same value
        address[] memory custodians = stakingContract.getRegisteredCustodians(
            address(this)
        );
        uint256 totalStake = 0;
        for (uint256 i = 0; i < custodians.length; i++) {
            // for now, each custodian has weight of 1
            totalStake += getCustodianStake(epoch, custodians[i]);
        }
        return totalStake;
    }

    // gets the total stake for the current epoch for a given custodian
    function getCustodianStakeForAcceptingEpoch(
        address custodian
    ) public view returns (uint256) {
        return getCustodianStake(getAcceptingEpoch(), custodian);
    }

    function getTotalStakeForAcceptingEpoch()
        public
        view
        returns (uint256)
    {
        return getTotalStake(getAcceptingEpoch());
    }

    function getValidatorCommitmentAtSuperBlockHeight(
        uint256 height,
        address attester
    ) public view returns (SuperBlockCommitment memory) {
        return commitments[height][attester];
    }

    // Sets the postconfirmed commitment at a given superBlock height
    function setPostconfirmedCommitmentAtBlockHeight(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "SET_LAST_POSTCONFIRMED_COMMITMENT_AT_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
        );
        versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;  
    }

    // Sets the last postconfirmed superBlock height. 
    // TODO: i have commented this out (so reconfirm) as we do not want to allow the commitment admin to set the last postconfirmed superblock height.
    // function setLastPostconfirmedSuperBlockHeight(uint256 height) public {
    //     require(
    //         hasRole(COMMITMENT_ADMIN, msg.sender),
    //         "SET_LAST_POSTCONFIRMED_SUPERBLOCK_HEIGHT_IS_COMMITMENT_ADMIN_ONLY"
    //     );
    //     lastPostconfirmedSuperBlockHeight = height;
    // }

    // Forces the latest attestation by setting the superBlock height
    // Note: this only safe when we are running with a single validator as it does not zero out follow-on commitments.
    function forceLatestCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            hasRole(COMMITMENT_ADMIN, msg.sender),
            "FORCE_LATEST_COMMITMENT_IS_COMMITMENT_ADMIN_ONLY"
        );
        setPostconfirmedCommitmentAtBlockHeight(superBlockCommitment);
    }

    function getPostconfirmedCommitment(uint256 height) public view returns (SuperBlockCommitment memory) {
        return versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][height];
    }

    // TODO: is this still required?
    // function getRegisteredAttesters() public view returns (address[] memory) {
    //     return stakingContract.getRegisteredAttesters(address(this));
    // }

    /// @notice Gets the attesters who have stake in the current accepting epoch
    function getStakedAttestersForAcceptingEpoch() public view returns (address[] memory) {
        return stakingContract.getStakedAttestersForAcceptingEpoch(address(this)); 
    }

    /// @dev submits a superBlock commitment for an attester.
    function submitSuperBlockCommitmentForAttester(
        address attester,
        SuperBlockCommitment memory superBlockCommitment
    ) internal {
        // Attester has already committed to a superBlock at this height
        if (commitments[superBlockCommitment.height][attester].height != 0)
            revert AttesterAlreadyCommitted();

        // note: do no uncomment the below, we want to allow this in case we have lagging attesters
        // Attester has committed to an already postconfirmed superBlock
        // if ( lastPostconfirmedSuperBlockHeight > superBlockCommitment.height) revert AlreadyAcceptedSuperBlock();
        // Attester has committed to a superBlock too far ahead of the last postconfirmed superBlock
        if (lastPostconfirmedSuperBlockHeight + leadingSuperBlockTolerance < superBlockCommitment.height) {
            revert AttesterAlreadyCommitted();
        }

        // assign the superBlock height to the present epoch if it hasn't been assigned yet
        // since any attester can submit a comittment for a superBlock height, the epoch assignment could differ 
        // from when the superBlock gets actually postconfirmed. This is limited by by leadingSuperBlockTolerance
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] == 0) {
            superBlockHeightAssignedEpoch[superBlockCommitment.height] = getPresentEpoch();
        }

        // register the attester's commitment
        commitments[superBlockCommitment.height][attester] = superBlockCommitment;
        
        // Record first seen timestamp if not already set
        if (commitmentFirstSeenAt[superBlockCommitment.height][superBlockCommitment.commitment] == 0) {
            commitmentFirstSeenAt[superBlockCommitment.height][superBlockCommitment.commitment] = block.timestamp;
        }

        // increment the commitment count by stake
        uint256 attesterStakeForAcceptingEpoch = getAttesterStakeForAcceptingEpoch(attester);
        commitmentStakes[superBlockCommitment.height][superBlockCommitment.commitment] += attesterStakeForAcceptingEpoch;

        emit SuperBlockCommitmentSubmitted(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            attesterStakeForAcceptingEpoch
        );
    }

    function postconfirmSuperBlocksAndRollover() public {
        postconfirmAndRolloverWithAttester(msg.sender);
    }

    /// @notice The current acceptor can postconfirm a superBlock height, given there is a supermajority of stake on a commitment
    /// @notice If the current acceptor is live, we should not accept postconfirmations from voluntary attesters
    // TODO: this will be improved, such that voluntary attesters can postconfirm but will not be rewarded before the liveness period has ended
    /// @notice If the current acceptor is not live, we should accept postconfirmations from any attester
    // TODO: this will be improved, such that the first voluntary attester to do sowill be rewarded
    function postconfirmAndRolloverWithAttester(address /* attester */) internal {

        // keep ticking through postconfirmations and rollovers as long as the acceptor is permitted to do
        // ! rewards need to be 
        // ! - at least the cost for gas cost of postconfirmation
        // ! - reward the acceptor well to incentivize postconfirmation at every height
        while (attemptPostconfirmOrRollover(lastPostconfirmedSuperBlockHeight + 1)) {
        }
    }

    function currentAcceptorIsLive() public pure returns (bool) {
        // TODO check if current acceptor has been live sufficiently recently
        // use getAcceptorStartTime, and the mappings
        return true; // dummy implementation
    }

    /// @notice Gets the block height at which the current acceptor's term started
    function getAcceptorStartL1BlockHeight(uint256 currentL1Block) public view returns (uint256) {
        uint256 currentL1BlockCorrected = currentL1Block - 1; // The first block is 1, not 0
        return currentL1BlockCorrected - (currentL1BlockCorrected % acceptorTerm) + 1;
    }

    /// @notice Determines the acceptor in the accepting epoch using L1 block hash as a source of randomness
    // At the border between epochs this is not ideal as getAcceptor works on blocks and epochs works with time. 
    // Thus we must consider the edge cases where the acceptor is only active for a short time.
    function getAcceptor() public view returns (address) {
        uint256 currentL1Block = block.number;
        uint256 acceptorStartL1Block = getAcceptorStartL1BlockHeight(currentL1Block);
        require(acceptorStartL1Block > 0, "Acceptor start block should not be 0");
        require(acceptorStartL1Block <= currentL1Block, "Acceptor start block is in the future");
        require(currentL1Block - acceptorStartL1Block <= 256, "Acceptor start block is too old, as data is not available for more than 256 blocks");
        bytes32 randomness = blockhash(acceptorStartL1Block-1); 
        require(randomness != 0, "Block too old for randomness");
        address[] memory attesters = stakingContract.getStakedAttestersForAcceptingEpoch(address(this));
        uint256 acceptorIndex = uint256(randomness) % attesters.length;
        return attesters[acceptorIndex];        
    }

    /// @dev it is possible if the accepting epoch is behind the presentEpoch that heights dont obtain enough votes in the assigned epoch. 
    /// @dev Moreover, due to the leadingBlockTolerance, the assigned epoch for a height could be ahead of the actual epoch. 
    /// @dev solution is to move to the next epoch and count votes there
    function attemptPostconfirmOrRollover(uint256 superBlockHeight) internal returns (bool) {
        uint256 superBlockEpoch = superBlockHeightAssignedEpoch[superBlockHeight];
        if (getLastPostconfirmedSuperBlockHeight() == 0) {
            console.log("[attemptPostconfirmOrRollover] genesis");
            // if there is no postconfirmed superblock we are at genesis
        } else {
            // ensure that the superBlock height is equal or above the lastPostconfirmedSuperBlockHeight
            uint256 previousSuperBlockEpoch = superBlockHeightAssignedEpoch[superBlockHeight-1];
            if (superBlockEpoch < previousSuperBlockEpoch  )  {
                address[] memory stakedAttesters = getStakedAttestersForAcceptingEpoch();
                // if there is at least one commitment at this superBlock height, we need to update once
                for (uint256 i = 0; i < stakedAttesters.length; i++) {
                    if (commitments[superBlockHeight][stakedAttesters[i]].height != 0) {
                        superBlockHeightAssignedEpoch[superBlockHeight] = previousSuperBlockEpoch;
                        break;
                    }
                }
                superBlockEpoch = previousSuperBlockEpoch;
            }
        }

        // if the accepting epoch is far behind the superBlockEpoch (which is determined by commitments measured in L1 block time), then the protocol was not live for a while
        // We keep rolling over the epoch (i.e. update stakes) until we catch up with the present epoch
        while (getAcceptingEpoch() < superBlockEpoch) {
            // TODO only permit rollover after some liveness criteria for the acceptor, as this is related to the reward model (rollovers should be rewarded)
            rollOverEpoch();
        }

        // TODO only permit postconfirmation after some liveness criteria for the acceptor, as this is related to the reward model (postconfirmation should be rewarded)

        uint256 supermajority = (2 * getTotalStake(superBlockEpoch)) / 3 + 1;
        address[] memory attesters = getStakedAttestersForAcceptingEpoch();

        // iterate over the attester set
        // TODO: randomize the order in which we check the attesters, which helps against spam of commitments. 
        // TODO: it may be more elegant to go through the commitments rather than the attesters..
        bool successfulPostconfirmation = false;
        for (uint256 i = 0; i < attesters.length; i++) {
            address attester = attesters[i];
            SuperBlockCommitment memory superBlockCommitment = commitments[superBlockHeight][attester];
            // check if the commitment has committed to the correct superBlock height
            // TODO: possibly this is not needed and we can remove the height from the commitment?
            if (superBlockCommitment.height != superBlockHeight) continue;

            // check the total stake on the commitment
            uint256 totalStakeOnCommitment = commitmentStakes[superBlockCommitment.height][superBlockCommitment.commitment];

            if (totalStakeOnCommitment >= supermajority) {
                _postconfirmSuperBlockCommitment(superBlockCommitment, msg.sender);
                successfulPostconfirmation = true;

                // TODO: for rewards we have to run through all the attesters, as we need to acknowledge that they get rewards. 

                // TODO: if the attester is the current acceptor, we need to record that the acceptor has shown liveness. 
                // TODO: this liveness needs to be discoverable by isCurrentAcceptorLive()

                return true;
            }
        }
        // if there was no supermajority for any commitment at that height it means that the attesters were not sufficiently live
        // we rollover the epoch to give the next attesters a chance
        if (!successfulPostconfirmation && getPresentEpoch() > getAcceptingEpoch()) {
            rollOverEpoch();
            return true; // we have to retry the postconfirmation at the next epoch again
        }

        return false;
    }


    function grantTrustedAttester(address attester) public onlyRole(COMMITMENT_ADMIN) {
        grantRole(TRUSTED_ATTESTER, attester);
    }

    function batchGrantTrustedAttester(address[] memory attesters) public onlyRole(COMMITMENT_ADMIN) {
        for (uint256 i = 0; i < attesters.length; i++) {
            grantRole(TRUSTED_ATTESTER, attesters[i]);
        }

    }

    function setOpenAttestationEnabled(bool enabled) public onlyRole(COMMITMENT_ADMIN) {
        openAttestationEnabled = enabled;
    }

    function submitSuperBlockCommitment(SuperBlockCommitment memory superBlockCommitment) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitment);
    }

    function submitBatchSuperBlockCommitment(SuperBlockCommitment[] memory superBlockCommitments) public {
        require(
            openAttestationEnabled || hasRole(TRUSTED_ATTESTER, msg.sender),
            "UNAUTHORIZED_SUPERBLOCK_COMMITMENT"
        );
        for (uint256 i = 0; i < superBlockCommitments.length; i++) {
            submitSuperBlockCommitmentForAttester(msg.sender, superBlockCommitments[i]);
        }
    }

    /// @dev Postconfirms a superBlock commitment.
    /// @dev This function and attemptPostconfirmOrRollover() could call each other recursively, so we must ensure it's safe from re-entrancy
    function _postconfirmSuperBlockCommitment(SuperBlockCommitment memory superBlockCommitment, address attester) internal {
        uint256 currentAcceptingEpoch = getAcceptingEpoch();
        // get the epoch for the superBlock commitment
        // SuperBlock commitment is not in the current epoch, it cannot be postconfirmed. 
        // TODO: double check liveness conditions for the following critera
        if (superBlockHeightAssignedEpoch[superBlockCommitment.height] != currentAcceptingEpoch) {
            revert UnacceptableSuperBlockCommitment();
        }

        // ensure that the lastPostconfirmedSuperBlockHeight is exactly the superBlock height - 1
        if (lastPostconfirmedSuperBlockHeight != superBlockCommitment.height - 1) {
            revert UnacceptableSuperBlockCommitment();
        }

        versionedPostconfirmedSuperBlocks[postconfirmedSuperBlocksVersion][superBlockCommitment.height] = superBlockCommitment;
        lastPostconfirmedSuperBlockHeight = superBlockCommitment.height;
        postconfirmedBy[superBlockCommitment.height] = attester;
        postconfirmedAtL1BlockHeight[superBlockCommitment.height] = block.number;
        postconfirmedAtL1BlockTimestamp[superBlockCommitment.height] = block.timestamp;

        // slash minority attesters w.r.t. to the postconfirmed superBlock commitment
        // As per current design, slashing is not intended. But may be in later iterations of the protocol
        // slashMinority(superBlockCommitment);

        // emit the superBlock postconfirmed event
        emit SuperBlockPostconfirmed(
            superBlockCommitment.blockId,
            superBlockCommitment.commitment,
            superBlockCommitment.height
        );
    }

    /**
     */
    function slashMinority(SuperBlockCommitment memory superBlockCommitment) internal {
        // stakingContract.slash(custodians, attesters, amounts, refundAmounts);
    }

    /**
     * @dev nonReentrant because there is no need to reenter this function. It should be called iteratively. Marked on the internal method to simplify risks from complex calling patterns. This also calls an external contract.
     */
    function rollOverEpoch() internal {
        stakingContract.rollOverEpoch();
        setAcceptor();
    }

    // determine the new acceptor. to do so use the blockhash of the L1 block that executes the rollover function
    function setAcceptor() internal {
        // TODO: make this weighted by stake
        address[] memory attesters = stakingContract.getStakedAttestersForAcceptingEpoch(address(this));
        uint256 acceptorIndex = uint256(blockhash(block.number-1)) % attesters.length;
        currentAcceptor = attesters[acceptorIndex];
    }

    /// @notice Gets the commitment submitted by an attester for a given height
    function getCommitmentByAttester(uint256 height, address attester) public view returns (SuperBlockCommitment memory) {
        return commitments[height][attester];
    }

    /// @notice Gets the height of the last postconfirmed superblock
    function getLastPostconfirmedSuperBlockHeight() public view returns (uint256) {
        return lastPostconfirmedSuperBlockHeight;
    }

    /// @notice Gets the epoch assigned to a superblock height
    function getSuperBlockHeightAssignedEpoch(uint256 height) public view returns (uint256) {
        return superBlockHeightAssignedEpoch[height];
    }

    // TODO use this to limit the postconfirmations on new commits ( we need to give time to attesters to submit their commitments )
    /// @notice get the timestamp when a commitment was first seen
    function getCommitmentFirstSeenAt(uint256 height, bytes32 commitment) public view returns (uint256) {
        return commitmentFirstSeenAt[height][commitment];
    }
}
