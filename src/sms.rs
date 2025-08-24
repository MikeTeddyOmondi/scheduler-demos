use vercel_runtime::{run, Error};

mod api {
    use http::StatusCode;
    use serde_json::json;
    use ujumbe_sms::{UjumbeSmsClient, UjumbeSmsConfig};
    pub use vercel_runtime::{Body, Error, Request, Response};

    pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
        // Load .env variables
        let api_key = std::env::var("UJUMBESMS_API_KEY")?;
        let email = std::env::var("UJUMBESMS_EMAIL")?;

        let sms_config = UjumbeSmsConfig::new(api_key, email);
        let sms_client = UjumbeSmsClient::new(sms_config)?;

        let path = req.uri().path();
        println!("Received request for path: {}", path);

        let response = sms_client
            .send_single_message(
                "254712345678", // Replace with the recipient's phone number
                "Scheduled message from Locci Scheduler",
                "UjumbeSMS",
            )
            .await?;

        println!("Single message response: {response:#?}");

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "message": "Hello from Locci Scheduler",
                    "data": response,
                })
                .to_string()
                .into(),
            )?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(api::handler).await
}
