use async_nats::Client;

#[derive(Clone)]
pub struct Nats {
    pub client: Client,
    pub subject: String,
}

impl Nats {
    pub async fn connect(url: &str, subject: String) -> anyhow::Result<Self> {
        let client = async_nats::connect(url).await?;
        Ok(Self { client, subject })
    }

    pub async fn publish_json<T: serde::Serialize>(&self, suffix: &str, value: &T) -> anyhow::Result<()> {
        let subject = format!("{}.{}", self.subject, suffix);
        let payload = serde_json::to_vec(value)?;
        self.client.publish(subject.into(), payload.into()).await?;
        Ok(())
    }
}
