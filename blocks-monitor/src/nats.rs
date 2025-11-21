use async_nats::{jetstream, jetstream::Context, Client};
use jetstream::object_store::ObjectStore;

use crate::types::{EventPayload, BlockPayload};

use std::io::Cursor;
use std::time::Duration;
use async_nats::jetstream::object_store::Object;
use tracing::{error, info};

#[derive(Clone)]
pub struct Nats {
    pub client: Client,
    pub js: Context,
    pub object_store: ObjectStore,
}

pub async fn connect(url: &str, bucket: &str) -> anyhow::Result<Nats> {
    // Create NATS Client with NATS connection, connect to NATS
    let client = loop {
        match async_nats::connect(url).await {
            Ok(conn) => break conn,
            Err(e) => {
                error!("[NATS] Connection failed: {}, retrying...", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    info!("[NATS] Connected to NATS at {}", url);

    let js = jetstream::new(client.clone());

    // ensure bucket exists; if already exists, get; otherwise, create
    let object_store = match js.get_object_store(bucket).await {
        Ok(store) => store,
        Err(_) => {
            info!("[NATS] Creating object store bucket '{}'", bucket);
            js.create_object_store(jetstream::object_store::Config {
                bucket: bucket.to_string(),
                ..Default::default()
            })
            .await?
        },
    };
    info!(bucket, "NATS Object Store ready");

    Ok(Nats { client, js, object_store })
}

pub async fn publish_event(
    object_store: &ObjectStore,
    payload: &EventPayload,
) -> anyhow::Result<()> {
    let key = format!(
        "event:{}::{}::{:?}::{:?}::{}::{}::{}::{}::{}::{}",
        payload.contract_name,
        payload.contract_address,
        payload.implementation_name,
        payload.implementation_address,
        payload.chain_id,
        payload.transaction_hash,
        payload.transaction_sender,
        payload.transaction_receiver,
        payload.event_name,
        payload.event_signature,
    );

    let bytes = serde_json::to_vec(&serde_json::to_value(payload)?)?;
    let mut cursor = Cursor::new(bytes);
    let _obj = object_store.put(key.as_str(), &mut cursor).await?;
    Ok(())
}

pub async fn publish_block(
    object_store: &ObjectStore,
    payload: &BlockPayload,
) -> anyhow::Result<()> {
    let key = format!(
        "block::{}::{}::{}",
        payload.chain_id,
        payload.block_number,
        payload.block_hash,
    );

    let bytes = serde_json::to_vec(&serde_json::to_value(payload)?)?;
    let mut cursor = Cursor::new(bytes);
    let _obj = object_store.put(key.as_str(), &mut cursor).await?;
    Ok(())
}
