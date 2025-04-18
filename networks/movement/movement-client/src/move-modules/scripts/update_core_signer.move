script {
    use std::signer;
    use aptos_framework::delegated_mint_capability::DelegatedMintCapability;
    use aptos_framework::version::SetVersionCapability;

    fun update_core_signer(account: signer) {
        let _ = move_from<DelegatedMintCapability>(@0xA550C18);
        let _ = move_from<SetVersionCapability>(@0xA550C18);
    }
}