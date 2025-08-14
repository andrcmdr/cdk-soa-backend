use alloy_sol_types::sol;
use alloy_primitives::{Bytes, B256};

sol! {
    contract AirdropStorage {
        function updateTrie(bytes32 root, bytes trieData);
    }
}

pub fn encode_contract_call(root: B256, trie_data: Vec<u8>) -> Bytes {
    let call = AirdropStorage::updateTrieCall {
        root,
        trieData: Bytes::from(trie_data),
    };
    call.abi_encode()
}
