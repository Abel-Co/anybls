// RON配置文件支持
use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::rule_set_downloader::RuleSetDownloader;
use std::path::Path;

/// RON配置根结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RonConfig {
    pub log: Option<LogConfig>,
    pub experimental: Option<ExperimentalConfig>,
    pub dns: Option<DnsConfig>,
    pub inbounds: Vec<InboundConfig>,
    pub outbounds: Vec<OutboundConfig>,
    pub route: RouteConfig,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub disabled: bool,
    pub timestamp: bool,
    pub level: String,
}

/// 实验性功能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalConfig {
    pub clash_api: Option<ClashApiConfig>,
    pub cache_file: Option<CacheFileConfig>,
}

/// Clash API配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashApiConfig {
    pub external_controller: String,
    pub external_ui: String,
    pub external_ui_download_url: String,
    pub external_ui_download_detour: String,
    pub secret: String,
    pub default_mode: String,
    pub access_control_allow_origin: Vec<String>,
    pub access_control_allow_private_network: bool,
}

/// 缓存文件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheFileConfig {
    pub enabled: bool,
    pub path: String,
    pub cache_id: String,
    pub store_fakeip: bool,
    pub store_rdrc: bool,
    pub rdrc_timeout: String,
}

/// DNS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub servers: Vec<DnsServer>,
    pub strategy: String,
    pub r#final: String,
}

/// DNS服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServer {
    pub tag: String,
    #[serde(rename = "type")]
    pub server_type: String,
    pub server: String,
    pub domain_resolver: Option<String>,
    pub detour: Option<String>,
}

/// 入站配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundConfig {
    #[serde(rename = "type")]
    pub inbound_type: String,
    pub listen: String,
    pub listen_port: u16,
    pub tcp_fast_open: Option<bool>,
    pub tcp_multi_path: Option<bool>,
    pub udp_fragment: Option<bool>,
    pub udp_timeout: Option<String>,
    pub sniff: Option<bool>,
}

/// 出站配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundConfig {
    pub tag: String,
    #[serde(rename = "type")]
    pub outbound_type: String,
    pub server: Option<String>,
    pub server_port: Option<u16>,
    pub password: Option<String>,
    pub uuid: Option<String>,
    pub flow: Option<String>,
    pub packet_encoding: Option<String>,
    pub routing_mark: Option<u32>,
    pub url: Option<String>,
    pub interval: Option<String>,
    pub tolerance: Option<u32>,
    pub interrupt_exist_connections: Option<bool>,
    pub outbounds: Option<Vec<String>>,
    pub tls: Option<TlsConfig>,
    pub transport: Option<TransportConfig>,
}

/// TLS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub disable_sni: Option<bool>,
    pub server_name: Option<String>,
    pub insecure: Option<bool>,
    pub alpn: Option<Vec<String>>,
    pub utls: Option<UtlsConfig>,
    pub reality: Option<RealityConfig>,
}

/// uTLS配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtlsConfig {
    pub enabled: bool,
    pub fingerprint: String,
}

/// Reality配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityConfig {
    pub enabled: bool,
    pub public_key: String,
    pub short_id: String,
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    #[serde(rename = "type")]
    pub transport_type: String,
    pub service_name: Option<String>,
    pub idle_timeout: Option<String>,
    pub ping_timeout: Option<String>,
    pub permit_without_stream: Option<bool>,
}

/// 路由配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub rules: Vec<RouteRule>,
    pub rule_set: Vec<RuleSetConfig>,
    pub default_domain_resolver: Option<String>,
    pub auto_detect_interface: Option<bool>,
    pub r#final: String,
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub action: String,
    pub protocol: Option<String>,
    pub rule_set: Option<Vec<String>>,
    pub domain_suffix: Option<Vec<String>>,
    pub outbound: Option<String>,
}

/// 规则集合配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSetConfig {
    pub tag: String,
    #[serde(rename = "type")]
    pub rule_set_type: String,
    pub url: String,
    pub format: String,
    pub download_detour: Option<String>,
}

impl RonConfig {
    /// 从RON文件加载配置
    pub fn from_ron_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::ProxyError::Io(e))?;
        
        let config: RonConfig = ron::from_str(&content)
            .map_err(|e| crate::error::ProxyError::Protocol(format!("Invalid RON config: {}", e)))?;
        
        Ok(config)
    }

    /// 获取入站配置
    pub fn get_inbounds(&self) -> &Vec<InboundConfig> {
        &self.inbounds
    }

    /// 获取出站配置
    pub fn get_outbounds(&self) -> &Vec<OutboundConfig> {
        &self.outbounds
    }

    /// 获取路由规则
    pub fn get_route_rules(&self) -> &Vec<RouteRule> {
        &self.route.rules
    }

    /// 获取默认出站
    pub fn get_default_outbound(&self) -> &String {
        &self.route.r#final
    }

    /// 下载所有远程规则集
    pub async fn download_rule_sets(&self, cache_dir: impl AsRef<Path>) -> Result<RuleSetDownloader> {
        let mut downloader = RuleSetDownloader::new(cache_dir)?;
        
        for rule_set in &self.route.rule_set {
            if rule_set.rule_set_type == "remote" {
                println!("准备下载规则集: {} -> {}", rule_set.tag, rule_set.url);
                downloader.download_rule_set(&rule_set.tag, &rule_set.url).await?;
            }
        }
        
        Ok(downloader)
    }

    /// 获取规则集配置
    pub fn get_rule_sets(&self) -> &Vec<RuleSetConfig> {
        &self.route.rule_set
    }

    /// 转换为我们的内部配置格式
    pub fn to_internal_config(&self) -> Result<crate::config::Config> {
        // 转换出站配置
        let mut outbounds = Vec::new();
        for outbound in &self.outbounds {
            let internal_outbound = match outbound.outbound_type.as_str() {
                "direct" => crate::config::OutboundConfig {
                    name: outbound.tag.clone(),
                    kind: crate::config::OutboundType::Direct,
                },
                "socks" => {
                    let server_addr = format!("{}:{}", 
                        outbound.server.as_ref().unwrap_or(&"127.0.0.1".to_string()),
                        outbound.server_port.unwrap_or(1080)
                    );
                    crate::config::OutboundConfig {
                        name: outbound.tag.clone(),
                        kind: crate::config::OutboundType::Socks5 { address: server_addr },
                    }
                },
                "vless" => {
                    let server_addr = format!("{}:{}", 
                        outbound.server.as_ref().unwrap_or(&"127.0.0.1".to_string()),
                        outbound.server_port.unwrap_or(443)
                    );
                    crate::config::OutboundConfig {
                        name: outbound.tag.clone(),
                        kind: crate::config::OutboundType::Vless {
                            address: server_addr,
                            uuid: outbound.uuid.clone().unwrap_or_default(),
                            tls: outbound.tls.as_ref().map_or(false, |t| t.enabled),
                        },
                    }
                },
                _ => continue,
            };
            outbounds.push(internal_outbound);
        }

        // 转换路由规则
        let mut rules = Vec::new();
        for rule in &self.route.rules {
            if let Some(outbound) = &rule.outbound {
                let mut rule_sets = Vec::new();
                if let Some(sets) = &rule.rule_set {
                    rule_sets = sets.clone();
                }

                let internal_rule = crate::config::HighPerformanceRouteRule {
                    rule_sets,
                    outbound: outbound.clone(),
                };
                rules.push(internal_rule);
            }
        }

        let internal_config = crate::config::Config {
            server: crate::config::ServerConfig {
                host: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
                port: 1080,
                max_connections: 1000,
                connection_timeout_secs: 30,
                keep_alive_timeout_secs: 60,
            },
            connection_pool: crate::config::ConnectionPoolConfig {
                max_connections_per_target: 10,
                max_total_connections: 1000,
                connection_timeout_secs: 30,
                idle_timeout_secs: 300,
                cleanup_interval_secs: 60,
            },
            dns: crate::config::DnsConfig {
                servers: Vec::new(),
                timeout_secs: 5,
                enable_ipv6: true,
                cache_ttl_secs: 300,
            },
            logging: crate::config::LoggingConfig {
                level: "info".to_string(),
                structured: false,
                file: None,
                enable_metrics: true,
            },
            performance: crate::config::PerformanceConfig {
                buffer_size: 65536,
                tcp_nodelay: true,
                reuse_addr: true,
                keep_alive: true,
                worker_threads: 0,
            },
            traffic_mark: crate::config::TrafficMarkConfig::default(),
            outbounds,
            router: crate::config::RouterConfig {
                default_outbound: self.route.r#final.clone(),
                rules: Vec::new(), // 旧格式规则，我们使用新的高性能路由器
            },
            high_performance_router: crate::config::HighPerformanceRouterConfig {
                default_outbound: self.route.r#final.clone(),
                rules,
                cache: crate::config::CacheConfig {
                    max_size: 10000,
                    enabled: true,
                },
                rule_set_files: crate::config::RuleSetFilesConfig {
                    domain_files: Vec::new(),
                    ip_files: Vec::new(),
                },
            },
        };

        Ok(internal_config)
    }
}