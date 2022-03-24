use anyhow::Result;
use log::info;
use ogp_image_generator::get_ogp_image_buffer;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let encoded_url = &args[1];

    info!("encoded_url = {encoded_url}");
    get_ogp_image_buffer(&encoded_url).await?;

    Ok(())
}
