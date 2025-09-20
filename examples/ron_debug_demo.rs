// RON配置解析调试演示程序
use anybls::ron_config::RonConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("RON配置解析调试演示程序");
    println!("======================");

    // 1. 读取原始文件内容
    println!("\n步骤1: 读取原始文件内容");
    let content = std::fs::read_to_string("examples/simple_config.ron")?;
    println!("文件内容:");
    println!("{}", content);

    // 2. 尝试解析RON
    println!("\n步骤2: 尝试解析RON");
    match ron::from_str::<RonConfig>(&content) {
        Ok(config) => {
            println!("RON解析成功");
            println!("配置: {:?}", config);
        }
        Err(e) => {
            println!("RON解析失败: {}", e);
            
            // 3. 尝试解析为基本结构
            println!("\n步骤3: 尝试解析为基本结构");
            #[derive(Debug, serde::Deserialize)]
            struct SimpleConfig {
                log: Option<serde_json::Value>,
                inbounds: Vec<serde_json::Value>,
                outbounds: Vec<serde_json::Value>,
                route: serde_json::Value,
            }
            
            match ron::from_str::<SimpleConfig>(&content) {
                Ok(simple_config) => {
                    println!("基本结构解析成功");
                    println!("基本配置: {:?}", simple_config);
                }
                Err(e2) => {
                    println!("基本结构解析也失败: {}", e2);
                }
            }
        }
    }

    Ok(())
}
