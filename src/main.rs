use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // initialize tracing
    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/proxy", post(proxy));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    let res = r#"{"success":true,"info":{"note":"Atomicals ElectrumX Digital Object Proxy Online","usageInfo":{"note":"The service offers both POST and GET requests for proxying requests to ElectrumX. To handle larger broadcast transaction payloads use the POST method instead of GET.","POST":"POST /proxy/:method with string encoded array in the field \\\"params\\\" in the request body. ","GET":"GET /proxy/:method?params=[\\\"value1\\\"] with string encoded array in the query argument \\\"params\\\" in the URL."},"healthCheck":"GET /proxy/health","github":"https://github.com/atomicals/electrumx-proxy","license":"MIT"}}"#;
    res
}

async fn proxy() -> (StatusCode, Json<User>) {
    (StatusCode::CREATED, Json())
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
