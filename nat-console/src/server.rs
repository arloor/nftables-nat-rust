use crate::auth::{jwt_auth_middleware, JwtConfig};
use crate::config::ConfigFormat;
use crate::handlers::{
    get_config, get_current_user, get_rules, get_rules_json, login_handler, logout_handler,
    save_config, AppState,
};
use crate::Args;
use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post},
    Router,
};
use axum_bootstrap::TlsParam;
use log::info;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

pub async fn run_server(args: Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 读取配置文件
    let config_format = if let Some(toml_path) = &args.toml_config {
        ConfigFormat::from_toml_file(toml_path)?
    } else if let Some(legacy_path) = &args.compatible_config {
        ConfigFormat::from_legacy_file(legacy_path)?
    } else {
        return Err("No config file provided".into());
    };

    let config_path = args
        .toml_config
        .clone()
        .or(args.compatible_config.clone())
        .ok_or("No config file provided")?;

    // 生成密码哈希
    let password_hash = bcrypt::hash(&args.password, bcrypt::DEFAULT_COST)?;

    let jwt_config = JwtConfig::new(&args.jwt_secret);

    let state = Arc::new(AppState {
        jwt_config: jwt_config.clone(),
        username: args.username,
        password_hash,
        config_path,
        config_format: Arc::new(RwLock::new(config_format)),
    });

    // 受保护的路由
    let protected_routes = Router::new()
        .route("/api/me", get(get_current_user))
        .route("/api/config", get(get_config).post(save_config))
        .route("/api/rules", get(get_rules_json))
        .route("/rules", get(get_rules))
        .layer(middleware::from_fn_with_state(
            Arc::new(jwt_config.clone()),
            jwt_auth_middleware,
        ));

    // 构建应用
    let app = Router::new()
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/health", get(|| async { (StatusCode::OK, "OK") }))
        .merge(protected_routes)
        .fallback_service(ServeDir::new("webui/static"))
        .layer((
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(|req: &axum::extract::Request| {
                    let method = req.method();
                    let path = req.uri().path();
                    tracing::debug_span!("request", %method, %path)
                })
                .on_failure(()),
            tower_http::cors::CorsLayer::permissive(),
            tower_http::timeout::TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(30),
            ),
            tower_http::compression::CompressionLayer::new()
                .gzip(true)
                .br(true)
                .deflate(true)
                .zstd(true),
        ))
        .with_state(state);

    // 启动服务器
    let server =
        axum_bootstrap::new_server(args.port, app, axum_bootstrap::generate_shutdown_receiver())
            .with_timeout(Duration::from_secs(600));

    // 如果提供了证书，使用 TLS
    let server = if let (Some(cert), Some(key)) = (args.cert, args.key) {
        info!("Starting HTTPS server on port {}", args.port);
        server.with_tls_param(Some(TlsParam {
            tls: true,
            cert,
            key,
        }))
    } else {
        info!("Starting HTTP server on port {}", args.port);
        info!("⚠️  Warning: Running without TLS! This is not secure for production.");
        server
    };

    server.run().await?;

    Ok(())
}
