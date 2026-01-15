use crate::config::{ConfigFormat, LegacyConfigLine, get_nftables_rules};
use axum::{Json, extract::State, http::StatusCode, response::Html};
use axum_bootstrap::jwt::{Claims, ClaimsPayload, JwtConfig, LOGOUT_COOKIE};
use axum_extra::extract::CookieJar;
use log::{error, info};
use nat_common::TomlConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub jwt_config: JwtConfig,
    pub username: String,
    pub password_hash: String,
    pub config_path: String,
    pub config_format: Arc<RwLock<ConfigFormat>>,
}

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    success: bool,
    message: String,
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, CookieJar, Json<LoginResponse>), StatusCode> {
    // 验证用户名
    if req.username != state.username {
        return Ok((
            StatusCode::UNAUTHORIZED,
            CookieJar::new(),
            Json(LoginResponse {
                success: false,
                message: "用户名或密码错误".to_string(),
            }),
        ));
    }

    // 验证密码
    let password_valid = bcrypt::verify(&req.password, &state.password_hash).map_err(|e| {
        error!("密码验证失败: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !password_valid {
        return Ok((
            StatusCode::UNAUTHORIZED,
            CookieJar::new(),
            Json(LoginResponse {
                success: false,
                message: "用户名或密码错误".to_string(),
            }),
        ));
    }

    // 生成JWT token
    let cookie = Claims::new(ClaimsPayload {
        username: req.username,
    })
    .to_cookie(&state.jwt_config)
    .map_err(|e| {
        error!("生成JWT token失败: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let jar = CookieJar::new().add(cookie);

    Ok((
        StatusCode::OK,
        jar,
        Json(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
        }),
    ))
}

pub async fn logout_handler() -> Result<(StatusCode, CookieJar, Json<LoginResponse>), StatusCode> {
    let jar = CookieJar::new().add(LOGOUT_COOKIE.clone());

    Ok((
        StatusCode::OK,
        jar,
        Json(LoginResponse {
            success: true,
            message: "已退出登录".to_string(),
        }),
    ))
}

#[derive(Serialize)]
pub struct UserInfo {
    username: String,
}

pub async fn get_current_user(
    Claims { payload, .. }: Claims,
) -> Result<Json<UserInfo>, StatusCode> {
    Ok(Json(UserInfo {
        username: payload.username,
    }))
}

#[derive(Serialize)]
pub struct ConfigResponse {
    format: String,
    content: String, // 直接返回字符串格式
}

pub async fn get_config(
    _user: Claims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let config = state.config_format.read().await;

    Ok(Json(ConfigResponse {
        format: match &*config {
            ConfigFormat::Toml(_) => "toml".to_string(),
            ConfigFormat::Legacy(_) => "legacy".to_string(),
        },
        content: config.to_string(),
    }))
}

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    format: String,
    content: String, // 直接接收字符串格式
}

pub async fn save_config(
    _user: Claims,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveConfigRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    info!("Saving config, format: {}", req.format);

    let new_config = match req.format.as_str() {
        "toml" => {
            // 使用 nat-common 的验证功能
            TomlConfig::from_toml_str(&req.content).map_err(|e| {
                error!("Invalid TOML config: {:?}", e);
                (StatusCode::BAD_REQUEST, format!("配置验证失败: {}", e))
            })?;
            ConfigFormat::Toml(req.content)
        }
        "legacy" => {
            let lines: Vec<LegacyConfigLine> = req
                .content
                .lines()
                .map(|line| LegacyConfigLine {
                    line: line.to_string(),
                })
                .collect();
            ConfigFormat::Legacy(lines)
        }
        _ => return Err((StatusCode::BAD_REQUEST, "未知的配置格式".to_string())),
    };

    // 保存到文件
    new_config.save_to_file(&state.config_path).map_err(|e| {
        error!("Failed to save config to file: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存配置失败: {}", e),
        )
    })?;

    // 更新内存中的配置
    let mut config = state.config_format.write().await;
    *config = new_config;

    info!("Config saved successfully");
    Ok((StatusCode::OK, "配置已保存".to_string()))
}

pub async fn get_rules(_user: Claims) -> Result<Html<String>, (StatusCode, String)> {
    let rules = get_nftables_rules().map_err(|e| {
        error!("Failed to get nftables rules: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get rules: {}", e),
        )
    })?;

    Ok(Html(format!("<pre>{}</pre>", rules)))
}

#[derive(Serialize)]
pub struct RulesResponse {
    rules: String,
}

pub async fn get_rules_json(_user: Claims) -> Result<Json<RulesResponse>, (StatusCode, String)> {
    let rules = get_nftables_rules().map_err(|e| {
        error!("Failed to get nftables rules: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get rules: {}", e),
        )
    })?;

    Ok(Json(RulesResponse { rules }))
}
