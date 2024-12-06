use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use argon2::{self, Config};
use rand::Rng;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,  // 用户ID
    exp: usize,   // 过期时间
    iat: usize,   // 签发时间
    role: String, // 用户角色
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
}

pub struct AuthManager {
    users: Arc<RwLock<HashMap<String, User>>>,
    jwt_secret: String,
    token_expiration: Duration,
    max_failed_attempts: u32,
    failed_attempts: Arc<RwLock<HashMap<String, (u32, SystemTime)>>>,
    lockout_duration: Duration,
}

impl AuthManager {
    pub fn new(
        jwt_secret: String,
        token_expiration: Duration,
        max_failed_attempts: u32,
        lockout_duration: Duration,
    ) -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            jwt_secret,
            token_expiration,
            max_failed_attempts,
            failed_attempts: Arc::new(RwLock::new(HashMap::new())),
            lockout_duration,
        }
    }

    pub async fn register_user(
        &self,
        username: String,
        password: String,
        role: String,
    ) -> Result<String> {
        // 检查用户名是否已存在
        let users = self.users.read().await;
        if users.values().any(|u| u.username == username) {
            return Err(anyhow!("Username already exists"));
        }
        drop(users);

        // 生成密码哈希
        let salt = rand::thread_rng().gen::<[u8; 32]>();
        let config = Config::default();
        let hash = argon2::hash_encoded(
            password.as_bytes(),
            &salt,
            &config,
        )?;

        // 创建新用户
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            password_hash: hash,
            role,
            created_at: SystemTime::now(),
            last_login: None,
        };

        // 保存用户
        let user_id = user.id.clone();
        self.users.write().await.insert(user_id.clone(), user);

        info!("New user registered: {}", user_id);
        Ok(user_id)
    }

    pub async fn login(&self, username: String, password: String) -> Result<String> {
        // 检查账户锁定状态
        if self.is_account_locked(&username).await {
            return Err(anyhow!("Account is locked due to too many failed attempts"));
        }

        // 查找用户
        let mut users = self.users.write().await;
        let user = users.values_mut().find(|u| u.username == username)
            .ok_or_else(|| anyhow!("User not found"))?;

        // 验证密码
        if !argon2::verify_encoded(&user.password_hash, password.as_bytes())? {
            self.record_failed_attempt(&username).await;
            return Err(anyhow!("Invalid password"));
        }

        // 清除失败尝试记录
        self.clear_failed_attempts(&username).await;

        // 更新最后登录时间
        user.last_login = Some(SystemTime::now());

        // 生成 JWT token
        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as usize + self.token_expiration.as_secs() as usize;

        let claims = Claims {
            sub: user.id.clone(),
            exp: expiration,
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs() as usize,
            role: user.role.clone(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        info!("User logged in successfully: {}", user.id);
        Ok(token)
    }

    pub async fn verify_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::default();
        let key = DecodingKey::from_secret(self.jwt_secret.as_bytes());
        
        match decode::<Claims>(token, &key, &validation) {
            Ok(token_data) => Ok(token_data.claims),
            Err(e) => {
                warn!("Token verification failed: {}", e);
                Err(anyhow!("Invalid token"))
            }
        }
    }

    async fn is_account_locked(&self, username: &str) -> bool {
        let attempts = self.failed_attempts.read().await;
        if let Some((count, time)) = attempts.get(username) {
            if *count >= self.max_failed_attempts {
                let elapsed = SystemTime::now()
                    .duration_since(*time)
                    .unwrap_or(Duration::from_secs(0));
                return elapsed < self.lockout_duration;
            }
        }
        false
    }

    async fn record_failed_attempt(&self, username: &str) {
        let mut attempts = self.failed_attempts.write().await;
        let entry = attempts.entry(username.to_string())
            .or_insert((0, SystemTime::now()));
        entry.0 += 1;
        entry.1 = SystemTime::now();
        
        if entry.0 >= self.max_failed_attempts {
            warn!("Account locked due to too many failed attempts: {}", username);
        }
    }

    async fn clear_failed_attempts(&self, username: &str) {
        self.failed_attempts.write().await.remove(username);
    }

    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: String,
        new_password: String,
    ) -> Result<()> {
        let mut users = self.users.write().await;
        let user = users.get_mut(user_id)
            .ok_or_else(|| anyhow!("User not found"))?;

        // 验证旧密码
        if !argon2::verify_encoded(&user.password_hash, old_password.as_bytes())? {
            return Err(anyhow!("Invalid old password"));
        }

        // 生成新密码哈希
        let salt = rand::thread_rng().gen::<[u8; 32]>();
        let config = Config::default();
        let new_hash = argon2::hash_encoded(
            new_password.as_bytes(),
            &salt,
            &config,
        )?;

        user.password_hash = new_hash;
        info!("Password changed for user: {}", user_id);
        Ok(())
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        let mut users = self.users.write().await;
        users.remove(user_id)
            .ok_or_else(|| anyhow!("User not found"))?;
        
        info!("User deleted: {}", user_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_registration_and_login() {
        let auth = AuthManager::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            3,
            Duration::from_secs(300),
        );

        // 注册用户
        let user_id = auth.register_user(
            "testuser".to_string(),
            "password123".to_string(),
            "user".to_string(),
        ).await.expect("Failed to register user");

        // 登录
        let token = auth.login("testuser".to_string(), "password123".to_string())
            .await
            .expect("Failed to login");

        // 验证token
        let claims = auth.verify_token(&token)
            .await
            .expect("Failed to verify token");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.role, "user");
    }

    #[tokio::test]
    async fn test_failed_login_attempts() {
        let auth = AuthManager::new(
            "test_secret".to_string(),
            Duration::from_secs(3600),
            3,
            Duration::from_secs(300),
        );

        // 注册用户
        auth.register_user(
            "testuser2".to_string(),
            "password123".to_string(),
            "user".to_string(),
        ).await.expect("Failed to register user");

        // 尝试使用错误密码登录
        for _ in 0..3 {
            let result = auth.login("testuser2".to_string(), "wrongpassword".to_string()).await;
            assert!(result.is_err());
        }

        // 使用正确密码登录应该失败，因为账户已锁定
        let result = auth.login("testuser2".to_string(), "password123".to_string()).await;
        assert!(result.is_err());
    }
} 