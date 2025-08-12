use vercel_runtime::{run, Error};
mod api {
    use http::StatusCode;
    use serde_json::json;
    pub use vercel_runtime::{run, Body, Error, Request, Response};

    pub async fn handler(_req: Request) -> Result<Response<Body>, Error> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "message": "Hello from Locci Scheduler"
                })
                .to_string()
                .into(),
            )?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Check if we're running locally
    if std::env::var("AWS_LAMBDA_RUNTIME_API").unwrap_or_default() == "localhost" {
        println!("Starting local server on http://localhost:3000");
        run_local().await
    } else {
        run(api::handler).await
    }
}

async fn run_local() -> Result<(), Error> {
    use http_body_util::Full;
    use hyper::service::service_fn;
    use hyper_util::{rt::TokioIo, server::conn::auto::Builder};
    use std::{convert::Infallible, net::SocketAddr};
    use tokio::net::TcpListener;

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let service = service_fn(|_req: hyper::Request<hyper::body::Incoming>| async {
                // Call the handler function
                let vercel_res = api::handler(vercel_runtime::Request::default())
                    .await
                    .unwrap();

                // Convert the response body to a hyper response
                let (parts, body) = vercel_res.into_parts();
                let bytes = match body {
                    vercel_runtime::Body::Empty => hyper::body::Bytes::new(),
                    vercel_runtime::Body::Text(t) => hyper::body::Bytes::from(t),
                    vercel_runtime::Body::Binary(b) => hyper::body::Bytes::from(b),
                };

                // Build the response
                Ok::<_, Infallible>(hyper::Response::from_parts(parts, Full::new(bytes)))
            });

            let exec = tokio::runtime::Handle::current();
            if let Err(err) = Builder::new(exec).serve_connection(io, service).await {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
