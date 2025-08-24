use tracing::{error, info};
use vercel_runtime::{run, Error};

mod api {
    use http::StatusCode;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use tracing::{debug, error, info, instrument, warn, Span};
    use ujumbe_sms::{UjumbeSmsClient, UjumbeSmsConfig};
    pub use vercel_runtime::{Body, Error, Request, Response};

    #[derive(Deserialize, Debug)]
    struct RequestData {
        phone: Option<String>,
        message: Option<String>,
        sender_id: Option<String>,
        // Add other fields as needed
    }

    #[derive(Serialize)]
    struct ApiResponse {
        message: String,
        data: Option<Value>,
        request_info: RequestInfo,
        trace_id: String,
    }

    #[derive(Serialize)]
    struct RequestInfo {
        has_body_data: bool,
        query_params: std::collections::HashMap<String, String>,
        path: String,
        method: String,
    }

    // Helper function to parse query parameters
    #[instrument(level = "debug")]
    fn parse_query_params(query: Option<&str>) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();

        if let Some(query_str) = query {
            debug!("Parsing query string: {}", query_str);
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    let decoded_key = urlencoding::decode(key).unwrap_or_default().to_string();
                    let decoded_value = urlencoding::decode(value).unwrap_or_default().to_string();

                    debug!("Parsed query param: {} = {}", decoded_key, decoded_value);
                    params.insert(decoded_key, decoded_value);
                }
            }
            info!("Parsed {} query parameters", params.len());
        } else {
            debug!("No query string found");
        }

        params
    }

    async fn send_sms(
        client: &UjumbeSmsClient,
        phone: &str,
        message: &str,
        sender_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        info!("Attempting to send SMS to: {}", phone);
        debug!(
            "SMS details - Sender: {}, Message length: {}",
            sender_id,
            message.len()
        );

        let response = client
            .send_single_message(phone, message, sender_id)
            .await?;

        info!("SMS sent successfully to: {}", phone);
        debug!("SMS response: {:#?}", response);

        Ok(json!(response))
    }

    #[instrument(level = "info", skip(req))]
    pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
        // Generate trace ID for this request
        let trace_id = uuid::Uuid::new_v4().to_string();
        let span = Span::current();
        span.record("trace_id", &trace_id);

        info!("Starting request processing with trace_id: {}", trace_id);

        // Load .env variables
        let api_key = match std::env::var("UJUMBESMS_API_KEY") {
            Ok(key) => {
                debug!("Successfully loaded UJUMBESMS_API_KEY");
                key
            }
            Err(e) => {
                error!("Failed to load UJUMBESMS_API_KEY: {}", e);
                return Err(e.into());
            }
        };

        let email = match std::env::var("UJUMBESMS_EMAIL") {
            Ok(email) => {
                debug!("Successfully loaded UJUMBESMS_EMAIL: {}", email);
                email
            }
            Err(e) => {
                error!("Failed to load UJUMBESMS_EMAIL: {}", e);
                return Err(e.into());
            }
        };

        info!("Initializing SMS client");
        let sms_config = UjumbeSmsConfig::new(api_key, email);
        let sms_client = match UjumbeSmsClient::new(sms_config) {
            Ok(client) => {
                debug!("SMS client initialized successfully");
                client
            }
            Err(e) => {
                error!("Failed to initialize SMS client: {}", e);
                return Err(Box::new(e));
            }
        };

        // Get request info
        let path = req.uri().path().to_string();
        let method = req.method().to_string();
        let query_params = parse_query_params(req.uri().query());

        info!("Processing {} request for path: {}", method, path);
        if !query_params.is_empty() {
            debug!("Query parameters: {:?}", query_params);
        }

        // Parse request body
        info!("Reading request body");
        let body_bytes = match req.into_body() {
            Body::Binary(bytes) => {
                debug!("Received binary body with {} bytes", bytes.len());
                bytes
            }
            Body::Text(text) => {
                debug!("Received text body with {} characters", text.len());
                text.into_bytes()
            }
            Body::Empty => {
                debug!("Received empty body");
                Vec::new()
            }
        };

        let request_data: Option<RequestData> = if !body_bytes.is_empty() {
            info!("Attempting to parse request body as JSON");
            match serde_json::from_slice::<RequestData>(&body_bytes) {
                Ok(data) => {
                    info!("Successfully parsed request data");
                    debug!("Parsed request data: {:?}", data);
                    Some(data)
                }
                Err(e) => {
                    warn!("Failed to parse JSON body: {}", e);
                    // Try to parse as raw text if JSON parsing fails
                    if let Ok(text) = String::from_utf8(body_bytes.clone()) {
                        debug!(
                            "Raw body text (first 200 chars): {}",
                            text.chars().take(200).collect::<String>()
                        );
                    } else {
                        warn!("Body is not valid UTF-8");
                    }
                    None
                }
            }
        } else {
            debug!("No body data received");
            None
        };

        // Determine response based on whether we have data or not
        let (response_message, sms_response_data) =
            if request_data.is_some() || !query_params.is_empty() {
                // We have data (either in body or query params), send greeting message
                info!("Data detected - returning greeting message");
                ("Hello from Locci Scheduler - Data received!", None)
            } else {
                // No data, send SMS
                info!("No data detected - sending default SMS");
                let phone = "254717135176"; // Default phone or get from somewhere
                let message = "Scheduled message from Locci Scheduler";
                let sender_id = "UjumbeSMS";

                match send_sms(&sms_client, phone, message, sender_id).await {
                    Ok(response) => {
                        info!("Default SMS sent successfully");
                        ("SMS sent successfully", Some(response))
                    }
                    Err(e) => {
                        error!("Failed to send default SMS: {}", e);
                        ("Failed to send SMS", Some(json!({"error": e.to_string()})))
                    }
                }
            };

        // If we have request data, we can also use it to send SMS with custom values
        let final_sms_data = if let Some(data) = &request_data {
            if let (Some(phone), Some(msg)) = (&data.phone, &data.message) {
                info!("Sending custom SMS based on request data");
                let sender = data.sender_id.as_deref().unwrap_or("UjumbeSMS");

                match send_sms(&sms_client, phone, msg, sender).await {
                    Ok(response) => {
                        info!("Custom SMS sent successfully to: {}", phone);
                        Some(response)
                    }
                    Err(e) => {
                        error!("Failed to send custom SMS to {}: {}", phone, e);
                        Some(json!({"error": e.to_string()}))
                    }
                }
            } else {
                if data.phone.is_none() {
                    debug!("No phone number provided in request data");
                }
                if data.message.is_none() {
                    debug!("No message provided in request data");
                }
                sms_response_data
            }
        } else {
            sms_response_data
        };

        info!("Building API response");
        let api_response = ApiResponse {
            message: response_message.to_string(),
            data: final_sms_data,
            request_info: RequestInfo {
                has_body_data: request_data.is_some(),
                query_params,
                path,
                method,
            },
            trace_id: trace_id.clone(),
        };

        info!(
            "Request processing completed successfully - trace_id: {}",
            trace_id
        );

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*") // Enable CORS if needed
            .header(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            )
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization",
            )
            .header("X-Trace-Id", &trace_id) // Include trace ID in response headers
            .body(match serde_json::to_string(&api_response) {
                Ok(json_str) => {
                    debug!("Response serialized successfully");
                    json_str.into()
                }
                Err(e) => {
                    error!("Failed to serialize response: {}", e);
                    return Err(e.into());
                }
            })?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(true)
        // .with_thread_ids(true)
        // .with_file(true)
        .with_line_number(true)
        .init();

    info!("Locci Scheduler Demo server initiated...");
    info!("Tracing initialized...");

    match run(api::handler).await {
        Ok(_) => {
            info!("API server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("API server error: {}", e);
            Err(e)
        }
    }
}
