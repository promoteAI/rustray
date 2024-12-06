//! 认证模块
//! 
//! 本模块实现了系统的认证和授权功能，支持：
//! - 用户认证和会话管理
//! - 基于角色的访问控制(RBAC)
//! - 权限管理
//! - 安全令牌生成和验证
//! - 审计日志

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::metrics::MetricsCollector;

/// 用户角色
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    /// 管理员
    Admin,
    /// 普通用户
    User,
    /// 访客
    Guest,
}

/// 权限类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    /// 读取权限
    Read,
    /// 写入权限
    Write,
    /// 执行权限
    Execute,
    /// 管理权限
    Manage,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户ID
    pub id: String,
    /// 用户名
    pub username: String,
    /// 密码哈希
    pub password_hash: String,
    /// 角色
    pub role: UserRole,
    /// 权限列表
    pub permissions: Vec<Permission>,
    /// 创建时间
    pub created_at: u64,
    /// 最后登录时间
    pub last_login: Option<u64>,
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话ID
    pub id: String,
    /// 用户ID
    pub user_id: String,
    /// 创建时间
    pub created_at: u64,
    /// 过期时间
    pub expires_at: u64,
    /// IP地址
    pub ip_address: String,
    /// 用户代理
    pub user_agent: String,
}

/// JWT声明
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// 主题（用户ID）
    sub: String,
    /// 过期时间
    exp: u64,
    /// 发行时间
    iat: u64,
    /// 角色
    role: UserRole,
}

/// 认证管理器
pub struct AuthManager {
    /// 用户映射
    users: Arc<Mutex<HashMap<String, User>>>,
    /// 会话映射
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    /// JWT密钥
    jwt_secret: String,
    /// 会话过期时间
    session_ttl: Duration,
    /// 指标收集器
    metrics: Arc<MetricsCollector>,
    /// 审计日志通道
    audit_tx: mpsc::Sender<AuditLog>,
}

/// 审计日志
#[derive(Debug, Clone, Serialize)]
pub struct AuditLog {
    /// 事件ID
    pub event_id: String,
    /// 事件类型
    pub event_type: String,
    /// 用户ID
    pub user_id: String,
    /// 资源
    pub resource: String,
    /// 操作
    pub action: String,
    /// 状态
    pub status: String,
    /// 时间戳
    pub timestamp: u64,
    /// 详细信息
    pub details: String,
}

impl AuthManager {
    /// 创建新的认证管理器
    pub fn new(
        jwt_secret: String,
        session_ttl: Duration,
        metrics: Arc<MetricsCollector>,
        audit_tx: mpsc::Sender<AuditLog>,
    ) -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            jwt_secret,
            session_ttl,
            metrics,
            audit_tx,
        }
    }

    /// 注册新用户
    pub async fn register_user(
        &self,
        username: String,
        password: String,
        role: UserRole,
    ) -> Result<User, String> {
        let users = self.users.lock().map_err(|e| e.to_string())?;
        
        // 检查用户名是否已存在
        if users.values().any(|u| u.username == username) {
            return Err("Username already exists".to_string());
        }

        // 创建用户
        let user = User {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash: Self::hash_password(&password)?,
            role,
            permissions: Self::get_default_permissions(&role),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_login: None,
        };

        // 保存用户
        users.insert(user.id.clone(), user.clone());

        // 记录审计日志
        self.log_audit(
            "user_registration",
            &user.id,
            "user",
            "create",
            "success",
            "User registration successful",
        ).await?;

        // 更新指标
        self.metrics.increment_counter("auth.users.registered", 1)
            .map_err(|e| e.to_string())?;

        Ok(user)
    }

    /// 用户登录
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip_address: String,
        user_agent: String,
    ) -> Result<(String, String), String> {
        let mut users = self.users.lock().map_err(|e| e.to_string())?;
        
        // 查找用户
        let user = users.values_mut()
            .find(|u| u.username == username)
            .ok_or_else(|| "User not found".to_string())?;

        // 验证密码
        if !Self::verify_password(&password, &user.password_hash)? {
            // 记录失败
            self.log_audit(
                "login_failed",
                &user.id,
                "session",
                "create",
                "failed",
                "Invalid password",
            ).await?;

            self.metrics.increment_counter("auth.login.failed", 1)
                .map_err(|e| e.to_string())?;

            return Err("Invalid password".to_string());
        }

        // 更新最后登录时间
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        user.last_login = Some(now);

        // 创建会话
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            created_at: now,
            expires_at: now + self.session_ttl.as_secs(),
            ip_address,
            user_agent,
        };

        // 生成JWT
        let token = self.generate_token(user)?;

        // 保存会话
        self.sessions.lock().map_err(|e| e.to_string())?
            .insert(session.id.clone(), session);

        // 记录审计日志
        self.log_audit(
            "login_success",
            &user.id,
            "session",
            "create",
            "success",
            "User login successful",
        ).await?;

        // 更新指标
        self.metrics.increment_counter("auth.login.success", 1)
            .map_err(|e| e.to_string())?;

        Ok((session.id, token))
    }

    /// 验证令牌
    pub fn verify_token(&self, token: &str) -> Result<User, String> {
        let validation = Validation::default();
        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());

        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|e| format!("Invalid token: {}", e))?
            .claims;

        let users = self.users.lock().map_err(|e| e.to_string())?;
        users.get(&claims.sub)
            .cloned()
            .ok_or_else(|| "User not found".to_string())
    }

    /// 检查权限
    pub fn check_permission(
        &self,
        user_id: &str,
        required_permission: &Permission,
    ) -> Result<bool, String> {
        let users = self.users.lock().map_err(|e| e.to_string())?;
        
        let user = users.get(user_id)
            .ok_or_else(|| "User not found".to_string())?;

        Ok(user.permissions.contains(required_permission))
    }

    /// 注销会话
    pub async fn logout(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|e| e.to_string())?;
        
        if let Some(session) = sessions.remove(session_id) {
            // 记录审计日志
            self.log_audit(
                "logout",
                &session.user_id,
                "session",
                "delete",
                "success",
                "User logout successful",
            ).await?;

            // 更新指标
            self.metrics.increment_counter("auth.logout", 1)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    // 私有辅助方法

    /// 生成JWT令牌
    fn generate_token(&self, user: &User) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user.id.clone(),
            exp: now + self.session_ttl.as_secs(),
            iat: now,
            role: user.role.clone(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        ).map_err(|e| format!("Failed to generate token: {}", e))
    }

    /// 获取默认权限
    fn get_default_permissions(role: &UserRole) -> Vec<Permission> {
        match role {
            UserRole::Admin => vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Manage,
            ],
            UserRole::User => vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
            ],
            UserRole::Guest => vec![
                Permission::Read,
            ],
        }
    }

    /// 密码哈希
    fn hash_password(password: &str) -> Result<String, String> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| format!("Failed to hash password: {}", e))
    }

    /// 验证密码
    fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
        bcrypt::verify(password, hash)
            .map_err(|e| format!("Failed to verify password: {}", e))
    }

    /// 记录审计日志
    async fn log_audit(
        &self,
        event_type: &str,
        user_id: &str,
        resource: &str,
        action: &str,
        status: &str,
        details: &str,
    ) -> Result<(), String> {
        let log = AuditLog {
            event_id: Uuid::new_v4().to_string(),
            event_type: event_type.to_string(),
            user_id: user_id.to_string(),
            resource: resource.to_string(),
            action: action.to_string(),
            status: status.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            details: details.to_string(),
        };

        self.audit_tx.send(log).await
            .map_err(|e| format!("Failed to send audit log: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_auth_manager() -> AuthManager {
        let (tx, _) = mpsc::channel(100);
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        
        AuthManager::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            metrics,
            tx,
        )
    }

    #[tokio::test]
    async fn test_user_registration() {
        let auth = create_test_auth_manager().await;
        
        let result = auth.register_user(
            "test_user".to_string(),
            "password123".to_string(),
            UserRole::User,
        ).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "test_user");
        assert_eq!(user.role, UserRole::User);
    }

    #[tokio::test]
    async fn test_user_login() {
        let auth = create_test_auth_manager().await;
        
        // 注册用户
        let user = auth.register_user(
            "test_user".to_string(),
            "password123".to_string(),
            UserRole::User,
        ).await.unwrap();

        // 测试登录
        let result = auth.login(
            "test_user",
            "password123",
            "127.0.0.1".to_string(),
            "test_agent".to_string(),
        ).await;

        assert!(result.is_ok());
        let (session_id, token) = result.unwrap();
        assert!(!session_id.is_empty());
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_permission_check() {
        let auth = create_test_auth_manager().await;
        
        // 注册管理员用户
        let admin = auth.register_user(
            "admin".to_string(),
            "admin123".to_string(),
            UserRole::Admin,
        ).await.unwrap();

        // 测试权限检查
        let result = auth.check_permission(&admin.id, &Permission::Manage);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_token_verification() {
        let auth = create_test_auth_manager().await;
        
        // 注册用户并登录
        let user = auth.register_user(
            "test_user".to_string(),
            "password123".to_string(),
            UserRole::User,
        ).await.unwrap();

        let (_, token) = auth.login(
            "test_user",
            "password123",
            "127.0.0.1".to_string(),
            "test_agent".to_string(),
        ).await.unwrap();

        // 验证令牌
        let result = auth.verify_token(&token);
        assert!(result.is_ok());
        let verified_user = result.unwrap();
        assert_eq!(verified_user.id, user.id);
    }
} 