// 规则集下载演示程序
use anybls::ron_config::RonConfig;
use anybls::rule_set_downloader::{RuleSetDownloader, CacheStats};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("规则集下载演示程序");
    println!("==================");

    // 1. 加载RON配置文件
    println!("\n步骤1: 加载RON配置文件");
    let config = RonConfig::from_ron_file("examples/config.ron")?;
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

    // 7. 清理过期缓存（可选）
    println!("\n步骤7: 清理过期缓存");
    let mut downloader3 = RuleSetDownloader::new(cache_dir)?;
    downloader3.cleanup_expired_cache(1)?; // 清理1天前的缓存
    let stats3 = downloader3.get_cache_stats();
    println!("清理后缓存统计: {}", stats3);

    // 8. 显示路由规则
    println!("\n步骤8: 路由规则");
    let route_rules = config.get_route_rules();
    println!("发现 {} 条路由规则:", route_rules.len());
    
    for (i, rule) in route_rules.iter().enumerate() {
        println!("  规则 {}: action={}", i + 1, rule.action);
        if let Some(rule_sets) = &rule.rule_set {
            println!("    规则集: {:?}", rule_sets);
        }
        if let Some(domain_suffix) = &rule.domain_suffix {
            println!("    域名后缀: {:?}", domain_suffix);
        }
        if let Some(outbound) = &rule.outbound {
            println!("    出站: {}", outbound);
        }
    }

    // 9. 显示出站配置
    println!("\n步骤9: 出站配置");
    let outbounds = config.get_outbounds();
    println!("发现 {} 个出站:", outbounds.len());
    
    for outbound in outbounds {
        println!("  {}: {} -> {}:{}", 
            outbound.tag,
            outbound.outbound_type,
            outbound.server.as_ref().unwrap_or(&"localhost".to_string()),
            outbound.server_port.unwrap_or(0)
        );
    }

    // 10. 显示入站配置
    println!("\n步骤10: 入站配置");
    let inbounds = config.get_inbounds();
    println!("发现 {} 个入站:", inbounds.len());
    
    for inbound in inbounds {
        println!("  {}: {}:{}", 
            inbound.inbound_type,
            inbound.listen,
            inbound.listen_port
        );
    }

    println!("\n规则集下载演示完成！");
    println!("\n提示:");
    println!("  - 规则集已下载到: {}", cache_dir);
    println!("  - 下次运行时会检查更新，无变化则不重复下载");
    println!("  - 支持ETag和Last-Modified头进行变更检测");
    println!("  - 缓存文件超过24小时会自动重新下载");

    Ok(())
}
