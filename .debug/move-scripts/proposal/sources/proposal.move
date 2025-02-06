// Consensus config upgrade proposal

// config: V3 {
//     alg: JolteonV2 {
//         main: ConsensusConfigV1 {
//             decoupled_execution: true,
//             back_pressure_limit: 10,
//             exclude_round: 40,
//             proposer_election_type: LeaderReputation(
//                 ProposerAndVoterV2(
//                     ProposerAndVoterConfig {
//                         active_weight: 1000,
//                         inactive_weight: 10,
//                         failed_weight: 1,
//                         failure_threshold_percent: 10,
//                         proposer_window_num_validators_multiplier: 10,
//                         voter_window_num_validators_multiplier: 1,
//                         weight_by_voting_power: true,
//                         use_history_from_previous_epoch_max_count: 5,
//                     },
//                 ),
//             ),
//             max_failed_authors_to_store: 10,
//         },
//         quorum_store_enabled: true,
//         order_vote_enabled: false,
//     },
//     vtxn: V0,
// }

script {
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;

    fun main(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let framework_signer = &core_signer;

        let consensus_blob: vector<u8> = x"0202010a0000000000000028000000000000000201e8030000000000000a0000000000000001000000000000000a0000000a00000000000000010000000000000001050000000a00000000000000010000";

        consensus_config::set_for_next_epoch(framework_signer, consensus_blob);
        aptos_governance::reconfigure(framework_signer);
    }
}
