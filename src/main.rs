use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router, Extension,
    response::IntoResponse,
    extract::Path,
};
use log::debug;
use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use serde_json::{Value};
use tokio::time::{timeout, Duration};
use tokio::sync::Mutex;


// 定义 RpcRequest 结构体表示 JSON-RPC 请求
#[derive(Debug, Serialize, Deserialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Vec<Value>,
    id: usize,
}


#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse {
    // 根据实际响应的结构定义字段
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    id: usize,
}

#[derive(Debug, Deserialize)]
struct RpcParam {
    params: Vec<Value>,
}


#[tokio::main]
async fn main() {
    // initialize tracing
    // build our application with a route
    env_logger::init();

    let electrumx_host = "0.0.0.0";
    let electrumx_port = 50010;

    let tcp_stream = Arc::new(Mutex::new(TcpStream::connect(format!(
        "{}:{}",
        electrumx_host, electrumx_port
    ))
    .await
    .unwrap()));

    let app = Router::new()
        .route("/", get(root))
        .route("/proxy/health", get(health_check))
        .route("/proxy/:method", get(proxy))
        .route("/proxy/:method", post(proxy))
        .layer(Extension(tcp_stream));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    // tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    let res = r#"{"success":true,"info":{"note":"Atomicals ElectrumX Digital Object Proxy Online","usageInfo":{"note":"The service offers both POST and GET requests for proxying requests to ElectrumX. To handle larger broadcast transaction payloads use the POST method instead of GET.","POST":"POST /proxy/:method with string encoded array in the field \\\"params\\\" in the request body. ","GET":"GET /proxy/:method?params=[\\\"value1\\\"] with string encoded array in the query argument \\\"params\\\" in the URL."},"healthCheck":"GET /proxy/health","github":"https://github.com/atomicals/electrumx-proxy","license":"MIT"}}"#;
    res
}


async fn proxy(
        Extension(state): Extension<Arc<Mutex<TcpStream>>>,
        Path(method): Path<String>,
        Json(param): Json<RpcParam>,
    ) -> impl IntoResponse {

    let method = method;
    let params = param.params;
    debug!("{:#?}", method);
    debug!("{:#?}", params);

    // 调用 ElectrumX JSON-RPC
    let response = send_request(&state, &method, params).await.unwrap();
    let res = serde_json::json!({
        "success": true,
        "response": response.result
    });

    (StatusCode::OK, Json(res))
}


async fn health_check(
        Extension(state): Extension<Arc<Mutex<TcpStream>>>,
) -> impl IntoResponse {
    // Performing a simple health check by sending an ElectrumX version request
    let response = send_request(&state, "server.version", vec![]).await;

    match response {
        Ok(_) => (StatusCode::OK, "ElectrumX is healthy"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "ElectrumX is unreachable"),
    }
}

async fn send_request(stream: &Arc<Mutex<TcpStream>>, method: &str, params: Vec<serde_json::Value>) -> Result<RpcResponse, Box<dyn std::error::Error>> {
    // 构建 JSON-RPC 请求
    let request = RpcRequest {
        jsonrpc: "2.0",
        method,
        params,
        id: 1,
    };

    // 将结构体转换为 JSON 字符串
    let request_json = serde_json::to_string(&request)?;
    let a = format!("{}\n", request_json);

    // 创建异步 TCP 连接
    // let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    let mut stream = stream.lock().await;

    // 发送请求
    stream.write_all(a.as_bytes()).await?;

    let mut buffer = Vec::new();
    let mut newline_found = false;

    while !newline_found {
        let mut chunk = vec![0; 1024]; // 适当调整缓冲区大小
        let n = timeout(Duration::from_secs(5), stream.read(&mut chunk)).await??;

        if n == 0 {
            break; // 到达 EOF，结束循环
        }

        // 将读取的数据追加到缓冲区
        buffer.extend_from_slice(&chunk[..n]);

        // 检查是否包含 \n
        if let Some(index) = buffer.iter().position(|&c| c == b'\n') {
            newline_found = true;
            buffer.truncate(index + 1); // 保留包含 \n 的部分，丢弃之后的数据
        }
    }

    // 处理响应
    let response_str = String::from_utf8_lossy(&buffer).to_string();
    let response: RpcResponse = serde_json::from_str(&response_str)?;
    Ok(response)
}
