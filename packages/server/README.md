# werewolf-server 

## コマンド例
起動
```
cargo run
```

websocket connect
```
websocat ws://localhost:8080/api/room/ws
```

## API仕様

### ユーザー関連API

#### ユーザー登録
- POST /api/users/register
  - 入力: { username: string, email: string, password: string }
  - 出力: { user: UserResponse, token: string }

#### ユーザーログイン
- POST /api/users/login
  - 入力: { email: string, password: string }
  - 出力: { user: UserResponse, token: string }

#### ユーザー情報取得
- GET /api/users/{userId}
  - ヘッダー: Authorization: Bearer {token}
  - 出力: UserResponse

### ルーム関連API

#### ルーム作成
- POST /api/room/create
  - 出力: "Room created with ID: {roomId}"

#### ルーム一覧取得
- GET /api/room/rooms
  - 出力: { [roomId: string]: RoomInfo }

#### 特定のルーム情報取得
- GET /api/room/{roomId}
  - 出力: RoomInfo

#### ルーム参加
- POST /api/room/{roomId}/join/{playerId}
  - 出力: "Successfully joined room" | エラーメッセージ

#### ルーム退出
- POST /api/room/{roomId}/leave/{playerId}
  - 出力: "Successfully left room" | エラーメッセージ

#### WebSocket接続
- GET /api/room/ws
  - WebSocketを通じてリアルタイムのルーム状態更新を受信

### ゲーム関連API

#### ゲーム開始
- POST /api/game/{roomId}/start
  - 出力: ゲーム開始結果

#### ゲーム終了
- POST /api/game/{roomId}/end
  - 出力: ゲーム終了結果

#### ゲーム状態取得
- GET /api/game/{roomId}/state
  - 出力: 現在のゲーム状態

#### ゲームアクション
- POST /api/game/{roomId}/actions/vote
  - 入力: { voter_id: string, target_id: string }
  - 出力: 投票結果

- POST /api/game/{roomId}/actions/night-action
  - 入力: NightActionRequest
  - 出力: 夜行動の結果

#### フェーズ管理
- POST /api/game/{roomId}/phase/next
  - 出力: 次のフェーズへの移行結果

#### 勝利判定
- GET /api/game/{roomId}/check-winner
  - 出力: 勝利判定結果

## Supabaseの設定

このアプリケーションはSupabaseをデータストアとして使用しています。

### セットアップ手順

1. Supabaseでプロジェクトを作成します
2. `supabase/migrations/create_users_table.sql`に含まれるSQLをSupabaseのSQL Editorで実行します
3. `.env`ファイルに以下の環境変数を設定します:
   ```
   SUPABASE_URL=https://your-project-id.supabase.co
   SUPABASE_KEY=your-supabase-anon-key
   JWT_SECRET=your-secure-jwt-secret
   ```

## 以前は未実装だったAPI（現在実装済み）

1. ユーザー管理関連
   - POST /api/users/register ✅
   - POST /api/users/login ✅
   - GET /api/users/{userId} ✅

## まだ未実装のAPI

1. ルーム管理関連（追加機能）
   - DELETE /api/room/{roomId}/delete

2. 管理者向け機能
   - POST /api/lobbies/{lobbyId}/rules
   - POST /api/lobbies/{lobbyId}/kick

3. ゲームログ関連
   - GET /api/users/{userId}/logs
   - GET /api/games/{gameId}/logs
