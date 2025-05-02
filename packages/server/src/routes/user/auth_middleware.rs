use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;

use crate::{state::AppState, utils::auth::verify_token};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // ヘッダーからトークンを取得
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_value| {
            if auth_value.starts_with("Bearer ") {
                Some(auth_value[7..].to_owned())
            } else {
                None
            }
        });

    let token = match auth_header {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "認証が必要です" })),
            ));
        }
    };

    // トークンを検証
    let claims = match verify_token(&token) {
        Ok(claims) => claims,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "無効なトークンです" })),
            ));
        }
    };

    // ユーザーIDをリクエスト拡張に設定
    request.extensions_mut().insert(claims.sub);

    // 次のハンドラに進む
    Ok(next.run(request).await)
}
