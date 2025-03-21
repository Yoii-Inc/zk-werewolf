use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::env;

use crate::models::user::{
    AuthResponse, LoginUserRequest, RegisterUserRequest, User, UserResponse,
};
use crate::utils::auth::{create_token, hash_password, verify_password, AuthError};

#[derive(Clone)]
pub struct UserService {
    client: Client,
    supabase_url: String,
    supabase_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum UserServiceError {
    #[error("Supabaseへのリクエストに失敗しました: {0}")]
    RequestFailed(String),
    #[error("認証エラー: {0}")]
    AuthError(#[from] AuthError),
    #[error("ユーザーが既に存在します")]
    UserAlreadyExists,
    #[error("ユーザーが見つかりませんでした")]
    UserNotFound,
    #[error("不正なパスワードです")]
    InvalidPassword,
}

impl UserService {
    pub fn new() -> Self {
        let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
        let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
        let client = Client::new();

        Self {
            client,
            supabase_url,
            supabase_key,
        }
    }

    async fn check_user_exists(&self, email: &str) -> Result<bool, UserServiceError> {
        let url = format!("{}/rest/v1/users?email=eq.{}", self.supabase_url, email);

        let response = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        let users: Vec<User> = response
            .json()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        Ok(!users.is_empty())
    }

    pub async fn register(
        &self,
        req: RegisterUserRequest,
    ) -> Result<AuthResponse, UserServiceError> {
        // 既存ユーザーチェック
        if self.check_user_exists(&req.email).await? {
            return Err(UserServiceError::UserAlreadyExists);
        }

        // パスワードハッシュ
        let password_hash = hash_password(&req.password)?;

        // 新規ユーザー作成
        let user = User::new(req.username, req.email, password_hash);

        // Supabaseに保存
        let url = format!("{}/rest/v1/users", self.supabase_url);

        let user_json = json!({
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "password_hash": user.password_hash,
            "created_at": user.created_at,
            "updated_at": user.updated_at,
        });

        self.client
            .post(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .json(&user_json)
            .send()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        // JWTトークン生成
        let token = create_token(&user)?;

        Ok(AuthResponse {
            user: UserResponse::from(user),
            token,
        })
    }

    pub async fn login(&self, req: LoginUserRequest) -> Result<AuthResponse, UserServiceError> {
        // ユーザー検索
        let url = format!("{}/rest/v1/users?email=eq.{}", self.supabase_url, req.email);

        let response = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        let users: Vec<User> = response
            .json()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        let user = users.first().ok_or(UserServiceError::UserNotFound)?.clone();

        // パスワード検証
        if !verify_password(&req.password, &user.password_hash) {
            return Err(UserServiceError::InvalidPassword);
        }

        // 最終ログイン時間更新
        let url = format!("{}/rest/v1/users?id=eq.{}", self.supabase_url, user.id);
        let now = Utc::now();

        let update_json = json!({
            "last_login": now,
            "updated_at": now
        });

        self.client
            .patch(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .json(&update_json)
            .send()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        // JWTトークン生成
        let mut updated_user = user.clone();
        updated_user.last_login = Some(now);
        updated_user.updated_at = now;

        let token = create_token(&updated_user)?;

        Ok(AuthResponse {
            user: UserResponse::from(updated_user),
            token,
        })
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User, UserServiceError> {
        let url = format!("{}/rest/v1/users?id=eq.{}", self.supabase_url, user_id);

        let response = self
            .client
            .get(&url)
            .header("apikey", &self.supabase_key)
            .header("Authorization", format!("Bearer {}", self.supabase_key))
            .send()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        let users: Vec<User> = response
            .json()
            .await
            .map_err(|e| UserServiceError::RequestFailed(e.to_string()))?;

        users.first().cloned().ok_or(UserServiceError::UserNotFound)
    }
}
