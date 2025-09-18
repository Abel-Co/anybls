// RON配置演示程序
use anybls::ron_config::RonConfig;
use anybls::protocols::{Protocol, DirectProtocol, Socks5Protocol, TproxyProtocol};
use anybls::routing::{HighPerformanceRouter, RouteRule};
use anybls::routing::rule_sets::{RuleSetManager, DomainRuleSet, IpRuleSet};
use anybls::inbound::ProtocolInbound;
use anybls::outbound::OutboundManager;
use std::net::SocketAddr;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();

    println!("RON配置演示程序");
    println!("================");

    // 加载RON配置
    let ron_config = RonConfig::from_ron_file("examples/simple_config.ron")?;
    println!("✅ RON配置文件加载成功");

    // 转换为内部配置
    let internal_config = ron_config.to_internal_config()?;
    println!("✅ 配置转换成功");

    // 显示配置信息
    println!("\n📋 配置信息:");
    println!("入站配置数量: {}", ron_config.inbounds.len());
    for inbound in &ron_config.inbounds {
        println!("  - {}: {}:{}", inbound.inbound_type, inbound.listen, inbound.listen_port);
    }

    println!("出站配置数量: {}", internal_config.outbounds.len());
    for outbound in &internal_config.outbounds {
        println!("  - {}: {:?}", outbound.name, outbound.kind);
    }

    println!("路由规则数量: {}", internal_config.high_performance_router.rules.len());
    for rule in &internal_config.high_performance_router.rules {
        println!("  - 规则集: {:?}, 出站: {}", rule.rule_sets, rule.outbound);
    }

    // 创建出站管理器
    let outbound_manager = OutboundManager::from_configs(&internal_config.outbounds)?;
    println!("✅ 出站管理器创建成功");

    // 创建规则集合管理器
    let mut rule_manager = RuleSetManager::new();
    
    // 添加一些示例规则集合
    let domain_set = DomainRuleSet {
        id: "GeoSite-CN".to_string(),
        domain: vec!["baidu.com".to_string(), "qq.com".to_string()],
        domain_suffix: vec![".cn".to_string(), ".com.cn".to_string()],
        domain_keyword: vec!["baidu".to_string(), "tencent".to_string()],
        domain_regex: vec![r".*\.cn$".to_string()],
    };
    rule_manager.add_domain_set(domain_set);

    let ip_set = IpRuleSet {
        id: "GeoIP-CN".to_string(),
        ip_cidr: vec!["1.0.0.0/8".to_string(), "14.0.0.0/8".to_string()],
    };
    rule_manager.add_ip_set(ip_set);

    println!("✅ 规则集合管理器创建成功");

    // 创建高性能路由器
    let mut router = HighPerformanceRouter::new(
        internal_config.high_performance_router.default_outbound.clone()
    );
    router.set_rule_manager(rule_manager);

    // 添加路由规则
    for rule_config in &internal_config.high_performance_router.rules {
        let route_rule = RouteRule {
            rule_sets: rule_config.rule_sets.clone(),
            outbound: rule_config.outbound.clone(),
        };
        router.add_rule(route_rule);
    }

    println!("✅ 高性能路由器创建成功");

    // 测试路由功能
    println!("\n🧪 路由测试:");
    let test_domains = vec![
        "baidu.com",
        "www.qq.com",
        "google.com",
        "github.com",
        "example.cn",
    ];

    for domain in test_domains {
        let outbound = router.select_outbound_for_domain(domain);
        println!("  {} -> {}", domain, outbound);
    }

    // 测试IP路由
    let test_ips = vec![
        "1.1.1.1",
        "8.8.8.8",
        "14.0.0.1",
        "192.168.1.1",
    ];

    println!("\n🌐 IP路由测试:");
    for ip_str in test_ips {
        let ip: std::net::IpAddr = ip_str.parse()?;
        let outbound = router.select_outbound_for_ip(ip);
        println!("  {} -> {}", ip_str, outbound);
    }

    // 创建入站监听器（模拟）
    println!("\n🔌 入站监听器创建:");
    for inbound_config in &ron_config.inbounds {
        let bind_addr: SocketAddr = format!("{}:{}", inbound_config.listen, inbound_config.listen_port).parse()?;
        match inbound_config.inbound_type.as_str() {
            "tproxy" => {
                println!("  TProxy监听器: {} (模拟)", bind_addr);
                
                // 在实际应用中，这里会创建真正的TProxy监听器
                // let tproxy = TproxyProtocol::new();
                // let inbound = ProtocolInbound::new(Box::new(tproxy), bind_addr);
                // inbound.start().await?;
            },
            "socks" => {
                println!("  SOCKS5监听器: {} (模拟)", bind_addr);
                
                // 在实际应用中，这里会创建真正的SOCKS5监听器
                // let socks5 = Socks5Protocol::new();
                // let inbound = ProtocolInbound::new(Box::new(socks5), bind_addr);
                // inbound.start().await?;
            },
            _ => {
                println!("  未知入站类型: {}: {} (跳过)", inbound_config.inbound_type, bind_addr);
            }
        }
    }

    // 显示缓存统计
    println!("\n📊 缓存统计:");
    let stats = router.get_cache_stats();
    println!("  域名缓存: {} 条目", stats.domain_cache_size);
    println!("  IP缓存: {} 条目", stats.ip_cache_size);
    println!("  总缓存: {} 条目", stats.total_size);

    // 性能测试
    println!("\n⚡ 性能测试:");
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        router.select_outbound_for_domain("baidu.com");
        router.select_outbound_for_domain("google.com");
        router.select_outbound_for_domain("github.com");
    }
    let duration = start.elapsed();
    println!("  10000次域名匹配耗时: {:?}", duration);
    println!("  平均每次: {:?}", duration / 30000);

    println!("\n✅ 演示完成！");
    println!("\n💡 下一步:");
    println!("  1. 实现真正的入站监听器启动");
    println!("  2. 实现出站连接的实际建立");
    println!("  3. 添加规则集合文件的动态加载");
    println!("  4. 实现完整的流量转发逻辑");

    Ok(())
}
