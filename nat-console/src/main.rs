use clap::Parser;
use log::info;

mod auth;
mod config;
mod handlers;
mod server;

type DynError = Box<dyn std::error::Error + Send + Sync>;

/// WebUI for nftables NAT management
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 监听端口
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// 用户名
    #[arg(short, long)]
    username: String,

    /// 密码
    #[arg(long)]
    password: String,

    /// JWT 密钥
    #[arg(long, default_value = "your-secret-key-change-in-production")]
    jwt_secret: String,

    /// TLS 证书路径
    #[arg(long)]
    cert: Option<String>,

    /// TLS 私钥路径
    #[arg(long)]
    key: Option<String>,

    /// 传统配置文件路径（兼容模式）
    #[arg(long)]
    compatible_config: Option<String>,

    /// TOML 配置文件路径
    #[arg(long)]
    toml_config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    nat_common::logger::init(env!("CARGO_CRATE_NAME"));
    let args = Args::parse();

    info!("Starting WebUI server on port {}", args.port);
    info!("Username: {}", args.username);

    // 验证至少提供了一种配置文件
    if args.compatible_config.is_none() && args.toml_config.is_none() {
        return Err("请提供配置文件路径 (--compatible-config 或 --toml-config)".into());
    }

    server::run_server(args).await?;

    Ok(())
}
