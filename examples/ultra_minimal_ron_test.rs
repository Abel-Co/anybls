// 超最小RON测试
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct UltraMinimalConfig {
    rule_set: Vec<RuleSetConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RuleSetConfig {
    tag: String,
    #[serde(rename = "type")]
    rule_set_type: String,
    url: Option<String>,
    format: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("超最小RON测试");
    println!("============");

    // 1. 创建最小配置内容
    let content = r#"
(
    rule_set: [
        (tag: "GeoSite-CN", type: "remote", url: "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@sing/geo/geosite/google.srs", format: "binary"),
    ],
)
"#;

    println!("测试内容:");
    println!("{}", content);

    // 2. 尝试解析
    match ron::from_str::<UltraMinimalConfig>(content) {
        Ok(config) => {
            println!("RON解析成功");
            println!("配置: {:?}", config);
            
            // 3. 显示规则集
            println!("\n规则集:");
            for rule_set in &config.rule_set {
                if rule_set.rule_set_type == "remote" {
                    println!("  远程规则集: {} -> {}", 
                        rule_set.tag, 
                        rule_set.url.as_ref().unwrap_or(&"无URL".to_string())
                    );
                } else {
                    println!("  本地规则集: {}", rule_set.tag);
                }
            }
        }
        Err(e) => {
            println!("RON解析失败: {}", e);
        }
    }

    Ok(())
}
