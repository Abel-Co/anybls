// 高性能路由器
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use crate::routing::{
    rule_sets::{RuleSetManager, RuleSetId},
    matchers::{MatcherResult, MatcherCache},
    cache::{MatchCache, CacheStats},
};

/// 路由规则
#[derive(Debug, Clone)]
pub struct RouteRule {
    pub rule_sets: Vec<RuleSetId>,  // 规则集合ID列表（OR关系）
    pub outbound: String,           // 出站名称
}

/// 高性能路由器
pub struct HighPerformanceRouter {
    rule_manager: RuleSetManager,
    matcher_cache: Arc<RwLock<MatcherCache>>,
    match_cache: Arc<RwLock<MatchCache>>,
    rules: Vec<RouteRule>,
    default_outbound: String,
}

impl HighPerformanceRouter {
    /// 创建新的路由器
    pub fn new(default_outbound: String) -> Self {
        Self {
            rule_manager: RuleSetManager::new(),
            matcher_cache: Arc::new(RwLock::new(MatcherCache::new())),
            match_cache: Arc::new(RwLock::new(MatchCache::new(10000))),
            rules: Vec::new(),
            default_outbound,
        }
    }

    /// 添加路由规则
    pub fn add_rule(&mut self, rule: RouteRule) {
        self.rules.push(rule);
    }

    /// 设置规则集合管理器
    pub fn set_rule_manager(&mut self, manager: RuleSetManager) {
        self.rule_manager = manager;
    }

    /// 选择出站 - 域名匹配
    pub fn select_outbound_for_domain(&self, domain: &str) -> String {
        // 检查缓存
        if let Some(cached_result) = self.match_cache.read().unwrap().get_domain(domain) {
            if *cached_result == MatcherResult::Match {
                return self.find_matching_outbound_for_domain(domain);
            }
        }

        // 遍历规则
        for rule in &self.rules {
            if self.matches_domain_rule(domain, rule) {
                // 缓存匹配结果
                self.match_cache.write().unwrap().set_domain(domain.to_string(), MatcherResult::Match);
                return rule.outbound.clone();
            }
        }

        // 缓存未匹配结果
        self.match_cache.write().unwrap().set_domain(domain.to_string(), MatcherResult::NoMatch);
        self.default_outbound.clone()
    }

    /// 选择出站 - IP匹配
    pub fn select_outbound_for_ip(&self, ip: IpAddr) -> String {
        // 检查缓存
        if let Some(cached_result) = self.match_cache.read().unwrap().get_ip(&ip) {
            if *cached_result == MatcherResult::Match {
                return self.find_matching_outbound_for_ip(ip);
            }
        }

        // 遍历规则
        for rule in &self.rules {
            if self.matches_ip_rule(ip, rule) {
                // 缓存匹配结果
                self.match_cache.write().unwrap().set_ip(ip, MatcherResult::Match);
                return rule.outbound.clone();
            }
        }

        // 缓存未匹配结果
        self.match_cache.write().unwrap().set_ip(ip, MatcherResult::NoMatch);
        self.default_outbound.clone()
    }

    /// 检查域名是否匹配规则
    fn matches_domain_rule(&self, domain: &str, rule: &RouteRule) -> bool {
        for rule_set_id in &rule.rule_sets {
            if let Some(domain_set) = self.rule_manager.get_domain_set(rule_set_id) {
                if self.matches_domain_set(domain, domain_set) {
                    return true;
                }
            }
        }
        false
    }

    /// 检查IP是否匹配规则
    fn matches_ip_rule(&self, ip: IpAddr, rule: &RouteRule) -> bool {
        for rule_set_id in &rule.rule_sets {
            if let Some(ip_set) = self.rule_manager.get_ip_set(rule_set_id) {
                if self.matches_ip_set(ip, ip_set) {
                    return true;
                }
            }
        }
        false
    }

    /// 匹配域名集合
    fn matches_domain_set(&self, domain: &str, domain_set: &crate::routing::rule_sets::DomainRuleSet) -> bool {
        // 获取或创建匹配器
        let matcher = {
            let mut cache = self.matcher_cache.write().unwrap();
            cache.get_domain_matcher(
                &domain_set.id,
                domain_set.domain.clone(),
                domain_set.domain_suffix.clone(),
                domain_set.domain_keyword.clone(),
                domain_set.domain_regex.clone(),
            ).unwrap()
        };

        matcher.matches(domain) == MatcherResult::Match
    }

    /// 匹配IP集合
    fn matches_ip_set(&self, ip: IpAddr, ip_set: &crate::routing::rule_sets::IpRuleSet) -> bool {
        // 获取或创建匹配器
        let matcher = {
            let mut cache = self.matcher_cache.write().unwrap();
            cache.get_ip_matcher(&ip_set.id, ip_set.ip_cidr.clone()).unwrap()
        };

        matcher.matches(ip) == MatcherResult::Match
    }

    /// 查找匹配的出站（用于缓存命中时）
    fn find_matching_outbound_for_domain(&self, domain: &str) -> String {
        for rule in &self.rules {
            if self.matches_domain_rule(domain, rule) {
                return rule.outbound.clone();
            }
        }
        self.default_outbound.clone()
    }

    /// 查找匹配的出站（用于缓存命中时）
    fn find_matching_outbound_for_ip(&self, ip: IpAddr) -> String {
        for rule in &self.rules {
            if self.matches_ip_rule(ip, rule) {
                return rule.outbound.clone();
            }
        }
        self.default_outbound.clone()
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> CacheStats {
        self.match_cache.read().unwrap().stats()
    }

    /// 清除缓存
    pub fn clear_cache(&self) {
        self.match_cache.write().unwrap().clear();
    }

    /// 获取规则数量
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// 获取规则集合数量
    pub fn rule_set_count(&self) -> usize {
        self.rule_manager.all_domain_sets().len() + self.rule_manager.all_ip_sets().len()
    }
}

impl Default for HighPerformanceRouter {
    fn default() -> Self {
        Self::new("direct".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::rule_sets::{DomainRuleSet, IpRuleSet};

    #[test]
    fn test_router_domain_matching() {
        let mut router = HighPerformanceRouter::new("direct".to_string());
        
        // 添加域名规则集合
        let domain_set = DomainRuleSet {
            id: "google_domains".to_string(),
            domain: vec!["google.com".to_string()],
            domain_suffix: vec!["google.com".to_string()],
            domain_keyword: vec!["google".to_string()],
            domain_regex: vec![],
        };
        router.rule_manager.add_domain_set(domain_set);

        // 添加路由规则
        let rule = RouteRule {
            rule_sets: vec!["google_domains".to_string()],
            outbound: "proxy".to_string(),
        };
        router.add_rule(rule);

        // 测试匹配
        assert_eq!(router.select_outbound_for_domain("google.com"), "proxy");
        assert_eq!(router.select_outbound_for_domain("www.google.com"), "proxy");
        assert_eq!(router.select_outbound_for_domain("other.com"), "direct");
    }

    #[test]
    fn test_router_ip_matching() {
        let mut router = HighPerformanceRouter::new("direct".to_string());
        
        // 添加IP规则集合
        let ip_set = IpRuleSet {
            id: "private_ips".to_string(),
            ip_cidr: vec!["192.168.0.0/16".to_string(), "10.0.0.0/8".to_string()],
        };
        router.rule_manager.add_ip_set(ip_set);

        // 添加路由规则
        let rule = RouteRule {
            rule_sets: vec!["private_ips".to_string()],
            outbound: "direct".to_string(),
        };
        router.add_rule(rule);

        // 测试匹配
        assert_eq!(router.select_outbound_for_ip("192.168.1.1".parse().unwrap()), "direct");
        assert_eq!(router.select_outbound_for_ip("10.1.1.1".parse().unwrap()), "direct");
        assert_eq!(router.select_outbound_for_ip("8.8.8.8".parse().unwrap()), "direct");
    }
}
