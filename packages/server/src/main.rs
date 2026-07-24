use axum::{
    extract::Query,
    http::{self, HeaderValue, Method},
    routing::get,
    Json, Router,
};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing_subscriber::EnvFilter;

mod app;
mod blockchain;
mod models;
mod routes;
mod services;
mod state;
mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct Info {
    name: String,
}

#[derive(Serialize, Debug)]
struct Greet {
    message: String,
}

// ゲームの状態を保持する構造体
struct GameState {
    players: Vec<String>,
    // ... その他ゲームの状態 (役職、生存状況、ゲームフェーズなど) ...
}

// GameStateを共有するためのArc<Mutex<GameState>>
static GAME_STATE: Lazy<Arc<Mutex<GameState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(GameState {
        players: Vec::new(),
    }))
});

async fn greet(Query(params): Query<Info>) -> Json<Greet> {
    // ゲームの状態にプレイヤーを追加
    let mut state = GAME_STATE.lock().unwrap();
    state.players.push(params.name.clone());
    drop(state); // ロックを解放
    Json(Greet {
        message: format!(
            "Hello, {}!, {}",
            params.name,
            GAME_STATE.lock().unwrap().players.join(", ")
        ),
    })
}

#[tokio::main]
async fn main() {
    // グローバルなtracing subscriberを初期化 (何よりも先に行う必要がある)。
    // RUST_LOG (ECSタスク定義でinfoに設定済み) を尊重し、未設定時はinfoにフォールバックする。
    // JSON出力にすることで CloudWatch Logs Insights でフィールドを直接クエリできる。
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true) // status/latency_msなどのイベントフィールドをトップレベルに
        .with_current_span(true) // method/path/user_agentなどのspanフィールドを`span`配下に残す
        .with_span_list(false) // 冗長な`spans`配列は出さない
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 環境変数をロード
    if let Err(e) = dotenv() {
        eprintln!("Warning: .envファイルの読み込みに失敗しました: {}", e);
    }

    // 環境変数の存在確認
    let required_vars = [
        "SUPABASE_URL",
        "SUPABASE_KEY",
        "JWT_SECRET",
        "ZK_MPC_NODE_0_HTTP",
        "ZK_MPC_NODE_1_HTTP",
        "ZK_MPC_NODE_2_HTTP",
    ];
    let mut missing_vars = Vec::new();

    for var in &required_vars {
        if std::env::var(var).is_err() {
            eprintln!("Error: 環境変数 {} が設定されていません", var);
            missing_vars.push(*var);
        }
    }

    // 必須環境変数が不足している場合はプロセスを終了
    if !missing_vars.is_empty() {
        eprintln!(
            "Fatal: 必須環境変数が設定されていません: {}",
            missing_vars.join(", ")
        );
        eprintln!("サーバーを起動できません。.envファイルを確認してください。");
        std::process::exit(1);
    }

    // CORSレイヤーの設定
    let origins = ["http://localhost:3000".parse::<HeaderValue>().unwrap()];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION]);

    // ルーティングの設定
    let traced_app = app::create_app()
        .route("/greet", get(greet))
        .layer(cors) // CORSレイヤーを追加
        .layer(
            TraceLayer::new_for_http() // HTTPトレースログを有効化
                .make_span_with(|request: &http::Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        path = %request.uri().path(), // クエリ文字列は含めない (トークン漏洩防止)
                        user_agent = request
                            .headers()
                            .get(http::header::USER_AGENT)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("-"),
                    )
                })
                .on_response(
                    |response: &http::Response<_>, latency: Duration, _span: &Span| {
                        tracing::info!(
                            status = response.status().as_u16(),
                            latency_ms = latency.as_millis() as u64,
                            "request completed"
                        );
                    },
                )
                .on_failure(
                    |failure: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                        tracing::error!(
                            error = %failure,
                            latency_ms = latency.as_millis() as u64,
                            "request failed"
                        );
                    },
                ),
        );

    // "/health" はALBが30秒間隔で叩くヘルスチェック用なので、TraceLayer/CORSの外側に
    // 登録してアクセスログがヘルスチェックのノイズで埋もれないようにする。
    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .merge(traced_app);

    // サーバーの起動
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listner = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("サーバーを起動しました: http://{}", addr);
    axum::serve(listner, app).await.unwrap();
}
