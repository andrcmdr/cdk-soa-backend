use crate::types::EventPayload;

use async_nats::jetstream::object_store::ObjectStore;

use std::io::Cursor;
use std::time::Duration;
use tracing::{error, info};

pub async fn init_nats(nats_url: &str, bucket: &str) -> anyhow::Result<ObjectStore> {
    // Create NATS Client with NATS connection, connect to NATS
    let client = loop {
        match async_nats::connect(nats_url).await {
            Ok(conn) => break conn,
            Err(e) => {
                error!("[NATS] Connection failed: {}, retrying...", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    info!("[NATS] Connected to NATS at {}", nats_url);

    let jetstream = async_nats::jetstream::new(client);

    let store = match jetstream.get_object_store(bucket).await {
        Ok(store) => store,
        Err(_) => {
            info!("[NATS] Creating object store bucket '{}'", bucket);
            jetstream.create_object_store(async_nats::jetstream::object_store::Config {
                bucket: bucket.to_string(),
                ..Default::default()
            })
            .await?
        },
    };
    Ok(store)
}

pub async fn publish_event(
    store: &ObjectStore,
    payload: &EventPayload,
) -> anyhow::Result<()> {
    let key = format!(
        "{}::{}::{}::{}::{}",
        payload.contract_name,
        payload.contract_address,
        payload.block_number,
        payload.transaction_hash,
        payload.event_name
    );

    let payload_bin = serde_json::to_vec(&serde_json::to_value(payload)?)?;
    let mut cursor = Cursor::new(payload_bin);
    store.put(key.as_str(), &mut cursor).await?;
    Ok(())
}
