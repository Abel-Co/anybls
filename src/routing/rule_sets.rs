// 规则集合数据结构
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use ipnet::IpNet;
use crate::error::Result;

/// 规则集合ID - 用于缓存和引用
pub type RuleSetId = String;

/// 域名规则集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRuleSet {
    pub id: RuleSetId,
    pub domain: Vec<String>,           // 完整域名匹配
    pub domain_suffix: Vec<String>,    // 域名后缀匹配
    pub domain_keyword: Vec<String>,   // 域名关键字匹配
    pub domain_regex: Vec<String>,     // 正则表达式匹配
}

/// IP规则集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRuleSet {
    pub id: RuleSetId,
    pub ip_cidr: Vec<String>,          // IP-CIDR列表
}

/// 规则集合枚举
#[derive(Debug, Clone)]
pub enum RuleSet {
    Domain(DomainRuleSet),
    Ip(IpRuleSet),
}

impl RuleSet {
    pub fn id(&self) -> &RuleSetId {
        match self {
            RuleSet::Domain(d) => &d.id,
            RuleSet::Ip(i) => &i.id,
        }
    }
}

/// 规则集合管理器
pub struct RuleSetManager {
    domain_sets: HashMap<RuleSetId, DomainRuleSet>,
    ip_sets: HashMap<RuleSetId, IpRuleSet>,
}

impl RuleSetManager {
    pub fn new() -> Self {
        Self {
            domain_sets: HashMap::new(),
            ip_sets: HashMap::new(),
        }
    }

    /// 添加域名规则集合
    pub fn add_domain_set(&mut self, set: DomainRuleSet) {
        self.domain_sets.insert(set.id.clone(), set);
    }

    /// 添加IP规则集合
    pub fn add_ip_set(&mut self, set: IpRuleSet) {
        self.ip_sets.insert(set.id.clone(), set);
    }

    /// 获取域名规则集合
    pub fn get_domain_set(&self, id: &RuleSetId) -> Option<&DomainRuleSet> {
        self.domain_sets.get(id)
    }

    /// 获取IP规则集合
    pub fn get_ip_set(&self, id: &RuleSetId) -> Option<&IpRuleSet> {
        self.ip_sets.get(id)
    }

    /// 获取所有域名规则集合
    pub fn all_domain_sets(&self) -> &HashMap<RuleSetId, DomainRuleSet> {
        &self.domain_sets
    }

    /// 获取所有IP规则集合
    pub fn all_ip_sets(&self) -> &HashMap<RuleSetId, IpRuleSet> {
        &self.ip_sets
    }
}

/// 从JSON资源文件加载规则集合
impl RuleSetManager {
    /// 从域名JSON文件加载
    pub fn load_domain_from_json(&mut self, json_content: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct DomainJsonFile {
            rules: Vec<DomainRuleSet>,
        }

        let file: DomainJsonFile = serde_json::from_str(json_content)
            .map_err(|e| crate::error::ProxyError::Protocol(format!("Invalid domain JSON: {}", e)))?;

        for rule in file.rules {
            self.add_domain_set(rule);
        }

        Ok(())
    }

    /// 从IP JSON文件加载
    pub fn load_ip_from_json(&mut self, json_content: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct IpJsonFile {
            rules: Vec<IpRuleSet>,
        }

        let file: IpJsonFile = serde_json::from_str(json_content)
            .map_err(|e| crate::error::ProxyError::Protocol(format!("Invalid IP JSON: {}", e)))?;

        for rule in file.rules {
            self.add_ip_set(rule);
        }

        Ok(())
    }
}

impl Default for RuleSetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_set_manager() {
        let mut manager = RuleSetManager::new();
        
        let domain_set = DomainRuleSet {
            id: "test_domain".to_string(),
            domain: vec!["example.com".to_string()],
            domain_suffix: vec!["google.com".to_string()],
            domain_keyword: vec!["test".to_string()],
            domain_regex: vec![r"^test.*\.com$".to_string()],
        };
        
        manager.add_domain_set(domain_set);
        
        assert!(manager.get_domain_set(&"test_domain".to_string()).is_some());
        assert!(manager.get_domain_set(&"nonexistent".to_string()).is_none());
    }
}
