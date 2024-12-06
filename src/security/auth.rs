//! 认证模块
//! 
//! 本模块实现了系统的认证和授权功能，支持：
//! - 用户认证和会话管理
//! - 基于角色的访问控制(RBAC)
//! - 权限管理
//! - 安全令牌生成和验证
//! - 审计日志

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use futures_util::future::TryFutureExt;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ReadTask,
    WriteTask,
    ExecuteTask,
    ManageUsers,
    ManageSystem,
}

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户ID
    pub id: Uuid,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 密码哈希
    pub password_hash: String,
    /// 角色
    pub role: UserRole,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后登录时间
    pub last_login: Option<SystemTime>,
}

impl User {
    pub fn new(
        username: String,
        email: String,
        password: String,
        role: UserRole,
    ) -> Result<Self> {
        let password_hash = bcrypt::hash(password.as_bytes(), bcrypt::DEFAULT_COST)
            .map_err(|e| RustRayError::InternalError(e.to_string()))?;

        Ok(Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            role,
            created_at: SystemTime::now(),
            last_login: None,
        })
    }

    pub fn verify_password(&self, password: &str) -> bool {
        bcrypt::verify(password.as_bytes(), &self.password_hash)
            .unwrap_or(false)
    }
}

/// 会话信息
#[derive(Debug, Clone)]
pub struct Session {
    /// 会话ID
    pub id: Uuid,
    /// 用户ID
    pub user_id: Uuid,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间
    pub expires_at: Instant,
    /// IP地址
    pub ip_address: String,
    /// 用户代理
    pub user_agent: String,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Instant::now()
    }

    pub fn extend_expiry(&mut self, duration: Duration) {
        self.expires_at = Instant::now() + duration;
    }
}

/// JWT声明
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,           // Subject (User ID)
    pub session_id: Uuid,    // Session ID
    pub exp: u64,           // Expiration time
    pub iat: u64,           // Issued at
    pub iss: String,        // Issuer
}

impl Claims {
    pub fn new(user_id: Uuid, session_id: Uuid, ttl: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sub: user_id,
            session_id,
            exp: now + ttl.as_secs(),
            iat: now,
            iss: "rustray".to_string(),
        }
    }
}

/// 认证管理器
pub struct AuthManager {
    /// 用户映射
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    /// 会话映射
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
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
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
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
            id: Uuid::new_v4(),
            username,
            email: "".to_string(),
            password_hash: Self::hash_password(&password)?,
            role: role.clone(),
            created_at: SystemTime::now(),
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
            .await
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
                .await
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
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            created_at: Instant::now(),
            expires_at: Instant::now() + self.session_ttl,
            ip_address,
            user_agent,
        };

        // Clone the session ID before moving the session
        let session_id = session.id;

        // Insert the session into the sessions map
        self.sessions.write().await
            .insert(session_id, session);

        // Generate a new token
        let token = self.generate_token(user)?;

        // 记录审��志
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
            .await
            .map_err(|e| e.to_string())?;

        Ok((session_id, token))
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
        let mut sessions = self.sessions.write().await;
        
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
                .await
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
            iss: "rustray".to_string(),
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
                Permission::ReadTask,
                Permission::WriteTask,
                Permission::ExecuteTask,
                Permission::ManageUsers,
                Permission::ManageSystem,
            ],
            UserRole::User => vec![
                Permission::ReadTask,
                Permission::WriteTask,
                Permission::ExecuteTask,
            ],
            UserRole::Guest => vec![
                Permission::ReadTask,
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

pub trait HasPermissions {
    fn has_permission(&self, permission: &Permission) -> bool;
    fn has_any_permission(&self, permissions: &[Permission]) -> bool;
    fn has_all_permissions(&self, permissions: &[Permission]) -> bool;
}

impl HasPermissions for User {
    fn has_permission(&self, permission: &Permission) -> bool {
        Permission::get_default_permissions(&self.role).contains(permission)
    }

    fn has_any_permission(&self, permissions: &[Permission]) -> bool {
        let user_permissions = Permission::get_default_permissions(&self.role);
        permissions.iter().any(|p| user_permissions.contains(p))
    }

    fn has_all_permissions(&self, permissions: &[Permission]) -> bool {
        let user_permissions = Permission::get_default_permissions(&self.role);
        permissions.iter().all(|p| user_permissions.contains(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::collector::MetricsCollector;

    #[tokio::test]
    async fn test_auth_flow() {
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let auth = AuthService::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            metrics,
        );

        // Test user registration
        let user = auth.register_user(
            "test_user".to_string(),
            "test@example.com".to_string(),
            "password123".to_string(),
            UserRole::User,
        ).await.unwrap();

        assert_eq!(user.username, "test_user");
        assert_eq!(user.role, UserRole::User);

        // Test authentication
        let authenticated = auth.authenticate_user("test_user", "password123").await.unwrap();
        assert_eq!(authenticated.id, user.id);

        // Test invalid password
        let result = auth.authenticate_user("test_user", "wrong_password").await;
        assert!(result.is_err());

        // Test session creation
        let (session_id, token) = auth.create_session(
            user.id,
            "127.0.0.1".to_string(),
            "test_agent".to_string(),
        ).await.unwrap();

        // Test token validation
        let session = auth.validate_token(&token).await.unwrap();
        assert_eq!(session.id, session_id);
        assert_eq!(session.user_id, user.id);

        // Test permission check
        assert!(auth.check_permission(user.id, &Permission::ReadTask).await.unwrap());
        assert!(!auth.check_permission(user.id, &Permission::ManageUsers).await.unwrap());

        // Test session invalidation
        auth.invalidate_session(session_id).await.unwrap();
        let result = auth.validate_token(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_admin_permissions() {
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let auth = AuthService::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            metrics,
        );

        // Create admin user
        let admin = auth.register_user(
            "admin".to_string(),
            "admin@example.com".to_string(),
            "admin123".to_string(),
            UserRole::Admin,
        ).await.unwrap();

        // Test admin permissions
        assert!(auth.check_permission(admin.id, &Permission::ManageUsers).await.unwrap());
        assert!(auth.check_permission(admin.id, &Permission::ManageSystem).await.unwrap());
    }

    #[tokio::test]
    async fn test_user_management() {
        let metrics = Arc::new(MetricsCollector::new("test".to_string()));
        let auth = AuthService::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            metrics,
        );

        // Create user
        let user = auth.register_user(
            "test_user".to_string(),
            "test@example.com".to_string(),
            "password123".to_string(),
            UserRole::User,
        ).await.unwrap();

        // Update user
        let updated = auth.update_user(
            user.id,
            Some("new@example.com".to_string()),
            Some("newpassword".to_string()),
            Some(UserRole::Admin),
        ).await.unwrap();

        assert_eq!(updated.email, "new@example.com");
        assert_eq!(updated.role, UserRole::Admin);

        // Test new password
        let result = auth.authenticate_user("test_user", "newpassword").await;
        assert!(result.is_ok());

        // Delete user
        auth.delete_user(user.id).await.unwrap();

        // Try to get deleted user
        let result = auth.get_user(user.id).await;
        assert!(result.is_err());
    }
} 