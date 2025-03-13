use axum::{
    extract::Query,
    http::{self, HeaderValue, Method},
    routing::get,
    Json,
};
use env_logger::Builder;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod app;
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

// ログ設定
fn init_logger() {
    let mut builder = Builder::new();
    builder.filter_level(log::LevelFilter::Info); // ログレベルを設定
    builder.init();
}

#[tokio::main]
async fn main() {
    init_logger(); // ロガーの初期化

    // CORSレイヤーの設定
    let origins = ["http://localhost:3000".parse::<HeaderValue>().unwrap()];

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET]);

    // ルーティングの設定
    let app = app::create_app()
        .route("/greet", get(greet))
        .layer(cors) // CORSレイヤーを追加
        .layer(
            TraceLayer::new_for_http() // HTTPトレースログを有効化
                .make_span_with(|request: &http::Request<_>| {
                    tracing::info_span!(
                        "HTTP request",
                        method = %request.method(),
                        uri = %request.uri(),
                        headers = ?request.headers()
                    )
                }),
        );

    // サーバーの起動

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listner = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listner, app).await.unwrap();
}
