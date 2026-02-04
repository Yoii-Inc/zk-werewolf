# E2E Circuit Tests

ZK Werewolfのゲーム全体フローをテストする統合E2Eテスト。

## クイックスタート

### ローカル実行

```bash
# 1. サービス起動
docker compose -f docker-compose.test.yml up

# 2. テスト実行
cd packages/nextjs
npm run test:e2e:circuits

# 3. サービス停止
docker compose down
```

### GitHub Actions (CI)

- プッシュ時に自動実行
- 手動実行: Actionsタブから「E2E Circuit Tests」を選択

## テスト構成

- `integration.test.ts` - メインの統合テスト（ゲーム全フロー）
- `setup.ts` - 共通セットアップ（ルーム作成、プレイヤー参加、ゲーム開始）
- `helpers/` - ヘルパー関数
  - `api.ts` - APIクライアント
  - `crypto.ts` - 暗号化処理
  - `game-setup.ts` - ゲームセットアップ

## テスト内容

統合E2Eテストでは以下のゲームフローを順番にテストします：

1. **ゲーム開始** - ルーム作成、プレイヤー参加、ゲーム開始
2. **役職配布** - ゲーム開始時の自動役職配布を確認
3. **公開鍵生成** - 占い師の公開鍵生成（KeyPublicize）
4. **占い処理** - 占い師の占い実行（Divination）
5. **襲撃処理** - 人狼の襲撃アクション
6. **投票処理** - 匿名投票と集計（AnonymousVoting）
7. **勝利判定** - ゲーム終了と勝敗判定（WinningJudgement）

## デバッグ

### サービスログ確認

```bash
# サーバーログ
docker-compose logs -f backend

# MPCノードログ
docker-compose logs -f zk-mpc-node-0
docker-compose logs -f zk-mpc-node-1
docker-compose logs -f zk-mpc-node-2

# 全サービスログ
docker-compose logs -f
```

### サービス状態確認

```bash
# サービス状態
docker-compose ps

# ヘルスチェック
curl http://localhost:8080/health
```

### よくある問題

#### サービスが起動しない

```bash
# サービス再起動
docker-compose down
docker-compose up -d backend zk-mpc-node-0 zk-mpc-node-1 zk-mpc-node-2

# ログ確認
docker-compose logs backend
```

#### テストがタイムアウトする

- MPC証明生成には時間がかかります（数十秒〜数分）
- `testTimeout: 300000`（5分）が設定されています
- ネットワークやCPU負荷により時間が変動します

#### WebSocket接続エラー

- サーバーが完全に起動するまで待機してください
- `checkServicesHealth()`で自動的にリトライします（最大60秒）

## CI/CD

### GitHub Actions

ワークフロー: `.github/workflows/e2e-circuit-tests.yml`

- **トリガー**: `main`, `develop`ブランチへのpush、Pull Request
- **タイムアウト**: 30分
- **実行内容**:
  1. Node.js 20のセットアップ
  2. 依存関係インストール
  3. WASM ビルド
  4. Dockerサービス起動
  5. E2Eテスト実行
  6. ログ出力（失敗時）
  7. クリーンアップ

### ローカルでCIと同じ環境でテスト

```bash
# CI用のdocker-composeを使用
docker-compose -f docker-compose.ci.yml up -d

# テスト実行
cd packages/nextjs && npm run test:e2e:circuits

# クリーンアップ
docker-compose -f docker-compose.ci.yml down
```

## 参考資料

- [プランニングドキュメント](../../../plan/proof-verification-test.md)
- [既存の統合テスト](./integration.test.ts)
- [サーバーAPI仕様](../../../packages/server/src/routes/game.rs)
- [ZK-MPCノード実装](../../../packages/zk-mpc-node/src/node.rs)

### スクリプトを使用する場合

```bash
# ローカル実行用スクリプト（サービス起動確認あり）
./scripts/run-e2e-circuit-tests-local.sh
```

## テストの構造

```
__tests__/e2e/circuits/
├── setup.ts                 # 共通セットアップ
├── helpers/
│   ├── crypto.ts           # 暗号化ヘルパー
│   ├── api.ts              # APIクライアント
│   └── assertions.ts       # カスタムマッチャー
├── key-publicize.test.ts   # ✅ 実装済み
├── role-assignment.test.ts # TODO
├── divination.test.ts      # TODO
├── anonymous-voting.test.ts # TODO
└── winning-judgement.test.ts # TODO
```

## テストの書き方

### 基本パターン

```typescript
import { testSetup, CryptoHelper } from "../setup";

describe("YourCircuit E2E", () => {
  beforeAll(testSetup.beforeAll);
  beforeEach(testSetup.beforeEach);

  test("your test case", async () => {
    // 1. データ準備
    const input = { /* ... */ };

    // 2. 暗号化
    const encrypted = await CryptoHelper.encryptForCircuit("YourCircuit", input);

    // 3. サーバーへ送信
    const { proofId } = await global.apiClient.submitProof(encrypted);

    // 4. 完了待ち
    const result = await global.apiClient.waitForCompletion(proofId);

    // 5. 検証
    expect(result).toBeValidProof();
  });
});
```

## トラブルシューティング

### サービスが起動していない

```
❌ Server is not healthy
```

**解決策**: サーバーとノードを起動してください
```bash
make server  # Terminal 1
make node    # Terminal 2
```

### タイムアウトエラー

```
Timeout waiting for proof completion
```

**解決策**: 証明生成には時間がかかります
- デフォルトタイムアウト: 300秒（5分）
- 必要に応じて調整: `testTimeout: 600000` (10分)

### ノードが見つからない

**症状**: ノードへの接続エラー

**解決策**:
```bash
# ノードのログ確認
make node  # フォアグラウンドで実行してログ確認
```

### 暗号化エラー

**症状**: WASM関数でエラー

**解決策**:
1. WASMビルドを確認
```bash
cd packages/mpc-algebra-wasm
wasm-pack build --target nodejs --out-dir pkg-node
```

2. 暗号パラメータの読み込み確認

## デバッグ

### 詳細ログを有効化

```typescript
// setup.ts内でconsole.logを使用
console.log("Debug:", JSON.stringify(data, null, 2));
```

### 個別テスト実行

```bash
# 特定のテストのみ
npm run test:e2e:circuits:single -- "KeyPublicize"

# 特定のテストケースのみ
npm run test:e2e:circuits:single -- "占い師の公開鍵"
```

## CI/CD

GitHub Actionsで自動実行されます。

```yaml
# .github/workflows/e2e-circuit-tests.yml
- name: Run E2E Circuit Tests
  run: ./scripts/run-e2e-circuit-tests-ci.sh
```

## 参考資料

- [実装計画書](../../../plan/proof-verification-test.md)
- [既存divination.test.ts](./divination.test.ts)
- [サーバーAPI仕様](../../server/src/routes/game.rs)
