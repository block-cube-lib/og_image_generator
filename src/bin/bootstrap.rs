use lambda_http::{http::StatusCode, service_fn, Body, Error, Request, RequestExt as _, Response};
use log::info;
use ogp_image_generator::get_ogp_image_buffer;

#[tokio::main]
async fn main() -> std::result::Result<(), lambda_http::Error> {
    let _ = dotenv::dotenv();
    env_logger::init();

    lambda_http::run(service_fn(|request: Request| async move {
        let response: Result<Response<Body>, Error> = if let Some(encoded_url) =
            request.query_string_parameters().first("encoded_url")
        {
            info!("encoded_url = {encoded_url}");
            match get_ogp_image_buffer(encoded_url).await {
                Ok(buffer) => Ok(Response::new(Body::from(buffer))),
                Err(e) => {
                    info!("failed create ogp image. {e:?}");
                    let mut response: Response<Body> = Response::<Body>::default();
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::Text(format!("failed create ogp image. {e:?}"));
                    Ok(response)
                }
            }
        } else {
            info!("query parameter error. encoded_url is not found.");
            let mut response: Response<Body> = Response::<Body>::default();
            *response.status_mut() = StatusCode::BAD_REQUEST;
            *response.body_mut() =
                Body::Text("query parameter error. encoded_url is not found.".to_string());
            Ok(response)
        };
        response
    }))
    .await?;

    Ok(())
}
