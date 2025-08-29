use alloy::primitives::{Address, Bytes, FixedBytes, Log, LogData, B256};
use alloy::json_abi::{Event, EventParam, JsonAbi, Param};
use alloy_dyn_abi::{DynSolValue, DynSolType};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use serde_json::Value;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ParsedEventParam {
    pub name: String,
    pub param_type: String,
    pub value: DynSolValue,
    pub indexed: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedEvent {
    pub name: String,
    pub signature: Option<B256>, // None for anonymous events
    pub params: Vec<ParsedEventParam>,
    pub anonymous: bool,
}

pub struct EventDecoder {
    events: HashMap<B256, Event>,
    anonymous_events: Vec<Event>,
    anonymous_presence: HashMap<String, bool>,
}

impl EventDecoder {
    /// Helper function to check if anonymous field is explicitly present in JSON, accepted as Serde JSON Value
    pub fn check_anonymous_field_presence_from_val(raw_value: &Value) -> Result<HashMap<String, bool>> {
        let mut anonymous_presence = HashMap::new();

        if let Value::Array(events) = raw_value {
            for event in events {
                if let Value::Object(obj) = event {
                    if let Some(Value::String(name)) = obj.get("name") {
                        anonymous_presence.insert(name.clone(), obj.contains_key("anonymous"));
                    }
                }
            }
        }

        Ok(anonymous_presence)
    }

    /// Helper function to check if anonymous field is explicitly present in JSON, accepted as str
    pub fn check_anonymous_field_presence_from_str(abi_json: &str) -> Result<HashMap<String, bool>> {
        // Parse as raw JSON first to check 'anonymous' field presence
        let raw_value: Value = serde_json::from_str(abi_json)?;

        EventDecoder::check_anonymous_field_presence_from_val(&raw_value)
    }

    /// Helper function to check if anonymous field is explicitly present in JSON, accepted as Vec<u8>
    pub fn check_anonymous_field_presence_from_vec(abi_json: &[u8]) -> Result<HashMap<String, bool>> {
        // Parse as raw JSON first to check 'anonymous' field presence
        let raw_value: Value = serde_json::from_slice(abi_json)?;

        EventDecoder::check_anonymous_field_presence_from_val(&raw_value)
    }

    /// Create a new EventDecoder from a JSON ABI
    pub fn new(abi_json: Arc<JsonAbi>, anonymous_presence: HashMap<String, bool>) -> Result<Self> {
        let mut events = HashMap::new();
        let mut anonymous_events = Vec::new();

        for event in abi_json.events() {
            let explicit_anonymous = anonymous_presence
                .get(&event.name)
                .copied()
                .unwrap_or(false);

            if explicit_anonymous {
                // Field is explicitly present, use its value
                if event.anonymous {
                    anonymous_events.push(event.clone());
                } else {
                    let signature = event.selector();
                    events.insert(signature, event.clone());
                }
            } else {
                // Field is not present, default to non-anonymous
                warn!("Event {} has no explicit anonymous field, treating as non-anonymous", event.name);
                let signature = event.selector();
                events.insert(signature, event.clone());
            }
        }

        Ok(Self {
            events,
            anonymous_events,
            anonymous_presence,
        })
    }

    /// Create a new EventDecoder from a JSON ABI string
    pub fn from_str(abi_json: &str) -> Result<Self> {
        // First check which events have explicit anonymous fields
        let anonymous_presence = EventDecoder::check_anonymous_field_presence_from_str(abi_json)?;

        // Then parse with JsonAbi
        let abi: JsonAbi = serde_json::from_str(abi_json)?;

        EventDecoder::new(abi.into(), anonymous_presence)
    }

    /// Create a new EventDecoder from a JSON ABI vector/array
    pub fn from_vec(abi_json: &[u8]) -> Result<Self> {
        // First check which events have explicit anonymous fields
        let anonymous_presence = EventDecoder::check_anonymous_field_presence_from_vec(abi_json)?;

        // Then parse with JsonAbi
        let abi: JsonAbi = serde_json::from_slice(abi_json)?;

        EventDecoder::new(abi.into(), anonymous_presence)
    }

    /// Create a new EventDecoder from a JSON ABI read from a file by its path
    pub fn from_file(abi_path: &Path) -> Result<Self> {
        let abi_json = std::fs::read_to_string(abi_path)?;

        // First check which events have explicit anonymous fields
        let anonymous_presence = EventDecoder::check_anonymous_field_presence_from_str(&abi_json.clone())?;

        // Then parse with JsonAbi
        let abi: JsonAbi = serde_json::from_str(&abi_json)?;

        EventDecoder::new(abi.into(), anonymous_presence)
    }

    /// Create EventDecoder from individual events
    pub fn from_events(events: Vec<Event>, anonymous_presence: HashMap<String, bool>) -> Result<Self> {
        let mut event_map = HashMap::new();
        let mut anonymous_events = Vec::new();

        for event in events {
            let explicit_anonymous = anonymous_presence
                .get(&event.name)
                .copied()
                .unwrap_or(false);

            if explicit_anonymous {
                // Field is explicitly present, use its value
                if event.anonymous {
                    anonymous_events.push(event.clone());
                } else {
                    let signature = event.selector();
                    event_map.insert(signature, event.clone());
                }
            } else {
                // Field is not present, default to non-anonymous
                warn!("Event {} has no explicit anonymous field, treating as non-anonymous", event.name);
                let signature = event.selector();
                event_map.insert(signature, event.clone());
            }
        }

        Ok(Self {
            events: event_map,
            anonymous_events,
            anonymous_presence,
        })
    }

    /// Decode a log entry into a ParsedEvent
    pub fn decode_log(&self, log: &Log) -> Result<ParsedEvent> {
        // First try to decode as a regular (non-anonymous) event
        if !log.topics().is_empty() {
            let event_signature = log.topics()[0];
            if let Some(event) = self.events.get(&event_signature) {
                return self.decode_log_with_event(log, event);
            }
        }

        // If no matching regular event found, try anonymous events
        self.try_decode_anonymous_event(log)
    }

    /// Try to decode log as an anonymous event
    pub fn try_decode_anonymous_event(&self, log: &Log) -> Result<ParsedEvent> {
        let mut last_error = None;

        for event in &self.anonymous_events {
            match self.decode_anonymous_log_with_event(log, event) {
                Ok(parsed) => return Ok(parsed),
                Err(e) => last_error = Some(e),
            }
        }

        match last_error {
            Some(e) => Err(anyhow!("Failed to decode as any anonymous event: {}", e)),
            None => Err(anyhow!("No anonymous events available for decoding")),
        }
    }

    /// Decode a log entry using a specific event definition
    pub fn decode_log_with_event(&self, log: &Log, event: &Event) -> Result<ParsedEvent> {
        if event.anonymous {
            return self.decode_anonymous_log_with_event(log, event);
        }

        let mut parsed_params = Vec::new();
        let mut topic_index = 1; // Skip the first topic (event signature)

        // Separate indexed and non-indexed parameters
        let indexed_params: Vec<&EventParam> = event.inputs.iter().filter(|p| p.indexed).collect();
        let non_indexed_params: Vec<&EventParam> = event.inputs.iter().filter(|p| !p.indexed).collect();

        // Decode indexed parameters from topics
        for param in &indexed_params {
            if topic_index >= log.topics().len() {
                return Err(anyhow!("Not enough topics for indexed parameter: {}", param.name));
            }

            let topic = log.topics()[topic_index];
            let value = self.decode_indexed_param(param, topic)?;

            parsed_params.push(ParsedEventParam {
                name: param.name.clone(),
                param_type: param.ty.to_string(),
                value,
                indexed: true,
            });

            topic_index += 1;
        }

        // Decode non-indexed parameters from data
        if !non_indexed_params.is_empty() {
            let data_values = self.decode_data_params(&non_indexed_params, &log.data.data)?;

            for (param, value) in non_indexed_params.iter().zip(data_values.iter()) {
                parsed_params.push(ParsedEventParam {
                    name: param.name.clone(),
                    param_type: param.ty.to_string(),
                    value: value.clone(),
                    indexed: false,
                });
            }
        }

        // Sort parameters by their original order in the event definition
        parsed_params.sort_by_key(|p| {
            event.inputs.iter().position(|param| param.name == p.name).unwrap_or(usize::MAX)
        });

        Ok(ParsedEvent {
            name: event.name.clone(),
            signature: Some(event.selector()),
            params: parsed_params,
            anonymous: false,
        })
    }

    /// Decode an anonymous event log using a specific event definition
    pub fn decode_anonymous_log_with_event(&self, log: &Log, event: &Event) -> Result<ParsedEvent> {
        if !event.anonymous {
            return Err(anyhow!("Event {} is not anonymous", event.name));
        }

        let mut parsed_params = Vec::new();
        let mut topic_index = 0; // Start from first topic for anonymous events

        // Separate indexed and non-indexed parameters
        let indexed_params: Vec<&EventParam> = event.inputs.iter().filter(|p| p.indexed).collect();
        let non_indexed_params: Vec<&EventParam> = event.inputs.iter().filter(|p| !p.indexed).collect();

        // Check if we have enough topics for all indexed parameters
        if indexed_params.len() > log.topics().len() {
            return Err(anyhow!(
                "Not enough topics ({}) for indexed parameters ({}) in anonymous event {}",
                log.topics().len(),
                indexed_params.len(),
                event.name
            ));
        }

        // Decode indexed parameters from topics
        for param in &indexed_params {
            if topic_index >= log.topics().len() {
                return Err(anyhow!("Not enough topics for indexed parameter: {}", param.name));
            }

            let topic = log.topics()[topic_index];
            let value = self.decode_indexed_param(param, topic)?;

            parsed_params.push(ParsedEventParam {
                name: param.name.clone(),
                param_type: param.ty.to_string(),
                value,
                indexed: true,
            });

            topic_index += 1;
        }

        // Decode non-indexed parameters from data
        if !non_indexed_params.is_empty() {
            let data_values = self.decode_data_params(&non_indexed_params, &log.data.data)?;

            for (param, value) in non_indexed_params.iter().zip(data_values.iter()) {
                parsed_params.push(ParsedEventParam {
                    name: param.name.clone(),
                    param_type: param.ty.to_string(),
                    value: value.clone(),
                    indexed: false,
                });
            }
        }

        // Sort parameters by their original order in the event definition
        parsed_params.sort_by_key(|p| {
            event.inputs.iter().position(|param| param.name == p.name).unwrap_or(usize::MAX)
        });

        Ok(ParsedEvent {
            name: event.name.clone(),
            signature: None, // Anonymous events don't have signatures
            params: parsed_params,
            anonymous: true,
        })
    }

    /// Try to decode log with a specific anonymous event by name
    pub fn try_decode_anonymous_event_by_name(&self, log: &Log, event_name: &str) -> Result<ParsedEvent> {
        let event = self.anonymous_events
            .iter()
            .find(|e| e.name == event_name)
            .ok_or_else(|| anyhow!("Anonymous event '{}' not found", event_name))?;

        self.decode_anonymous_log_with_event(log, event)
    }

    /// Get all anonymous events
    pub fn get_anonymous_events(&self) -> &[Event] {
        &self.anonymous_events
    }

    /// Check if a log could potentially be an anonymous event based on topic count and data
    pub fn could_be_anonymous_event(&self, log: &Log, event_name: &str) -> bool {
        if let Some(event) = self.anonymous_events.iter().find(|e| e.name == event_name) {
            let indexed_count = event.inputs.iter().filter(|p| p.indexed).count();
            let non_indexed_count = event.inputs.iter().filter(|p| !p.indexed).count();

            // Check topic count matches indexed parameters
            if log.topics().len() != indexed_count {
                return false;
            }

            // For non-indexed parameters, we can only do a basic check if data is present
            if non_indexed_count > 0 && log.data.data.is_empty() {
                return false;
            }

            true
        } else {
            false
        }
    }

    /// Decode an indexed parameter from a topic
    fn decode_indexed_param(&self, param: &EventParam, topic: B256) -> Result<DynSolValue> {
        let sol_type = DynSolType::parse(&param.ty)?;

        // For dynamic types (strings, bytes, arrays), topics contain keccak256 hashes
        match &sol_type {
            DynSolType::String | DynSolType::Bytes => {
                // Return the hash as bytes32 since we can't recover the original value
                Ok(DynSolValue::FixedBytes(topic.0.into(), 32))
            }
//            DynSolType::Array(_) | DynSolType::Slice(_) => {
            DynSolType::Array(_) => {
                // Return the hash as bytes32 since we can't recover the original value
                Ok(DynSolValue::FixedBytes(topic.0.into(), 32))
            }
            _ => {
                // For fixed-size types, decode directly from the topic
                let topic_bytes = topic.as_slice();
                sol_type.abi_decode_params(topic_bytes)
                    .map_err(|e| anyhow!("Failed to decode indexed parameter {}: {}", param.name, e))
            }
        }
    }

    /// Decode non-indexed parameters from log data
    fn decode_data_params(&self, params: &[&EventParam], data: &Bytes) -> Result<Vec<DynSolValue>> {
        if params.is_empty() {
            return Ok(Vec::new());
        }

        // Create tuple type from all non-indexed parameters
        let param_types: Result<Vec<DynSolType>> = params.iter()
            .map(|p| DynSolType::parse(&p.ty).map_err(|e| anyhow!("Failed to decode non-indexed parameter {} of type {}: {}", p.name, p.ty, e)))
            .collect();
        let param_types = param_types?;

        let tuple_type = DynSolType::Tuple(param_types);

        // Decode the data as a tuple
        let decoded = tuple_type.abi_decode_params(data)
            .map_err(|e| anyhow!("Failed to decode log data: {}", e))?;

        // Extract values from the tuple
        match decoded {
            DynSolValue::Tuple(values) => Ok(values),
            _ => Err(anyhow!("Expected tuple from log data decoding")),
        }
    }

    /// Get all available event signatures (non-anonymous events only)
    pub fn get_event_signatures(&self) -> Vec<B256> {
        self.events.keys().copied().collect()
    }

    /// Get event by signature (non-anonymous events only)
    pub fn get_event(&self, signature: B256) -> Option<&Event> {
        self.events.get(&signature)
    }

    /// Get total count of events (both anonymous and non-anonymous)
    pub fn total_events_count(&self) -> usize {
        self.events.len() + self.anonymous_events.len()
    }

    /// Get count of anonymous events
    pub fn anonymous_events_count(&self) -> usize {
        self.anonymous_events.len()
    }

    /// Get count of non-anonymous events
    pub fn regular_events_count(&self) -> usize {
        self.events.len()
    }
}

/// Helper function to format parsed event parameters for display
impl ParsedEvent {
    pub fn to_json(&self) -> Result<Value> {
        let mut event_json = serde_json::Map::new();
        event_json.insert("name".to_string(), Value::String(self.name.clone()));
        event_json.insert("anonymous".to_string(), Value::Bool(self.anonymous));

        match self.signature {
            Some(sig) => event_json.insert("signature".to_string(), Value::String(format!("{:#x}", sig))),
            None => event_json.insert("signature".to_string(), Value::Null),
        };

        let mut params_json = Vec::new();
        for param in &self.params {
            let mut param_json = serde_json::Map::new();
            param_json.insert("name".to_string(), Value::String(param.name.clone()));
            param_json.insert("type".to_string(), Value::String(param.param_type.clone()));
            param_json.insert("indexed".to_string(), Value::Bool(param.indexed));
            param_json.insert("value".to_string(), value_to_json(&param.value)?);
            params_json.push(Value::Object(param_json));
        }

        event_json.insert("parameters".to_string(), Value::Array(params_json));
        Ok(Value::Object(event_json))
    }

    pub fn format_params(&self) -> String {
        self.params
            .iter()
            .map(|p| format!("{}: {} = {}", p.name, p.param_type, format_value(&p.value)))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Convert DynSolValue to JSON Value for serialization
fn value_to_json(value: &DynSolValue) -> Result<Value> {
    match value {
        DynSolValue::Bool(b) => Ok(Value::Bool(*b)),
        DynSolValue::Int(i, _) => Ok(Value::String(i.to_string())),
        DynSolValue::Uint(u, _) => Ok(Value::String(u.to_string())),
        DynSolValue::FixedBytes(bytes, _) => Ok(Value::String(hex::encode(bytes))),
        DynSolValue::Bytes(bytes) => Ok(Value::String(hex::encode(bytes))),
        DynSolValue::Address(addr) => Ok(Value::String(format!("{:#x}", addr))),
        DynSolValue::String(s) => Ok(Value::String(s.clone())),
        DynSolValue::Array(arr) => {
            let json_arr: Result<Vec<Value>> = arr.iter().map(value_to_json).collect();
            Ok(Value::Array(json_arr?))
        }
        DynSolValue::Tuple(tuple) => {
            let json_arr: Result<Vec<Value>> = tuple.iter().map(value_to_json).collect();
            Ok(Value::Array(json_arr?))
        }
        _ => Ok(Value::String(format!("{:?}", value))),
    }
}

/// Format DynSolValue for human-readable display
fn format_value(value: &DynSolValue) -> String {
    match value {
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Uint(u, _) => u.to_string(),
        DynSolValue::FixedBytes(bytes, _) => format!("0x{}", hex::encode(bytes)),
        DynSolValue::Bytes(bytes) => format!("0x{}", hex::encode(bytes)),
        DynSolValue::Address(addr) => format!("{:#x}", addr),
        DynSolValue::String(s) => format!("\"{}\"", s),
        DynSolValue::Array(arr) => {
            let formatted: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        DynSolValue::Tuple(tuple) => {
            let formatted: Vec<String> = tuple.iter().map(format_value).collect();
            format!("({})", formatted.join(", "))
        }
        _ => format!("{:?}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{LogData, U256};

    #[test]
    fn test_event_decoder_creation() {
        let abi_json = r#"[
            {
                "type": "event",
                "name": "Transfer",
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "value", "type": "uint256", "indexed": false}
                ]
            }
        ]"#;

        let decoder = EventDecoder::from_str(abi_json).unwrap();
        assert_eq!(decoder.events.len(), 1);
    }

    #[test]
    fn test_log_decoding() {
        let abi_json = r#"[
            {
                "type": "event",
                "name": "Transfer",
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "value", "type": "uint256", "indexed": false}
                ]
            }
        ]"#;

        let decoder = EventDecoder::from_str(abi_json).unwrap();

        // Create a mock log (you would replace this with actual log data)
        let transfer_signature = B256::from_slice(&hex::decode("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap());
        let from_addr = B256::from_slice(&hex::decode("000000000000000000000000742d35Cc6634C0532925a3b8BC342A5b6437AFCD").unwrap());
        let to_addr = B256::from_slice(&hex::decode("000000000000000000000000742d35Cc6634C0532925a3b8BC342A5b6437AFCD").unwrap());

        let topics = vec![transfer_signature, from_addr, to_addr];
        let data = Bytes::from(hex::decode("0000000000000000000000000000000000000000000000000de0b6b3a7640000").unwrap());

        let log_data = LogData::new_unchecked(topics, data);
        let log = Log {
            address: Address::ZERO,
            data: log_data,
        };

        let parsed = decoder.decode_log(&log).unwrap();
        assert_eq!(parsed.name, "Transfer");
        assert_eq!(parsed.params.len(), 3);
    }

    #[test]
    fn test_event_decoder_with_anonymous_events() {
        let abi_json = r#"[
            {
                "type": "event",
                "name": "Transfer",
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "value", "type": "uint256", "indexed": false}
                ]
            },
            {
                "type": "event",
                "name": "AnonymousTransfer",
                "anonymous": true,
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "amount", "type": "uint256", "indexed": false}
                ]
            }
        ]"#;

        let decoder = EventDecoder::from_str(abi_json).unwrap();
        assert_eq!(decoder.regular_events_count(), 1);
        assert_eq!(decoder.anonymous_events_count(), 1);
        assert_eq!(decoder.total_events_count(), 2);
    }

    #[test]
    fn test_anonymous_event_decoding() {
        let abi_json = r#"[
            {
                "type": "event",
                "name": "AnonymousTransfer",
                "anonymous": true,
                "inputs": [
                    {"name": "from", "type": "address", "indexed": true},
                    {"name": "to", "type": "address", "indexed": true},
                    {"name": "amount", "type": "uint256", "indexed": false}
                ]
            }
        ]"#;

        let decoder = EventDecoder::from_str(abi_json).unwrap();

        // Create a mock log for anonymous event (no event signature in first topic)
        let from_addr = B256::from_slice(&hex::decode("000000000000000000000000742d35Cc6634C0532925a3b8BC342A5b6437AFCD").unwrap());
        let to_addr = B256::from_slice(&hex::decode("000000000000000000000000742d35Cc6634C0532925a3b8BC342A5b6437AFCE").unwrap());

        let topics = vec![from_addr, to_addr]; // No event signature for anonymous events
        let data = Bytes::from(hex::decode("0000000000000000000000000000000000000000000000000de0b6b3a7640000").unwrap());

        let log_data = LogData::new_unchecked(topics, data);
        let log = Log {
            address: Address::ZERO,
            data: log_data,
        };

        let parsed = decoder.try_decode_anonymous_event_by_name(&log, "AnonymousTransfer").unwrap();
        assert_eq!(parsed.name, "AnonymousTransfer");
        assert!(parsed.anonymous);
        assert!(parsed.signature.is_none());
        assert_eq!(parsed.params.len(), 3);
    }

    #[test]
    fn test_could_be_anonymous_event() {
        let abi_json = r#"[
            {
                "type": "event",
                "name": "AnonymousEvent",
                "anonymous": true,
                "inputs": [
                    {"name": "param1", "type": "uint256", "indexed": true},
                    {"name": "param2", "type": "string", "indexed": false}
                ]
            }
        ]"#;

        let decoder = EventDecoder::from_str(abi_json).unwrap();

        // Create log with 1 topic and some data
        let topics = vec![B256::ZERO];
        let data = Bytes::from(vec![1, 2, 3, 4]);
        let log_data = LogData::new_unchecked(topics, data.clone());
        let log = Log {
            address: Address::ZERO,
            data: log_data,
        };

        assert!(decoder.could_be_anonymous_event(&log, "AnonymousEvent"));

        // Test with wrong topic count
        let topics = vec![B256::ZERO, B256::ZERO]; // Too many topics
        let log_data = LogData::new_unchecked(topics, data);
        let log = Log {
            address: Address::ZERO,
            data: log_data,
        };

        assert!(!decoder.could_be_anonymous_event(&log, "AnonymousEvent"));
    }
}

/*

// Event Decoder Library usage examples (for regular non-anonymous events and anonymous events):

use alloy::primitives::{Log, LogData, Address, B256, Bytes};

fn main() -> Result<()> {
    // Create decoder from ABI JSON
    let abi_json = r#"[
        {
            "type": "event",
            "name": "Transfer",
            "inputs": [
                {"name": "from", "type": "address", "indexed": true},
                {"name": "to", "type": "address", "indexed": true},
                {"name": "value", "type": "uint256", "indexed": false}
            ]
        }
    ]"#;

    let decoder = EventDecoder::from_str(abi_json)?;

    // Decode a log
    let log = get_log_from_blockchain(); // A sample log subscription implementation to get logs
    let parsed_event = decoder.decode_log(&log)?;

    println!("Event: {}", parsed_event.name);
    println!("Parameters: {}", parsed_event.format_params());
    println!("JSON: {}", serde_json::to_string_pretty(&parsed_event.to_json()?)?);

    Ok(())
}

fn anonymous() -> Result<()> {
    // Create decoder from ABI JSON
    let abi_json = r#"[
        {
            "type": "event",
            "name": "Transfer",
            "inputs": [
                {"name": "from", "type": "address", "indexed": true},
                {"name": "to", "type": "address", "indexed": true},
                {"name": "value", "type": "uint256", "indexed": false}
            ]
        },
        {
            "type": "event",
            "name": "AnonymousTransfer",
            "anonymous": true,
            "inputs": [
                {"name": "from", "type": "address", "indexed": true},
                {"name": "to", "type": "address", "indexed": true},
                {"name": "amount", "type": "uint256", "indexed": false}
            ]
        }
    ]"#;

    let decoder = EventDecoder::new(abi_json)?;

    // Try to decode any log (will attempt both regular and anonymous)
    let parsed_event = decoder.decode_log(&log)?;

    // Specifically try anonymous events
    let anonymous_parsed = decoder.try_decode_anonymous_event(&log)?;

    // Try specific anonymous event by name
    let specific_anonymous = decoder.try_decode_anonymous_event_by_name(&log, "AnonymousTransfer")?;

    // Check if log could be a specific anonymous event
    if decoder.could_be_anonymous_event(&log, "AnonymousTransfer") {
        println!("This log could be an AnonymousTransfer event");
    }
}

*/
