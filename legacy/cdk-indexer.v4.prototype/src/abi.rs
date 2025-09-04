use alloy::primitives::{hex, Address, B256, Bytes};
use alloy::rpc::types::eth::Log;
use alloy::dyn_abi::{DynSolType, DynSolValue};
use alloy::json_abi as json_abi;
use json_abi::{Event, JsonAbi};
use serde_json::{json, Value};
use std::{collections::HashMap, fs};

#[derive(Debug, Clone)]
pub struct ContractAbi {
    pub name: String,
    pub address: Address,
    pub abi: JsonAbi,
    // Map first-topic (event signature hash) -> Event definition
    events_by_topic: HashMap<B256, Event>,
}

impl ContractAbi {
    pub fn load(name: String, address: Address, abi_path: &str) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(abi_path)?;
        let abi: JsonAbi = serde_json::from_str(&raw)?;
        let mut events_by_topic = HashMap::new();
        for (_name, events) in &abi.events {
            for ev in events {
                if !ev.anonymous {
                    events_by_topic.insert(ev.selector(), ev.clone());
                }
            }
        }
        Ok(Self { name, address, abi, events_by_topic })
    }

    /// Returns the event hash (keccak256 of the event signature) for the first topic.
    pub fn event_hash(&self, topic0: &B256) -> Option<B256> {
        self.events_by_topic.get(topic0).map(|ev| ev.selector())
    }

    /// Returns the event name (e.g., "Transfer") for the first topic.
    pub fn event_name(&self, topic0: &B256) -> Option<String> {
        self.events_by_topic.get(topic0).map(|ev| ev.name.clone())
    }

    /// Decode a log according to this contract's ABI.
    /// Returns (event_name, event_hash, decoded_params_json) if this ABI can decode the log.
    pub fn decode_log(&self, log: &Log) -> Option<(String, B256, Value)> {
        let topic0 = log.topics.first()?;
        let ev = self.events_by_topic.get(topic0)?;
        let event_hash = ev.selector();

        // Build ordered params split by indexed/non-indexed
        let mut indexed = Vec::new();
        let mut non_indexed_types = Vec::new();
        let mut non_indexed_names = Vec::new();
        for p in &ev.inputs {
            if p.indexed {
                indexed.push(p);
            } else {
                // p.ty is a parsed-ish string; use DynSolType::parse
                let ty = DynSolType::parse(&p.ty.to_string()).ok()?;
                non_indexed_types.push(ty);
                non_indexed_names.push(p.name.clone());
            }
        }

        // Decode non-indexed from data
        let mut params = serde_json::Map::new();
        if !non_indexed_types.is_empty() {
            let data_bytes: Bytes = log.data.clone().unwrap_or_default();
            let decoded = DynSolValue::decode_params(&non_indexed_types, &data_bytes).ok()?;
            for (name, val) in non_indexed_names.into_iter().zip(decoded) {
                params.insert(name, dyn_value_to_json(val));
            }
        }

        // Decode indexed from topics[1..]
        for (i, p) in indexed.iter().enumerate() {
            let key = if p.name.is_empty() { format!("indexed_{i}") } else { p.name.clone() };
            // For indexed dynamic types Solidity stores keccak256(value), so we can't decode the original value
            // Here we parse type; if it's static, decode from topic directly, else keep hex
            let ty = DynSolType::parse(&p.ty.to_string()).ok()?;
            if ty.is_dynamic() {
                if let Some(topic) = log.topics.get(i + 1) {
                    params.insert(key, json!({ "hash": format!("0x{}", hex::encode(topic)) }));
                }
            } else {
                if let Some(topic) = log.topics.get(i + 1) {
                    let bytes = Bytes::copy_from_slice(topic.as_slice());
                    if let Ok(val) = DynSolValue::decode(&ty, &bytes) {
                        params.insert(key, dyn_value_to_json(val));
                    } else {
                        params.insert(key, json!({ "raw": format!("0x{}", hex::encode(bytes)) }));
                    }
                }
            }
        }

        Some((ev.name.clone(), event_hash, Value::Object(params)))
    }
}

fn dyn_value_to_json(v: DynSolValue) -> Value {
    use DynSolValue as V;
    match v {
        V::Bool(b) => json!(b),
        V::Uint(u) => json!(format!("{u}")),
        V::Int(i) => json!(format!("{i}")),
        V::Address(a) => json!(a.to_string()),
        V::FixedBytes(b) | V::Bytes(b) => json!(format!("0x{}", hex::encode(b)) ),
        V::String(s) => json!(s),
        V::Array(vals) => Value::Array(vals.into_iter().map(dyn_value_to_json).collect()),
        V::Tuple(vals) => Value::Array(vals.into_iter().map(dyn_value_to_json).collect()),
        // For other composite/enum types, fall back to string
        _ => json!(v.to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct AbiIndex {
    // address -> ContractAbi
    pub by_address: HashMap<Address, ContractAbi>,
}

impl AbiIndex {
    pub fn new(contracts: impl IntoIterator<Item = ContractAbi>) -> Self {
        let mut by_address = HashMap::new();
        for c in contracts { by_address.insert(c.address, c); }
        Self { by_address }
    }

    pub fn get(&self, addr: &Address) -> Option<&ContractAbi> { self.by_address.get(addr) }
}
