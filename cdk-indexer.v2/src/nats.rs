use crate::types::EventPayload;
use async_nats::jetstream::object_store::{ObjectStore, PutOptions};
use async_nats::jetstream::JetStream;
use async_nats::Client;

pub async fn init_nats(nats_url: &str, bucket: &str) -> anyhow::Result<ObjectStore> {
    let client: Client = async_nats::connect(nats_url).await?;
    let js = JetStream::new(client);
    let store = match js.get_object_store(bucket).await {
        Ok(store) => store,
        Err(_) => js
            .create_object_store(async_nats::jetstream::object_store::Config {
                bucket: bucket.to_string(),
                ..Default::default()
            })
            .await?,
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
    store.put(&key, payload_bin.into(), PutOptions::default()).await?;
    Ok(())
}
