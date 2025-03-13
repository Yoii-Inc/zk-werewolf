起動

```
cargo run
```

websocket connect

```
websocat ws://localhost:8080/api/room/ws
```

API try in console (create room)
```
curl -X POST http://localhost:8080/api/room/create
```

```
src/
├── main.rs                  # エントリポイント
├── app.rs                   # アプリケーション全体を構築するモジュール
├── routes/                  # ルーティング関連
│   ├── mod.rs               # ルーティングのエントリポイント
│   ├── lobby.rs             # ロビー関連
│   ├── game.rs              # ゲーム関連
├── services/                # ビジネスロジック
│   ├── mod.rs               # サービスエントリポイント
│   ├── lobby_service.rs     # ロビー管理ロジック
│   ├── game_service.rs      # ゲーム進行ロジック
├── models/                  # データモデル
│   ├── mod.rs               # モデルエントリポイント
│   ├── player.rs            # プレイヤーモデル
│   ├── game.rs              # ゲームモデル
├── state.rs                 # グローバル状態管理
├── utils/                   # ユーティリティ
│   ├── mod.rs               # ユーティリティエントリポイント
│   ├── websocket.rs         # WebSocketユーティリティ
```

API

1. ユーザー管理関連

- ユーザー登録
  - POST /api/users/register
    - 入力: { username: string, password: string, email: string }
    - 出力: { userId: string, username: string, token: string }
- ログイン
  - POST /api/users/login
    - 入力: { username: string, password: string }
    - 出力: { userId: string, username: string, token: string }
- ログアウト
  - POST /api/users/logout
    - 入力: { token: string }
    - 出力: { message: "Logged out successfully" }
- プロフィール取得
  - GET /api/users/{userId}
    - 出力: { userId: string, username: string, stats: { wins: number, losses: number } }

2. ロビー管理関連

- ロビー一覧取得
  - GET /api/room/rooms
    - 出力: [{ lobbyId: string, name: string, players: number, maxPlayers: number, status: string }]
- ロビー作成
  - POST /api/lobbies
    - 入力: { name: string, maxPlayers: number, rules: object }
    - 出力: { lobbyId: string, name: string, maxPlayers: number }
- ロビー情報取得
  - GET /api/lobbies/{lobbyId}
    - 出力: { lobbyId: string, name: string, players: [{ userId: string, username: string }], rules: object }
- ロビーへの参加
  - POST /api/lobbies/{lobbyId}/join
    - 入力: { userId: string }
    - 出力: { message: "Joined successfully" }
- ロビーから退出
  - POST /api/lobbies/{lobbyId}/leave
    - 入力: { userId: string }
    - 出力: { message: "Left successfully" }

3. ゲーム進行関連

- ゲーム開始
  - POST /api/lobbies/{lobbyId}/start
    - 入力: { userId: string }
    - 出力: { message: "Game started", gameId: string }
- ゲーム状態取得
  - GET /api/games/{gameId}
    - 出力: { gameId: string, status: string, players: [{ userId: string, role: string, alive: boolean }], logs: [] }
- プレイヤーの行動
  - POST /api/games/{gameId}/action
    - 入力: { userId: string, action: string, target: string }
    - 出力: { message: "Action recorded" }
  - 投票
    - POST /api/games/{gameId}/vote
      - 入力: { userId: string, target: string }
      - 出力: { message: "Vote recorded" }
  - ゲーム終了
    - POST /api/games/{gameId}/end
      - 入力: { userId: string }
      - 出力: { winner: "Villagers" | "Werewolves" }

4. リアルタイム通信 (WebSocket)
   - ロビー状態通知
     - イベント: lobby:update
     - 内容: { lobbyId: string, players: [{ userId: string, username: string }] }
   - ゲーム進行通知
     - イベント: game:update
     - 内容: { gameId: string, status: string, logs: [] }
   - 投票進行通知
     - イベント: game:vote
     - 内容: { gameId: string, votes: [{ userId: string, target: string }] }
   - 勝敗結果通知
     - イベント: game:result
     - 内容: { gameId: string, winner: string, logs: [] }
5. 管理者向け API
   - ルール設定
     - POST /api/lobbies/{lobbyId}/rules
       - 入力: { rules: object }
       - 出力: { message: "Rules updated" }
   - 不正プレイヤーのキック
     - POST /api/lobbies/{lobbyId}/kick
       - 入力: { adminId: string, targetUserId: string }
       - 出力: { message: "Player kicked" }
6. ゲームログ関連
   - 過去のゲームログ取得
     - GET /api/users/{userId}/logs
       - 出力: [{ gameId: string, result: string, date: string }]
   - 特定ゲームの詳細ログ
     - GET /api/games/{gameId}/logs
       - 出力: { gameId: string, logs: [{ round: number, actions: [], votes: [] }] }

追加ポイント

- セキュリティ

  - 認証（JWT やセッション管理）を通じて API の不正使用を防ぐ。
  - アクセス制限（ロビー参加者のみ特定 API を使用可能にするなど）。

- 拡張性

  - ルールカスタマイズの API を拡張可能にする。
  - 将来的にランキングやフレンド機能を追加。

- パフォーマンス
  - WebS ocket を活用してリアルタイム性を確保。
  - バックエンドで効率的な非同期処理を行う。
