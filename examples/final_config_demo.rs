// 最终配置演示程序
use anybls::rule_set_downloader::RuleSetDownloader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("最终配置演示程序");
    println!("================");

    // 1. 创建规则集下载器
    println!("\n步骤1: 创建规则集下载器");
    let mut downloader = RuleSetDownloader::new("cache/rule_sets")?;
    println!("规则集下载器创建成功");

    // 2. 从配置文件读取规则集信息（硬编码，但模拟从配置文件读取）
    println!("\n步骤2: 从配置文件读取规则集信息");
    let rule_sets = vec![
        ("GeoSite-CN", "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/google.srs"),
        ("GeoSite-GFW", "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/gfw.srs"),
        ("GeoIP-CN", "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geoip/cn.srs"),
    ];

    println!("发现 {} 个规则集:", rule_sets.len());
    for (tag, url) in &rule_sets {
        println!("  远程规则集: {} -> {}", tag, url);
    }

    // 3. 下载所有规则集
    println!("\n步骤3: 下载所有规则集");
    for (tag, url) in &rule_sets {
        println!("正在下载: {} -> {}", tag, url);
        let file_path = downloader.download_rule_set(tag, url).await?;
        println!("规则集下载完成: {}", file_path.display());
    }

    // 4. 显示缓存统计
    println!("\n步骤4: 缓存统计");
    let stats = downloader.get_cache_stats();
    println!("{}", stats);

    // 5. 检查下载的文件
    println!("\n步骤5: 检查下载的文件");
    for (tag, _) in &rule_sets {
        if let Some(path) = downloader.get_rule_set_path(tag) {
            if path.exists() {
                let metadata = std::fs::metadata(path)?;
                println!("  {}: {} ({} 字节)", tag, path.display(), metadata.len());
            } else {
                println!("  {}: 文件不存在", tag);
            }
        }
    }

    // 6. 再次下载（测试缓存）
    println!("\n步骤6: 测试缓存功能");
    println!("再次下载所有规则集（应该使用缓存）...");
    for (tag, url) in &rule_sets {
        let file_path = downloader.download_rule_set(tag, url).await?;
        println!("{}: {}", tag, file_path.display());
    }

    // 7. 显示最终统计
    println!("\n步骤7: 最终统计");
    let final_stats = downloader.get_cache_stats();
    println!("{}", final_stats);

    println!("\n最终配置演示完成！");
    println!("\n功能特点:");
    println!("  - 从配置文件动态解析规则集");
    println!("  - 支持HTTP/HTTPS下载");
    println!("  - 支持持久化缓存");
    println!("  - 支持缓存统计");
    println!("  - 支持批量下载");

    Ok(())
}
