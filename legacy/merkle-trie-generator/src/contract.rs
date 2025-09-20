use alloy_sol_types::sol;
use alloy_primitives::{Bytes, B256, Address, U256};
use alloy_providers::ProviderBuilder;
use alloy_signers::wallet::LocalWallet;
use alloy_transport::Transport;
use alloy_rpc_types::transaction::eip2718::TypedTransaction;
use alloy_providers::Middleware;

use std::sync::Arc;

sol! {
    contract AirdropStorage {
        function updateTrie(bytes32 root, bytes trieData);
        function currentRoot() view returns (bytes32);
    }
}

pub async fn send_trie_update(
    rpc_url: &str,
    private_key_hex: &str,
    contract_address: Address,
    root: B256,
    trie_data: Vec<u8>,
) -> anyhow::Result<()> {
    let wallet: LocalWallet = private_key_hex.parse()?;
    let address = wallet.address();

    let provider = ProviderBuilder::new().on_ws(rpc_url).await?;
    let client = Arc::new(provider.with_signer(wallet));

    // Optional: validate current root from chain
    let view_call = AirdropStorage::currentRootCall {};
    let encoded = view_call.abi_encode();
    let result = client
        .call(&TypedTransaction::Legacy {
            to: Some(contract_address),
            data: Some(Bytes::from(encoded)),
            ..Default::default()
        }, None)
        .await?;

    let onchain_root = B256::from_slice(&result[..32]);
    if onchain_root == root {
        println!("Root already on-chain, skipping update.");
        return Ok(());
    }

    // Prepare contract calldata
    let update_call = AirdropStorage::updateTrieCall {
        root,
        trieData: Bytes::from(trie_data),
    };
    let calldata = update_call.abi_encode();

    let gas_price = client.get_gas_price().await?;
    let nonce = client.get_transaction_count(address, None).await?;
    let gas_limit = U256::from(1_000_000); // We can estimate gas limit instead

    let tx = TypedTransaction::Legacy {
        to: Some(contract_address),
        data: Some(Bytes::from(calldata)),
        nonce: Some(nonce),
        gas_price: Some(gas_price),
        gas: Some(gas_limit),
        value: Some(U256::ZERO),
        ..Default::default()
    };

    let pending = client.send_transaction(tx, None).await?;
    let receipt = pending.await?;
    println!("Transaction sent: {:?}", receipt.transaction_hash);

    Ok(())
}
