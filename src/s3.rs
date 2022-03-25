use anyhow::{anyhow, Context as _, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use once_cell::sync::OnceCell;

const BUCKET_NAME: &str = "github-io-ogp";

pub struct S3Connector {
    client: Client,
}

impl S3Connector {
    async fn new() -> Result<Self> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-2");
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get_object()
            .bucket(BUCKET_NAME)
            .key(key)
            .send()
            .await?;
        let data = resp.body.collect().await?;
        Ok(data.into_bytes().to_vec())
    }

    async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
        self.client
            .put_object()
            .bucket(BUCKET_NAME)
            .key(key)
            .body(bytes.to_vec().into())
            .send()
            .await?;
        Ok(())
    }
}

static S3_CONNECTOR: OnceCell<S3Connector> = OnceCell::<S3Connector>::new();

pub async fn init() -> Result<()> {
    S3_CONNECTOR
        .set(S3Connector::new().await?)
        .map_err(|_| anyhow!("S3_CONNECTOR set failed"))?;
    Ok(())
}

pub async fn get_object(key: &str) -> Result<Vec<u8>> {
    let s3_connector = S3_CONNECTOR.get().context("base image is not set")?;
    s3_connector.get_object(key).await
}

pub async fn put_object(key: &str, bytes: &[u8]) -> Result<()> {
    let s3_connector = S3_CONNECTOR.get().context("base image is not set")?;
    s3_connector.put_object(key, bytes).await
}
