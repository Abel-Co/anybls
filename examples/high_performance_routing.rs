// 高性能路由示例
use anybls::routing::{
    HighPerformanceRouter, RouteRule
};
use anybls::routing::rule_sets::{RuleSetManager, DomainRuleSet, IpRuleSet};
use anybls::routing::cache::CacheStats;
use std::net::IpAddr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("高性能路由系统示例");

    // 创建路由器
    let mut router = HighPerformanceRouter::new("direct".to_string());

    // 创建规则集合管理器
    let mut rule_manager = RuleSetManager::new();

    // 添加域名规则集合
    let google_domains = DomainRuleSet {
        id: "google_domains".to_string(),
        domain: vec!["google.com".to_string()],
        domain_suffix: vec!["google.com".to_string(), "youtube.com".to_string()],
        domain_keyword: vec!["google".to_string(), "youtube".to_string()],
        domain_regex: vec![r"^.*\.google\.com$".to_string()],
    };
    rule_manager.add_domain_set(google_domains);

    // 添加IP规则集合
    let google_ips = IpRuleSet {
        id: "google_ips".to_string(),
        ip_cidr: vec!["8.8.8.0/24".to_string(), "8.8.4.0/24".to_string()],
    };
    rule_manager.add_ip_set(google_ips);

    // 设置规则管理器
    router.set_rule_manager(rule_manager);

    // 添加路由规则
    let google_rule = RouteRule {
        rule_sets: vec!["google_domains".to_string(), "google_ips".to_string()],
        outbound: "proxy".to_string(),
    };
    router.add_rule(google_rule);

    // 测试域名匹配
    println!("\n=== 域名匹配测试 ===");
    test_domain_matching(&router);

    // 测试IP匹配
    println!("\n=== IP匹配测试 ===");
    test_ip_matching(&router);

    // 性能测试
    println!("\n=== 性能测试 ===");
    performance_test(&router);

    // 缓存统计
    println!("\n=== 缓存统计 ===");
    let stats = router.get_cache_stats();
    println!("域名缓存: {} 条目", stats.domain_cache_size);
    println!("IP缓存: {} 条目", stats.ip_cache_size);
    println!("总缓存: {} 条目", stats.total_size);

    Ok(())
}

fn test_domain_matching(router: &HighPerformanceRouter) {
    let test_domains = vec![
        "google.com",
        "www.google.com",
        "youtube.com",
        "www.youtube.com",
        "googleapis.com",
        "other.com",
        "example.com",
    ];

    for domain in test_domains {
        let outbound = router.select_outbound_for_domain(domain);
        println!("{} -> {}", domain, outbound);
    }
}

fn test_ip_matching(router: &HighPerformanceRouter) {
    let test_ips = vec![
        "8.8.8.8",
        "8.8.4.4",
        "1.1.1.1",
        "192.168.1.1",
        "10.0.0.1",
    ];

    for ip_str in test_ips {
        let ip: IpAddr = ip_str.parse().unwrap();
        let outbound = router.select_outbound_for_ip(ip);
        println!("{} -> {}", ip_str, outbound);
    }
}

fn performance_test(router: &HighPerformanceRouter) {
    use std::time::Instant;

    // 域名性能测试
    let test_domains = vec![
        "google.com", "www.google.com", "youtube.com", 
        "other.com", "example.com", "test.com"
    ];
    
    let iterations = 10000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        for domain in &test_domains {
            let _ = router.select_outbound_for_domain(domain);
        }
    }
    
    let duration = start.elapsed();
    let total_operations = iterations * test_domains.len();
    let ops_per_sec = total_operations as f64 / duration.as_secs_f64();
    
    println!("域名匹配性能: {:.0} ops/sec", ops_per_sec);
    println!("平均延迟: {:.2} μs", duration.as_micros() as f64 / total_operations as f64);

    // IP性能测试
    let test_ips: Vec<IpAddr> = vec![
        "8.8.8.8".parse().unwrap(),
        "8.8.4.4".parse().unwrap(),
        "1.1.1.1".parse().unwrap(),
        "192.168.1.1".parse().unwrap(),
    ];
    
    let start = Instant::now();
    
    for _ in 0..iterations {
        for ip in &test_ips {
            let _ = router.select_outbound_for_ip(*ip);
        }
    }
    
    let duration = start.elapsed();
    let total_operations = iterations * test_ips.len();
    let ops_per_sec = total_operations as f64 / duration.as_secs_f64();
    
    println!("IP匹配性能: {:.0} ops/sec", ops_per_sec);
    println!("平均延迟: {:.2} μs", duration.as_micros() as f64 / total_operations as f64);
}
