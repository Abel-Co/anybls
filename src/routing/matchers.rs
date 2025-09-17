// 高性能匹配器
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use fst::{Set, SetBuilder};
use aho_corasick::AhoCorasick;
use regex::RegexSet;
use ipnet::IpNet;
use radix_trie::Trie;
use crate::error::{ProxyError, Result};

/// 匹配结果
#[derive(Debug, Clone, PartialEq)]
pub enum MatcherResult {
    Match,
    NoMatch,
}

/// 域名匹配器 - 使用多种高性能算法
pub struct DomainMatcher {
    // 完整域名匹配 - FST Set
    exact_domains: Set<Vec<u8>>,
    
    // 域名后缀匹配 - 反向域名FST
    suffix_domains: Set<Vec<u8>>,
    
    // 关键字匹配 - AC自动机
    keyword_matcher: AhoCorasick,
    
    // 正则匹配 - RegexSet
    regex_matcher: RegexSet,
}

impl DomainMatcher {
    /// 创建新的域名匹配器
    pub fn new(
        exact_domains: Vec<String>,
        suffix_domains: Vec<String>,
        keyword_domains: Vec<String>,
        regex_domains: Vec<String>,
    ) -> Result<Self> {
        // 构建完整域名FST
        let mut exact_builder = SetBuilder::memory();
        for domain in &exact_domains {
            exact_builder.insert(domain)
                .map_err(|e| ProxyError::Protocol(format!("FST error: {}", e)))?;
        }
        let exact_domains = exact_builder.into_set();

        // 构建后缀域名FST（反向域名）
        let mut suffix_builder = SetBuilder::memory();
        for domain in &suffix_domains {
            let reversed = Self::reverse_domain(domain);
            suffix_builder.insert(&reversed)
                .map_err(|e| ProxyError::Protocol(format!("FST error: {}", e)))?;
        }
        let suffix_domains = suffix_builder.into_set();

        // 构建关键字AC自动机
        let keyword_matcher = AhoCorasick::new(&keyword_domains)
            .map_err(|e| ProxyError::Protocol(format!("AC error: {}", e)))?;

        // 构建正则表达式集合
        let regex_matcher = RegexSet::new(&regex_domains)
            .map_err(|e| crate::error::ProxyError::Protocol(format!("Invalid regex: {}", e)))?;

        Ok(Self {
            exact_domains,
            suffix_domains,
            keyword_matcher,
            regex_matcher,
        })
    }

    /// 匹配域名 - 按性能优化顺序
    pub fn matches(&self, domain: &str) -> MatcherResult {
        // 1. 完整域名匹配（最快）
        if self.exact_domains.contains(domain) {
            return MatcherResult::Match;
        }

        // 2. 后缀匹配
        let reversed = Self::reverse_domain(domain);
        if self.suffix_domains.contains(&reversed) {
            return MatcherResult::Match;
        }

        // 3. 关键字匹配
        if self.keyword_matcher.is_match(domain) {
            return MatcherResult::Match;
        }

        // 4. 正则匹配（最慢）
        if self.regex_matcher.is_match(domain) {
            return MatcherResult::Match;
        }

        MatcherResult::NoMatch
    }

    /// 反向域名（用于后缀匹配）
    fn reverse_domain(domain: &str) -> String {
        domain.split('.').rev().collect::<Vec<_>>().join(".")
    }
}

/// IP匹配器 - 使用radix_trie和HashMap
pub struct IpMatcher {
    ipv4_trie: Trie<u32, ()>,
    ipv6_networks: Vec<IpNet>, // IPv6使用简单的Vec，因为radix_trie不支持u128
}

impl IpMatcher {
    /// 创建新的IP匹配器
    pub fn new(ip_cidrs: Vec<String>) -> Result<Self> {
        let mut ipv4_trie = Trie::new();
        let mut ipv6_networks = Vec::new();

        for cidr_str in &ip_cidrs {
            let cidr: IpNet = cidr_str.parse()
                .map_err(|e| ProxyError::Protocol(format!("Invalid CIDR {}: {}", cidr_str, e)))?;

            match cidr {
                IpNet::V4(net) => {
                    // 将IPv4网络转换为前缀
                    let prefix = Self::ipv4_to_prefix(net.addr(), net.prefix_len());
                    ipv4_trie.insert(prefix, ());
                }
                IpNet::V6(_) => {
                    // IPv6直接存储网络
                    ipv6_networks.push(cidr);
                }
            }
        }

        Ok(Self {
            ipv4_trie,
            ipv6_networks,
        })
    }

    /// 匹配IP地址
    pub fn matches(&self, ip: IpAddr) -> MatcherResult {
        match ip {
            IpAddr::V4(ipv4) => {
                let prefix = Self::ipv4_to_prefix(ipv4, 32);
                if self.ipv4_trie.get_ancestor(&prefix).is_some() {
                    MatcherResult::Match
                } else {
                    MatcherResult::NoMatch
                }
            }
            IpAddr::V6(ipv6) => {
                // IPv6使用简单的线性搜索
                for network in &self.ipv6_networks {
                    if let IpNet::V6(net) = network {
                        if net.contains(&ipv6) {
                            return MatcherResult::Match;
                        }
                    }
                }
                MatcherResult::NoMatch
            }
        }
    }

    /// 将IPv4地址和前缀长度转换为前缀
    fn ipv4_to_prefix(addr: std::net::Ipv4Addr, prefix_len: u8) -> u32 {
        let ip = u32::from(addr);
        let mask = if prefix_len == 0 {
            0
        } else {
            !((1u32 << (32 - prefix_len)) - 1)
        };
        ip & mask
    }

}

/// 匹配器缓存
pub struct MatcherCache {
    domain_matchers: HashMap<String, Arc<DomainMatcher>>,
    ip_matchers: HashMap<String, Arc<IpMatcher>>,
}

impl MatcherCache {
    pub fn new() -> Self {
        Self {
            domain_matchers: HashMap::new(),
            ip_matchers: HashMap::new(),
        }
    }

    /// 获取或创建域名匹配器
    pub fn get_domain_matcher(
        &mut self,
        key: &str,
        exact_domains: Vec<String>,
        suffix_domains: Vec<String>,
        keyword_domains: Vec<String>,
        regex_domains: Vec<String>,
    ) -> Result<Arc<DomainMatcher>> {
        if let Some(matcher) = self.domain_matchers.get(key) {
            return Ok(matcher.clone());
        }

        let matcher = Arc::new(DomainMatcher::new(
            exact_domains,
            suffix_domains,
            keyword_domains,
            regex_domains,
        )?);
        
        self.domain_matchers.insert(key.to_string(), matcher.clone());
        Ok(matcher)
    }

    /// 获取或创建IP匹配器
    pub fn get_ip_matcher(
        &mut self,
        key: &str,
        ip_cidrs: Vec<String>,
    ) -> Result<Arc<IpMatcher>> {
        if let Some(matcher) = self.ip_matchers.get(key) {
            return Ok(matcher.clone());
        }

        let matcher = Arc::new(IpMatcher::new(ip_cidrs)?);
        self.ip_matchers.insert(key.to_string(), matcher.clone());
        Ok(matcher)
    }
}

impl Default for MatcherCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_matcher() {
        let matcher = DomainMatcher::new(
            vec!["example.com".to_string()],
            vec!["google.com".to_string()],
            vec!["test".to_string()],
            vec![r"^test.*\.com$".to_string()],
        ).unwrap();

        assert_eq!(matcher.matches("example.com"), MatcherResult::Match);
        assert_eq!(matcher.matches("www.google.com"), MatcherResult::Match);
        assert_eq!(matcher.matches("test.example.com"), MatcherResult::Match);
        assert_eq!(matcher.matches("test123.com"), MatcherResult::Match);
        assert_eq!(matcher.matches("other.com"), MatcherResult::NoMatch);
    }

    #[test]
    fn test_ip_matcher() {
        let matcher = IpMatcher::new(vec![
            "192.168.1.0/24".to_string(),
            "10.0.0.0/8".to_string(),
        ]).unwrap();

        assert_eq!(matcher.matches("192.168.1.1".parse().unwrap()), MatcherResult::Match);
        assert_eq!(matcher.matches("10.1.1.1".parse().unwrap()), MatcherResult::Match);
        assert_eq!(matcher.matches("8.8.8.8".parse().unwrap()), MatcherResult::NoMatch);
    }
}
