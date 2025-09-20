use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, ETAG};
use reqwest::Client;
use tokio;

/// 若想观察响应头: `content-encoding: br`；
/// Cargo.toml -> reqwest features 一定不要加 gzip, brotli，并添加依赖 brotli = "3.4", flate2 = "1.0".
/// - 否则将被自动解压缩，导致无法观测。
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

    {
        /*// 获取文本数据
        let content = resp.text().await?;*/
    }

    let content = {
        // 获取原始字节数据
        let body = resp.bytes().await?;
        println!("\nDownloaded {} bytes", body.len());

        // 根据编码类型解压缩
        if encoding == "br" {
            // Brotli 解压缩
            // let mut decoder = brotli::Decompressor::new(&body[..], 4096);
            let mut decompressed = Vec::new();
            // std::io::copy(&mut decoder, &mut decompressed)?;
            String::from_utf8_lossy(&decompressed).to_string()
        } else if encoding == "gzip" {
            // Gzip 解压缩
            // let mut decoder = flate2::read::GzDecoder::new(&body[..]);
            let mut decompressed = String::new();
            // std::io::Read::read_to_string(&mut decoder, &mut decompressed)?;
            decompressed
        } else {
            // 无压缩，直接转换
            String::from_utf8_lossy(&body).to_string()
        }
    };

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
