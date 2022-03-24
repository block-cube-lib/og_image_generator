use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;

const BUCKET_NAME: &str = "github-io-ogp";

pub struct S3Connector {
    client: Client,
}

impl S3Connector {
    pub async fn new() -> Result<Self> {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-2");
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
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

    pub async fn put_object(&self, key: &str, bytes: &[u8]) -> Result<()> {
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
