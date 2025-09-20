use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, ETAG};
use reqwest::Client;
use tokio;

/// 用于使用目的：reqwest features 直接添加 gzip, brotli.
/// 但会失去观察响应头 `content-encoding: br` / gzip 能力.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url =
        "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/google.json";

    let client = Client::new();

    let resp = client
        .get(url)
        .header(ACCEPT_ENCODING, "br")
        .header(ACCEPT, "*/*")
        .send()
        .await?;

    let get_header_value = |resp: &reqwest::Response, header_name| -> String {
        resp.headers()
            .get(header_name)
            .map(|value| value.to_str().unwrap_or("None").to_string())
            .unwrap_or_else(|| "None".to_string())
    };

    let content_length = resp.content_length().unwrap_or(0);
    let (encoding, etag) = (
        get_header_value(&resp, CONTENT_ENCODING),
        get_header_value(&resp, ETAG),
    );

    println!("Content Encoding: {}", encoding);
    println!("Content Length: {} bytes", content_length);
    println!("ETag: {}", etag);

    let resp_headers = resp.headers().clone();

    /** let content = resp.text().await?; */
    let body = resp.bytes().await?;
    let content = String::from_utf8_lossy(&body).to_string();

    // 获取文本数据
    println!("\nDownloaded {} bytes", body.len());
    println!("Decompressed {} characters", content.len());
    // println!("Content Json: {}", content);

    // 打印更多响应头信息
    println!("\n=== 完整响应头信息 ===");
    for (key, value) in resp_headers {
        if let Ok(value_str) = value.to_str() {
            if let Some(header_name) = key {
                println!("{}: {}", header_name, value_str);
            }
        }
    }
    println!("========================\n");

    Ok(())
}
