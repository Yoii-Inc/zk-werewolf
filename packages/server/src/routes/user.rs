use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::models::user::{LoginUserRequest, RegisterUserRequest};
use crate::services::user_service::UserServiceError;
use crate::state::AppState;

pub mod auth_middleware;

// ユーザールートの設定
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login_user))
        .route("/:id", get(get_user))
        .with_state(state)
}

// エラーハンドリング
impl IntoResponse for UserServiceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            UserServiceError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "ユーザーは既に存在します")
            }
            UserServiceError::UserNotFound => (StatusCode::NOT_FOUND, "ユーザーが見つかりません"),
            UserServiceError::InvalidPassword => (
                StatusCode::UNAUTHORIZED,
                "メールアドレスまたはパスワードが正しくありません",
            ),
            UserServiceError::AuthError(_) => {
                (StatusCode::UNAUTHORIZED, "認証エラーが発生しました")
            }
            UserServiceError::RequestFailed(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "サーバーエラーが発生しました",
            ),
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

// ユーザー登録
pub async fn register_user(
    State(state): State<AppState>,
    Json(req): Json<RegisterUserRequest>,
) -> Result<impl IntoResponse, UserServiceError> {
    let result = state.user_service.register(req).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

// ユーザーログイン
pub async fn login_user(
    State(state): State<AppState>,
    Json(req): Json<LoginUserRequest>,
) -> Result<impl IntoResponse, UserServiceError> {
    let result = state.user_service.login(req).await?;
    Ok((StatusCode::OK, Json(result)))
}

// ユーザー情報取得
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, UserServiceError> {
    let user = state.user_service.get_user_by_id(&user_id).await?;
    Ok((StatusCode::OK, Json(user)))
}
