// 直接测试规则集下载器
use anybls::rule_set_downloader::RuleSetDownloader;
use anybls::ron_config::RonConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("直接规则集下载演示程序");
    println!("====================");

    // 1. 加载配置文件
    println!("\n步骤1: 加载配置文件");
    let config = RonConfig::from_ron_file("examples/simple_config.ron")?;
    println!("配置文件加载成功");

    // 2. 显示规则集配置
    println!("\n步骤2: 显示规则集配置");
    let rule_sets = config.get_rule_sets();
    println!("发现 {} 个规则集:", rule_sets.len());
    
    for rule_set in rule_sets {
        if rule_set.rule_set_type == "remote" {
            println!("  远程规则集: {} -> {}", 
                rule_set.tag, 
                rule_set.url.as_ref().unwrap_or(&"无URL".to_string())
            );
        } else {
            println!("  本地规则集: {}", rule_set.tag);
        }
    }

    // 3. 创建规则集下载器
    println!("\n步骤3: 创建规则集下载器");
    let mut downloader = RuleSetDownloader::new("cache/rule_sets")?;
    println!("规则集下载器创建成功");

    // 4. 下载所有远程规则集
    println!("\n步骤4: 下载所有远程规则集");
    for rule_set in rule_sets {
        if rule_set.rule_set_type == "remote" {
            if let Some(url) = &rule_set.url {
                println!("正在下载: {} -> {}", rule_set.tag, url);
                let file_path = downloader.download_rule_set(&rule_set.tag, url).await?;
                println!("规则集下载完成: {}", file_path.display());
            }
        }
    }

    // 5. 显示缓存统计
    println!("\n步骤5: 缓存统计");
    let stats = downloader.get_cache_stats();
    println!("{}", stats);

    // 6. 检查下载的文件
    println!("\n步骤6: 检查下载的文件");
    for rule_set in config.get_rule_sets() {
        if rule_set.rule_set_type == "remote" {
            if let Some(path) = downloader.get_rule_set_path(&rule_set.tag) {
                if path.exists() {
                    let metadata = std::fs::metadata(path)?;
                    println!("  {}: {} ({} 字节)", rule_set.tag, path.display(), metadata.len());
                } else {
                    println!("  {}: 文件不存在", rule_set.tag);
                }
            }
        }
    }

    // 7. 再次下载（测试缓存）
    println!("\n步骤7: 测试缓存功能");
    println!("再次下载所有规则集（应该使用缓存）...");
    for rule_set in config.get_rule_sets() {
        if rule_set.rule_set_type == "remote" {
            if let Some(url) = &rule_set.url {
                let file_path = downloader.download_rule_set(&rule_set.tag, url).await?;
                println!("{}: {}", rule_set.tag, file_path.display());
            }
        }
    }

    // 8. 显示最终统计
    println!("\n步骤8: 最终统计");
    let final_stats = downloader.get_cache_stats();
    println!("{}", final_stats);

    // 9. 清理过期缓存
    println!("\n步骤9: 清理过期缓存");
    downloader.cleanup_expired_cache(7)?; // 清理7天前的缓存
    let cleanup_stats = downloader.get_cache_stats();
    println!("清理后统计: {}", cleanup_stats);

    println!("\n直接规则集下载演示完成！");
    println!("\n功能特点:");
    println!("  - 支持HTTP/HTTPS下载");
    println!("  - 支持ETag和Last-Modified变更检测");
    println!("  - 支持持久化缓存");
    println!("  - 支持过期缓存清理");
    println!("  - 支持缓存统计");

    Ok(())
}
