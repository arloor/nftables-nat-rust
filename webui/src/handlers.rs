use crate::auth::{create_jwt, AuthUser, JwtConfig};
use crate::config::{get_nftables_rules, ConfigFormat, LegacyConfigLine, TomlConfig};
use axum::{extract::State, http::StatusCode, response::Html, Json};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use log::{error, info};
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
    let token = create_jwt(&req.username, &state.jwt_config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 创建cookie
    let cookie = Cookie::build(("token", token))
        .path("/")
        .max_age(time::Duration::days(7))
        .same_site(SameSite::Lax)
        .http_only(true)
        .build();

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
    let cookie = Cookie::build(("token", ""))
        .path("/")
        .max_age(time::Duration::seconds(-1))
        .same_site(SameSite::Lax)
        .http_only(true)
        .build();

    let jar = CookieJar::new().add(cookie);

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
    AuthUser { username }: AuthUser,
) -> Result<Json<UserInfo>, StatusCode> {
    Ok(Json(UserInfo { username }))
}

#[derive(Serialize)]
pub struct ConfigResponse {
    format: String,
    content: serde_json::Value,
}

pub async fn get_config(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let config = state.config_format.read().await;

    match &*config {
        ConfigFormat::Toml(toml_config) => {
            let content = serde_json::to_value(toml_config).map_err(|e| {
                error!("Failed to serialize TOML config: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            Ok(Json(ConfigResponse {
                format: "toml".to_string(),
                content,
            }))
        }
        ConfigFormat::Legacy(lines) => {
            let content = serde_json::to_value(lines).map_err(|e| {
                error!("Failed to serialize legacy config: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            Ok(Json(ConfigResponse {
                format: "legacy".to_string(),
                content,
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    format: String,
    content: serde_json::Value,
}

pub async fn save_config(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<SaveConfigRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    info!("Saving config, format: {}", req.format);

    let new_config = match req.format.as_str() {
        "toml" => {
            let toml_config: TomlConfig = serde_json::from_value(req.content).map_err(|e| {
                error!("Failed to deserialize TOML config: {:?}", e);
                (
                    StatusCode::BAD_REQUEST,
                    format!("Invalid TOML config: {}", e),
                )
            })?;
            ConfigFormat::Toml(toml_config)
        }
        "legacy" => {
            let lines: Vec<LegacyConfigLine> =
                serde_json::from_value(req.content).map_err(|e| {
                    error!("Failed to deserialize legacy config: {:?}", e);
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid legacy config: {}", e),
                    )
                })?;
            ConfigFormat::Legacy(lines)
        }
        _ => return Err((StatusCode::BAD_REQUEST, "Unknown config format".to_string())),
    };

    // 保存到文件
    new_config.save_to_file(&state.config_path).map_err(|e| {
        error!("Failed to save config to file: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save config: {}", e),
        )
    })?;

    // 更新内存中的配置
    let mut config = state.config_format.write().await;
    *config = new_config;

    info!("Config saved successfully");
    Ok((StatusCode::OK, "配置已保存".to_string()))
}

pub async fn get_rules(_user: AuthUser) -> Result<Html<String>, (StatusCode, String)> {
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

pub async fn get_rules_json(_user: AuthUser) -> Result<Json<RulesResponse>, (StatusCode, String)> {
    let rules = get_nftables_rules().map_err(|e| {
        error!("Failed to get nftables rules: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get rules: {}", e),
        )
    })?;

    Ok(Json(RulesResponse { rules }))
}
