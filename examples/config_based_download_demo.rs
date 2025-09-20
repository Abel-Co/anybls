// 基于配置文件的规则集下载演示程序
use anybls::ron_config::RonConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("基于配置文件的规则集下载演示程序");
    println!("================================");

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
                rule_set.url
            );
        } else {
            println!("  本地规则集: {}", rule_set.tag);
        }
    }

    // 3. 使用配置方法下载所有规则集
    println!("\n步骤3: 下载所有规则集");
    let downloader = config.download_rule_sets("cache/rule_sets").await?;
    println!("所有规则集下载完成");

    // 4. 显示缓存统计
    println!("\n步骤4: 缓存统计");
    let stats = downloader.get_cache_stats();
    println!("{}", stats);

    // 5. 检查下载的文件
    println!("\n步骤5: 检查下载的文件");
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

    // 6. 显示出站配置
    println!("\n步骤6: 出站配置");
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

    // 7. 显示入站配置
    println!("\n步骤7: 入站配置");
    let inbounds = config.get_inbounds();
    println!("发现 {} 个入站:", inbounds.len());
    
    for inbound in inbounds {
        println!("  {}: {}:{}", 
            inbound.inbound_type,
            inbound.listen,
            inbound.listen_port
        );
    }

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

    println!("\n基于配置文件的规则集下载演示完成！");
    println!("\n功能特点:");
    println!("  - 从配置文件动态解析规则集");
    println!("  - 支持HTTP/HTTPS下载");
    println!("  - 支持持久化缓存");
    println!("  - 支持缓存统计");
    println!("  - 与RON配置完全集成");

    Ok(())
}
