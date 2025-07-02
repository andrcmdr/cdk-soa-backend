use ethers::{
    abi::{Abi, RawLog},
    types::{Address, Log},
};
use sqlx::Pool;
use serde_json::json;

use crate::db::insert_event;

pub async fn process_event(
    contract_address: Address,
    log: &Log,
    abi: &Abi,
    db: &Pool<sqlx::Postgres>,
) {
    if let Some(event) = abi.events().find(|e| log.topics[0] == e.signature()) {
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        };

        if let Ok(decoded) = event.parse_log(raw_log) {
            let mut param_map = serde_json::Map::new();

            for param in decoded.params {
                let value = match serde_json::to_value(&param.value) {
                    Ok(v) => v,
                    Err(_) => json!(format!("{:?}", param.value)), // fallback
                };
                param_map.insert(param.name.clone(), value);
            }

            let json = serde_json::Value::Object(param_map);

            println!(
                "Event: {} from {}\n{}",
                event.name, contract_address, json
            );

            insert_event(
                db,
                &format!("{contract_address:?}"),
                &event.name,
                &json,
            )
            .await;
        }
    }
}
