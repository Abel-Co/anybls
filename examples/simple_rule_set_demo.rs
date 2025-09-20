// 简化的规则集下载演示程序
use anybls::ron_config::RonConfig;
use anybls::rule_set_downloader::RuleSetDownloader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("简化规则集下载演示程序");
    println!("======================");

    // 1. 加载简化的RON配置文件
    println!("\n步骤1: 加载RON配置文件");
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

    // 3. 下载规则集
    println!("\n步骤3: 下载规则集");
    let cache_dir = "cache/rule_sets";
    let downloader = config.download_rule_sets(cache_dir).await?;
    println!("规则集下载完成");

    // 4. 显示缓存统计
    println!("\n步骤4: 缓存统计");
    let stats = downloader.get_cache_stats();
    println!("{}", stats);

    // 5. 显示下载的文件
    println!("\n步骤5: 下载的文件");
    for rule_set in rule_sets {
        if rule_set.rule_set_type == "remote" {
            if let Some(path) = downloader.get_rule_set_path(&rule_set.tag) {
                if path.exists() {
                    let metadata = std::fs::metadata(path)?;
                    println!("  {}: {} ({} 字节)", 
                        rule_set.tag, 
                        path.display(),
                        metadata.len()
                    );
                } else {
                    println!("  {}: 文件不存在", rule_set.tag);
                }
            }
        }
    }

    // 6. 测试缓存功能 - 再次下载
    println!("\n步骤6: 测试缓存功能");
    println!("再次下载规则集（应该使用缓存）...");
    let downloader2 = config.download_rule_sets(cache_dir).await?;
    let stats2 = downloader2.get_cache_stats();
    println!("缓存统计: {}", stats2);

    println!("\n简化规则集下载演示完成！");
    println!("\n提示:");
    println!("  - 规则集已下载到: {}", cache_dir);
    println!("  - 下次运行时会检查更新，无变化则不重复下载");
    println!("  - 支持ETag和Last-Modified头进行变更检测");

    Ok(())
}
