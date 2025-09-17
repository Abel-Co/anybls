// 匹配结果缓存
use crate::routing::matchers::MatcherResult;
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;

/// 缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    Domain(String),
    Ip(IpAddr),
}

/// 匹配结果缓存
pub struct MatchCache {
    domain_cache: HashMap<String, MatcherResult>,
    ip_cache: HashMap<IpAddr, MatcherResult>,
    max_size: usize,
}

impl MatchCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            domain_cache: HashMap::new(),
            ip_cache: HashMap::new(),
            max_size,
        }
    }

    /// 获取域名匹配结果
    pub fn get_domain(&self, domain: &str) -> Option<&MatcherResult> {
        self.domain_cache.get(domain)
    }

    /// 设置域名匹配结果
    pub fn set_domain(&mut self, domain: String, result: MatcherResult) {
        if self.domain_cache.len() >= self.max_size {
            // 简单的LRU策略：清除一半缓存
            let to_remove = self.domain_cache.len() / 2;
            let keys: Vec<String> = self.domain_cache.keys().take(to_remove).cloned().collect();
            for key in keys {
                self.domain_cache.remove(&key);
            }
        }
        self.domain_cache.insert(domain, result);
    }

    /// 获取IP匹配结果
    pub fn get_ip(&self, ip: &IpAddr) -> Option<&MatcherResult> {
        self.ip_cache.get(ip)
    }

    /// 设置IP匹配结果
    pub fn set_ip(&mut self, ip: IpAddr, result: MatcherResult) {
        if self.ip_cache.len() >= self.max_size {
            // 简单的LRU策略：清除一半缓存
            let to_remove = self.ip_cache.len() / 2;
            let keys: Vec<IpAddr> = self.ip_cache.keys().take(to_remove).cloned().collect();
            for key in keys {
                self.ip_cache.remove(&key);
            }
        }
        self.ip_cache.insert(ip, result);
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.domain_cache.clear();
        self.ip_cache.clear();
    }

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            domain_cache_size: self.domain_cache.len(),
            ip_cache_size: self.ip_cache.len(),
            total_size: self.domain_cache.len() + self.ip_cache.len(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub domain_cache_size: usize,
    pub ip_cache_size: usize,
    pub total_size: usize,
}

impl Default for MatchCache {
    fn default() -> Self {
        Self::new(10000) // 默认最大10000个条目
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_domain_cache() {
        let mut cache = MatchCache::new(100);

        cache.set_domain("example.com".to_string(), MatcherResult::Match);
        assert_eq!(cache.get_domain("example.com"), Some(&MatcherResult::Match));
        assert_eq!(cache.get_domain("other.com"), None);
    }

    #[test]
    fn test_ip_cache() {
        let mut cache = MatchCache::new(100);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        cache.set_ip(ip, MatcherResult::Match);
        assert_eq!(cache.get_ip(&ip), Some(&MatcherResult::Match));
    }
}
