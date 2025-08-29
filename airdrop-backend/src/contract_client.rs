use anyhow::Result;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::{ProviderBuilder, RootProvider},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
    contract::ContractInstance,
};
use alloy_provider::Provider;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    AirdropContract,
    "abi/AirdropContract.json"
);

pub struct ContractClient {
    provider: RootProvider<Http<Client>>,
    contract: ContractInstance<Http<Client>, RootProvider<Http<Client>>, AirdropContract::AirdropContractInstance>,
    wallet: EthereumWallet,
}

impl ContractClient {
    pub async fn new(
        rpc_url: &str,
        contract_address: Address,
        private_key: &str,
        chain_id: u64,
    ) -> Result<Self> {
        let signer: PrivateKeySigner = private_key.parse()?;
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet.clone())
            .on_http(rpc_url.parse()?);

        let contract = AirdropContract::new(contract_address, &provider);

        Ok(Self {
            provider,
            contract,
            wallet,
        })
    }

    pub async fn is_root_hash_exists(&self, root_hash: B256) -> Result<bool> {
        let call = self.contract.isRootHashExists(root_hash);
        let result = call.call().await?;
        Ok(result._0)
    }

    pub async fn submit_trie_update(&self, round_id: u32, root_hash: B256, trie_data: Vec<u8>) -> Result<B256> {
        tracing::info!(
            "Submitting trie update for round {} with root hash: 0x{}",
            round_id, 
            hex::encode(root_hash)
        );

        let call = self.contract.updateTrieRoot(U256::from(round_id), root_hash, trie_data.into());
        let pending_tx = call.send().await?;
        let receipt = pending_tx.get_receipt().await?;

        tracing::info!("Transaction submitted: 0x{}", hex::encode(receipt.transaction_hash));

        Ok(receipt.transaction_hash)
    }

    pub async fn get_trie_root(&self, round_id: u32) -> Result<B256> {
        let call = self.contract.getTrieRoot(U256::from(round_id));
        let result = call.call().await?;
        Ok(result._0)
    }

    pub async fn verify_eligibility(
        &self,
        round_id: u32,
        address: Address,
        amount: U256,
        proof: Vec<Vec<u8>>
    ) -> Result<bool> {
        let proof_bytes: Vec<_> = proof.into_iter().map(|p| p.into()).collect();

        let call = self.contract.verifyEligibility(
            U256::from(round_id),
            address,
            amount,
            proof_bytes
        );
        let result = call.call().await?;
        Ok(result._0)
    }
}
