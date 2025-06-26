use ethers::{
    abi::{Abi, RawLog},
    types::{Address, Log},
};
use sqlx::Pool;
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
            let mut setter = String::new();
            let mut value = String::new();

            for param in decoded.params {
                match param.name.as_str() {
                    "setter" => setter = format!("{:?}", param.value),
                    "value" => value = format!("{:?}", param.value),
                    _ => (),
                }
            }

            println!("Event from {}: {} -> {}", contract_address, setter, value);
            insert_event(db, &setter, &value).await;
        }
    }
}
