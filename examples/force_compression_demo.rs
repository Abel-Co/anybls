// 强制压缩演示程序
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, ACCEPT};
use reqwest::Client;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("强制压缩演示程序");
    println!("================");

    // 测试不同的URL和压缩方法
    let urls = vec![
        ("Cloudflare CDN", "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/google.json"),
        ("GitHub Raw", "https://raw.githubusercontent.com/MetaCubeX/meta-rules-dat/sing/geo/geosite/google.json"),
        ("测试压缩URL", "https://httpbin.org/gzip"), // 这个URL会返回gzip压缩内容
    ];

    for (name, url) in urls {
        println!("\n🔍 测试: {}", name);
        println!("URL: {}", url);
        
        let client = Client::new();
        
        // 方法1: 标准浏览器请求头
        let res1 = client
            .get(url)
            .header(ACCEPT_ENCODING, "gzip, br, deflate")
            .header(ACCEPT, "*/*")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?;

        let encoding1 = res1
            .headers()
            .get(CONTENT_ENCODING)
            .map(|v| v.to_str().unwrap_or("None"))
            .unwrap_or("None");
        
        let body1 = res1.bytes().await?;
        println!("  方法1 - 编码: {}, 大小: {} 字节", encoding1, body1.len());

        // 方法2: 强制压缩请求头
        let res2 = client
            .get(url)
            .header(ACCEPT_ENCODING, "gzip, br, deflate")
            .header(ACCEPT, "*/*")
            .header("User-Agent", "curl/7.68.0")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Cache-Control", "no-cache")
            .header("Pragma", "no-cache")
            .send()
            .await?;

        let encoding2 = res2
            .headers()
            .get(CONTENT_ENCODING)
            .map(|v| v.to_str().unwrap_or("None"))
            .unwrap_or("None");
        
        let body2 = res2.bytes().await?;
        println!("  方法2 - 编码: {}, 大小: {} 字节", encoding2, body2.len());

        // 方法3: 使用reqwest的自动解压缩
        let res3 = client
            .get(url)
            .header(ACCEPT_ENCODING, "gzip, br, deflate")
            .send()
            .await?;

        let encoding3 = res3
            .headers()
            .get(CONTENT_ENCODING)
            .map(|v| v.to_str().unwrap_or("None"))
            .unwrap_or("None");
        
        let text3 = res3.text().await?;
        println!("  方法3 - 编码: {}, 解压后: {} 字符", encoding3, text3.len());
    }

    println!("\n💡 强制压缩的方法总结:");
    println!("1. 使用真实的浏览器 User-Agent");
    println!("2. 发送正确的 Accept-Encoding 头");
    println!("3. 添加 Cache-Control: no-cache 避免缓存");
    println!("4. 使用 curl 等工具的用户代理");
    println!("5. 尝试不同的服务器/CDN");

    Ok(())
}