pub mod auth;

/// 安全配置
#[derive(Clone)]
pub struct SecurityConfig {
    /// JWT密钥
    pub jwt_secret: Vec<u8>,
    /// 令牌过期时间（秒）
    pub token_expiration: u64,
}

impl SecurityConfig {
    /// 创建新的安全配置
    /// 
    /// # Arguments
    /// * `jwt_secret` - JWT密钥
    /// * `token_expiration` - 令牌过期时间（秒）
    pub fn new(jwt_secret: Vec<u8>, token_expiration: u64) -> Self {
        Self {
            jwt_secret,
            token_expiration,
        }
    }

    /// 创建默认的安全配置
    pub fn default() -> Self {
        Self {
            jwt_secret: b"your-secret-key".to_vec(),
            token_expiration: 3600, // 1小时
        }
    }
} 