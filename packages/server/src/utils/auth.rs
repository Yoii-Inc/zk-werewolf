use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use std::env;

use crate::models::user::User;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("トークンの作成に失敗しました")]
    TokenCreation,
    #[error("トークンの検証に失敗しました")]
    TokenValidation,
    #[error("パスワードのハッシュ化に失敗しました")]
    PasswordHash,
    #[error("認証に失敗しました")]
    InvalidCredentials,
    #[error("ユーザーが見つかりませんでした")]
    UserNotFound,
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    bcrypt::hash(password, 10).map_err(|_| AuthError::PasswordHash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn create_token(user: &User) -> Result<String, AuthError> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user.id.clone(),
        exp,
        iat,
    };

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::TokenCreation)
}

pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AuthError::TokenValidation)?;

    Ok(token_data.claims)
}