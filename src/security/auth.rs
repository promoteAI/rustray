use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Status};
use crate::error::{Result, RustRayError};
use super::SecurityConfig;

/// JWT声明
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 主题（通常是节点ID）
    pub sub: String,
    /// 角色（worker或head）
    pub role: String,
    /// 过期时间
    pub exp: u64,
    /// 发布时间
    pub iat: u64,
}

/// 认证管理器
pub struct AuthManager {
    /// 编码密钥
    encoding_key: EncodingKey,
    /// 解码密钥
    decoding_key: DecodingKey,
    /// 安全配置
    config: SecurityConfig,
}

impl AuthManager {
    /// 创建新的认证管理器
    /// 
    /// # Arguments
    /// * `secret` - JWT密钥
    pub fn new(secret: &[u8]) -> Self {
        let config = SecurityConfig::default();
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            config,
        }
    }

    /// 使用指定配置创建认证管理器
    /// 
    /// # Arguments
    /// * `config` - 安全配置
    pub fn with_config(config: SecurityConfig) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(&config.jwt_secret),
            decoding_key: DecodingKey::from_secret(&config.jwt_secret),
            config,
        }
    }

    /// 生成JWT令牌
    /// 
    /// # Arguments
    /// * `id` - 节点ID
    /// * `role` - 节点角色
    pub fn generate_token(&self, id: &str, role: &str) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: id.to_string(),
            role: role.to_string(),
            exp: now + self.config.token_expiration,
            iat: now,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| RustRayError::AuthenticationError(e.to_string()))
    }

    /// 验证JWT令牌
    /// 
    /// # Arguments
    /// * `token` - JWT令牌
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        
        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|token_data| token_data.claims)
            .map_err(|e| RustRayError::AuthenticationError(e.to_string()))
    }
}

/// gRPC认证中间件
pub struct AuthInterceptor {
    /// 认证管理器
    auth_manager: AuthManager,
}

impl AuthInterceptor {
    /// 创建新的认证中间件
    /// 
    /// # Arguments
    /// * `auth_manager` - 认证管理器
    pub fn new(auth_manager: AuthManager) -> Self {
        Self { auth_manager }
    }

    /// 拦截并验证请求
    /// 
    /// # Arguments
    /// * `request` - gRPC请求
    pub fn intercept<T>(&self, request: Request<T>) -> std::result::Result<Request<T>, Status> {
        let token = match request.metadata().get("authorization") {
            Some(t) => t.to_str().map_err(|_| Status::unauthenticated("Invalid token"))?,
            None => return Err(Status::unauthenticated("Missing token")),
        };

        match self.auth_manager.verify_token(token) {
            Ok(claims) => {
                tracing::debug!("Request authenticated for {}", claims.sub);
                Ok(request)
            }
            Err(e) => {
                tracing::warn!("Authentication failed: {}", e);
                Err(Status::unauthenticated("Invalid token"))
            }
        }
    }
} 