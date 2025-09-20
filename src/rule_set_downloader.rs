// 规则集下载器和缓存系统
use crate::error::{ProxyError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::fs as async_fs;

/// 规则集缓存信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSetCacheInfo {
    pub tag: String,
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub file_path: PathBuf,
    pub download_time: u64,
    pub file_size: u64,
}

/// 规则集下载器
pub struct RuleSetDownloader {
    cache_dir: PathBuf,
    cache_info: HashMap<String, RuleSetCacheInfo>,
    cache_file: PathBuf,
}

impl RuleSetDownloader {
    /// 创建新的规则集下载器
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        let cache_file = cache_dir.join("rule_sets_cache.json");
        
        // 确保缓存目录存在
        fs::create_dir_all(&cache_dir)
            .map_err(|e| ProxyError::Io(e))?;
        
        // 加载现有缓存信息
        let cache_info = Self::load_cache_info(&cache_file)?;
        
        Ok(Self {
            cache_dir,
            cache_info,
            cache_file,
        })
    }
    
    /// 加载缓存信息
    fn load_cache_info(cache_file: &Path) -> Result<HashMap<String, RuleSetCacheInfo>> {
        if !cache_file.exists() {
            return Ok(HashMap::new());
        }
        
        let content = fs::read_to_string(cache_file)
            .map_err(|e| ProxyError::Io(e))?;
        
        let cache_info: HashMap<String, RuleSetCacheInfo> = serde_json::from_str(&content)
            .map_err(|e| ProxyError::Protocol(format!("Failed to parse cache info: {}", e)))?;
        
        Ok(cache_info)
    }
    
    /// 保存缓存信息
    fn save_cache_info(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.cache_info)
            .map_err(|e| ProxyError::Protocol(format!("Failed to serialize cache info: {}", e)))?;
        
        fs::write(&self.cache_file, content)
            .map_err(|e| ProxyError::Io(e))?;
        
        Ok(())
    }
    
    /// 下载规则集
    pub async fn download_rule_set(&mut self, tag: &str, url: &str) -> Result<PathBuf> {
        // 检查是否已有缓存
        if let Some(cache_info) = self.cache_info.get(tag) {
            if self.is_cache_valid(cache_info, url).await? {
                println!("使用缓存的规则集: {} -> {}", tag, cache_info.file_path.display());
                return Ok(cache_info.file_path.clone());
            }
        }
        
        println!("下载规则集: {} -> {}", tag, url);
        
        // 下载文件
        let (content, etag, last_modified) = self.download_file(url).await?;
        
        // 保存到缓存
        let file_path = self.cache_dir.join(format!("{}.srs", tag));
        async_fs::write(&file_path, &content).await
            .map_err(|e| ProxyError::Io(e))?;
        
        // 更新缓存信息
        let cache_info = RuleSetCacheInfo {
            tag: tag.to_string(),
            url: url.to_string(),
            etag,
            last_modified,
            file_path: file_path.clone(),
            download_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            file_size: content.len() as u64,
        };
        
        self.cache_info.insert(tag.to_string(), cache_info);
        self.save_cache_info()?;
        
        println!("规则集下载完成: {} ({} 字节)", tag, content.len());
        Ok(file_path)
    }
    
    /// 检查缓存是否有效
    async fn is_cache_valid(&self, cache_info: &RuleSetCacheInfo, url: &str) -> Result<bool> {
        // 检查文件是否存在
        if !cache_info.file_path.exists() {
            return Ok(false);
        }
        
        // 检查URL是否匹配
        if cache_info.url != url {
            return Ok(false);
        }
        
        // 检查ETag和Last-Modified（暂时跳过，避免网络问题）
        // TODO: 实现更稳定的远程变更检查
        // if cache_info.etag.is_some() || cache_info.last_modified.is_some() {
        //     match self.check_remote_changes(url, &cache_info.etag, &cache_info.last_modified).await {
        //         Ok(has_changes) => return Ok(!has_changes),
        //         Err(_) => {
        //             // 如果检查失败，假设有变化，重新下载
        //             return Ok(false);
        //         }
        //     }
        // }
        
        // 如果没有ETag和Last-Modified信息，检查文件年龄
        // 如果文件超过24小时，重新下载
        let file_age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - cache_info.download_time;
        
        Ok(file_age < 24 * 60 * 60) // 24小时
    }
    
    /// 检查远程文件是否有变化
    async fn check_remote_changes(
        &self,
        url: &str,
        etag: &Option<String>,
        last_modified: &Option<String>,
    ) -> Result<bool> {
        let client = reqwest::Client::new();
        let mut request = client.head(url);
        
        // 添加条件请求头
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }
        if let Some(last_modified) = last_modified {
            request = request.header("If-Modified-Since", last_modified);
        }
        
        let response = request.send().await
            .map_err(|e| ProxyError::Protocol(format!("Failed to check remote changes: {}", e)))?;
        
        // 304 Not Modified 表示没有变化
        Ok(response.status() == reqwest::StatusCode::NOT_MODIFIED)
    }
    
    /// 下载文件
    async fn download_file(&self, url: &str) -> Result<(Vec<u8>, Option<String>, Option<String>)> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await
            .map_err(|e| ProxyError::Protocol(format!("Failed to download file: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(ProxyError::Protocol(format!(
                "Failed to download file: HTTP {}",
                response.status()
            )));
        }
        
        let etag = response.headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let last_modified = response.headers()
            .get("last-modified")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let content = response.bytes().await
            .map_err(|e| ProxyError::Protocol(format!("Failed to read response: {}", e)))?;
        
        Ok((content.to_vec(), etag, last_modified))
    }
    
    /// 获取规则集文件路径
    pub fn get_rule_set_path(&self, tag: &str) -> Option<&PathBuf> {
        self.cache_info.get(tag).map(|info| &info.file_path)
    }
    
    /// 清理过期缓存
    pub fn cleanup_expired_cache(&mut self, max_age_days: u64) -> Result<()> {
        let max_age_seconds = max_age_days * 24 * 60 * 60;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut to_remove = Vec::new();
        
        for (tag, cache_info) in &self.cache_info {
            let age = current_time - cache_info.download_time;
            if age > max_age_seconds {
                // 删除文件
                if let Err(e) = fs::remove_file(&cache_info.file_path) {
                    eprintln!("Failed to remove expired cache file {}: {}", cache_info.file_path.display(), e);
                }
                to_remove.push(tag.clone());
            }
        }
        
        // 从缓存信息中移除
        let removed_count = to_remove.len();
        for tag in to_remove {
            self.cache_info.remove(&tag);
        }
        
        if removed_count > 0 {
            self.save_cache_info()?;
            println!("清理了 {} 个过期缓存文件", removed_count);
        }
        
        Ok(())
    }
    
    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_files = self.cache_info.len();
        let total_size: u64 = self.cache_info.values()
            .map(|info| info.file_size)
            .sum();
        
        CacheStats {
            total_files,
            total_size,
            cache_dir: self.cache_dir.clone(),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size: u64,
    pub cache_dir: PathBuf,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "缓存统计: {} 个文件, {} 字节, 目录: {}", 
               self.total_files, 
               self.total_size, 
               self.cache_dir.display())
    }
}
