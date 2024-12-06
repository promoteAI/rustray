use std::time::Duration;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::sync::Arc;
use std::sync::mpsc;

pub mod auth;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT密钥
    pub jwt_secret: String,
    /// Token过期时间（秒）
    pub token_expiration: u64,
    /// 最大失败尝试次数
    pub max_failed_attempts: u32,
    /// 锁定时间（秒）
    pub lockout_duration: u64,
    /// 密码最小长度
    pub min_password_length: usize,
    /// 是否要求密码包含数字
    pub require_numbers: bool,
    /// 是否要求密码包含特殊字符
    pub require_special_chars: bool,
    /// 是否要求密码包含大写字母
    pub require_uppercase: bool,
    /// 是否要求密码包含小写字母
    pub require_lowercase: bool,
    /// 密码历史记录长度
    pub password_history_length: usize,
    /// SSL/TLS配置
    pub tls_config: Option<TlsConfig>,
    /// IP白名单
    pub ip_whitelist: Vec<String>,
    /// 允许的API路径
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// 证书路径
    pub cert_path: String,
    /// 私钥路径
    pub key_path: String,
    /// 是否验证客户端证书
    pub verify_client: bool,
    /// CA证书路径
    pub ca_cert_path: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "default_secret_please_change".to_string(),
            token_expiration: 3600,
            max_failed_attempts: 5,
            lockout_duration: 300,
            min_password_length: 8,
            require_numbers: true,
            require_special_chars: true,
            require_uppercase: true,
            require_lowercase: true,
            password_history_length: 5,
            tls_config: None,
            ip_whitelist: vec![],
            allowed_paths: vec![],
        }
    }
}

impl SecurityConfig {
    pub fn validate(&self) -> Result<()> {
        // 验证JWT密钥
        if self.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!("JWT secret must be at least 32 characters long"));
        }

        // 验证过期时间
        if self.token_expiration == 0 {
            return Err(anyhow::anyhow!("Token expiration must be greater than 0"));
        }

        // 验证密码策略
        if self.min_password_length < 8 {
            return Err(anyhow::anyhow!("Minimum password length must be at least 8"));
        }

        // 验证TLS配置
        if let Some(tls) = &self.tls_config {
            if !std::path::Path::new(&tls.cert_path).exists() {
                return Err(anyhow::anyhow!("Certificate file not found"));
            }
            if !std::path::Path::new(&tls.key_path).exists() {
                return Err(anyhow::anyhow!("Private key file not found"));
            }
            if tls.verify_client {
                if let Some(ca_path) = &tls.ca_cert_path {
                    if !std::path::Path::new(ca_path).exists() {
                        return Err(anyhow::anyhow!("CA certificate file not found"));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn to_auth_manager(&self) -> auth::AuthManager {
        auth::AuthManager::new(
            self.jwt_secret.clone(),
            Duration::from_secs(self.token_expiration),
            self.max_failed_attempts,
            Duration::from_secs(self.lockout_duration),
        )
    }

    pub fn validate_password(&self, password: &str) -> Result<()> {
        // 检查长度
        if password.len() < self.min_password_length {
            return Err(anyhow::anyhow!(
                "Password must be at least {} characters long",
                self.min_password_length
            ));
        }

        // 检查数字
        if self.require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err(anyhow::anyhow!("Password must contain at least one number"));
        }

        // 检查特殊字符
        if self.require_special_chars && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(anyhow::anyhow!("Password must contain at least one special character"));
        }

        // 检查大写字母
        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(anyhow::anyhow!("Password must contain at least one uppercase letter"));
        }

        // 检查小写字母
        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(anyhow::anyhow!("Password must contain at least one lowercase letter"));
        }

        Ok(())
    }

    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        if self.ip_whitelist.is_empty() {
            return true;
        }
        self.ip_whitelist.iter().any(|allowed| {
            if allowed.ends_with("*") {
                let prefix = &allowed[..allowed.len() - 1];
                ip.starts_with(prefix)
            } else {
                ip == allowed
            }
        })
    }

    pub fn is_path_allowed(&self, path: &str) -> bool {
        if self.allowed_paths.is_empty() {
            return true;
        }
        self.allowed_paths.iter().any(|allowed| {
            if allowed.ends_with("*") {
                let prefix = &allowed[..allowed.len() - 1];
                path.starts_with(prefix)
            } else {
                path == allowed
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let config = SecurityConfig::default();

        // 测试有效密码
        assert!(config.validate_password("Password123!").is_ok());

        // 测试密码太短
        assert!(config.validate_password("Pw1!").is_err());

        // 测试缺少数字
        assert!(config.validate_password("Password!").is_err());

        // 测试缺少特殊字符
        assert!(config.validate_password("Password123").is_err());

        // 测试缺少大写字母
        assert!(config.validate_password("password123!").is_err());

        // 测试缺少小写字母
        assert!(config.validate_password("PASSWORD123!").is_err());
    }

    #[test]
    fn test_ip_whitelist() {
        let mut config = SecurityConfig::default();
        config.ip_whitelist = vec![
            "192.168.1.*".to_string(),
            "10.0.0.1".to_string(),
        ];

        assert!(config.is_ip_allowed("192.168.1.100"));
        assert!(config.is_ip_allowed("192.168.1.200"));
        assert!(config.is_ip_allowed("10.0.0.1"));
        assert!(!config.is_ip_allowed("192.168.2.100"));
        assert!(!config.is_ip_allowed("10.0.0.2"));
    }

    #[test]
    fn test_path_whitelist() {
        let mut config = SecurityConfig::default();
        config.allowed_paths = vec![
            "/api/v1/*".to_string(),
            "/health".to_string(),
        ];

        assert!(config.is_path_allowed("/api/v1/users"));
        assert!(config.is_path_allowed("/api/v1/tasks"));
        assert!(config.is_path_allowed("/health"));
        assert!(!config.is_path_allowed("/api/v2/users"));
        assert!(!config.is_path_allowed("/metrics"));
    }
} 