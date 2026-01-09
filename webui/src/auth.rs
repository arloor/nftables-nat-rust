use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::{Html, Response},
};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// JWT过期时间（7天）
const JWT_EXPIRATION_HOURS: i64 = 24 * 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // 用户名
    pub username: String, // 用户名（显式字段）
    pub exp: usize,       // 过期时间
    pub iat: usize,       // 签发时间
}

#[derive(Clone)]
pub struct JwtConfig {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
}

impl JwtConfig {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}

/// 生成JWT token
pub fn create_jwt(
    username: &str,
    config: &JwtConfig,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let exp = (now + chrono::Duration::hours(JWT_EXPIRATION_HOURS)).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        sub: username.to_string(),
        username: username.to_string(),
        exp,
        iat,
    };

    encode(&Header::default(), &claims, &config.encoding_key)
}

/// 验证JWT token
pub fn verify_jwt(token: &str, config: &JwtConfig) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(token, &config.decoding_key, &validation)?;
    Ok(token_data.claims)
}

/// JWT认证中间件
pub async fn jwt_auth_middleware(
    State(config): State<Arc<JwtConfig>>,
    cookie_jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Html<String>)> {
    // 从cookie中获取JWT token
    let token = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .ok_or((StatusCode::UNAUTHORIZED, Html("Missing token".to_string())))?;

    // 验证JWT token
    let claims = verify_jwt(&token, &config)
        .map_err(|_| (StatusCode::UNAUTHORIZED, Html("Invalid token".to_string())))?;

    // 将claims存入request extensions，后续handler可以使用
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// 认证用户信息提取器
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Html<String>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts.extensions.get::<Claims>().ok_or((
            StatusCode::UNAUTHORIZED,
            Html("Missing or invalid token".to_string()),
        ))?;

        Ok(AuthUser {
            username: claims.username.clone(),
        })
    }
}
