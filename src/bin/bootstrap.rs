use lambda_http::{http::StatusCode, service_fn, Body, Error, Request, RequestExt as _, Response};
use log::info;
use og_image_generator::{get_ogp_image_buffer, init};

#[tokio::main]
async fn main() -> std::result::Result<(), lambda_http::Error> {
    let _ = dotenv::dotenv();
    env_logger::init();

    init().await?;

    lambda_http::run(service_fn(|request: Request| async move {
        let response: Result<Response<Body>, Error> = if let Some(encoded_url) =
            request.query_string_parameters().first("encoded_url")
        {
            info!("encoded_url = {encoded_url}");
            let url = base64::decode(&encoded_url)?;
            let url = String::from_utf8(url)?;
            if !url.starts_with("https://block-cube-lib.github.io") {
                let msg = "query parameter error. url is not block-cube-lib.github.io constnt.";
                info!("{msg}");
                let response = create_bad_request_response(msg);
                return Ok(response);
            }

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
            let msg = "query parameter error. encoded_url is not found.";
            info!("{msg}");
            let response = create_bad_request_response(msg);
            Ok(response)
        };
        response
    }))
    .await?;

    Ok(())
}

fn create_bad_request_response(message: &str) -> Response<Body> {
    let mut response: Response<Body> = Response::<Body>::default();
    *response.status_mut() = StatusCode::BAD_REQUEST;
    *response.body_mut() = Body::Text(message.to_string());
    response
}
