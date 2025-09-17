use crate::config::{RouterConfig, RouterRuleConfig};
use crate::error::{ProxyError, Result};
use ipnet::IpNet;
use regex::Regex;
use std::net::IpAddr;

pub struct CompiledRule {
    pub outbound: String,
    pub domain: Vec<String>,
    pub domain_suffix: Vec<String>,
    pub domain_keyword: Vec<String>,
    pub domain_regex: Vec<Regex>,
    pub ip_cidr: Vec<IpNet>,
}

pub struct Router {
    pub default_outbound: String,
    pub rules: Vec<CompiledRule>,
}

pub enum RouteDecision {
    PubOutbound(String),
}

impl Router {
    pub fn compile(cfg: &RouterConfig) -> Result<Self> {
        let mut compiled = Vec::new();
        for r in &cfg.rules {
            compiled.push(compile_rule(r)?);
        }
        Ok(Self { default_outbound: cfg.default_outbound.clone(), rules: compiled })
    }

    pub fn select_outbound_for_domain(&self, domain: &str) -> String {
        for r in &self.rules {
            // exact domain
            if r.domain.iter().any(|d| d.eq_ignore_ascii_case(domain)) { return r.outbound.clone(); }
            // suffix
            if r.domain_suffix.iter().any(|suf| domain.ends_with(suf)) { return r.outbound.clone(); }
            // keyword
            if r.domain_keyword.iter().any(|kw| domain.contains(kw)) { return r.outbound.clone(); }
            // regex
            if r.domain_regex.iter().any(|re| re.is_match(domain)) { return r.outbound.clone(); }
        }
        self.default_outbound.clone()
    }

    pub fn select_outbound_for_ip(&self, ip: IpAddr) -> String {
        for r in &self.rules {
            if r.ip_cidr.iter().any(|cidr| cidr.contains(&ip)) { return r.outbound.clone(); }
        }
        self.default_outbound.clone()
    }
}

static mut GLOBAL_ROUTER: Option<Router> = None;

pub fn init_global_router(cfg: &RouterConfig) -> Result<()> {
    let r = Router::compile(cfg)?;
    unsafe { GLOBAL_ROUTER = Some(r); }
    Ok(())
}

pub fn get_global_router() -> &'static Router {
    unsafe { GLOBAL_ROUTER.as_ref().expect("Router not initialized") }
}

fn compile_rule(rule: &RouterRuleConfig) -> Result<CompiledRule> {
    let mut regexes = Vec::new();
    for re_s in &rule.domains.domain_regex {
        let re = Regex::new(re_s).map_err(|e| ProxyError::Protocol(format!("Invalid domain_regex: {}", e)))?;
        regexes.push(re);
    }
    let mut cidrs = Vec::new();
    for c in &rule.ip_cidr {
        let net: IpNet = c.parse().map_err(|e| ProxyError::Protocol(format!("Invalid ip_cidr: {}", e)))?;
        cidrs.push(net);
    }
    Ok(CompiledRule {
        outbound: rule.outbound.clone(),
        domain: rule.domains.domain.clone(),
        domain_suffix: rule.domains.domain_suffix.clone(),
        domain_keyword: rule.domains.domain_keyword.clone(),
        domain_regex: regexes,
        ip_cidr: cidrs,
    })
}


