# zk-werewolf 実行計画（実装前レビュー版）

作成日: 2026-02-25

このドキュメントは、現状コードを踏まえて「どの順序で」「何を」「どこまで」実装するかを決めるための計画書です。
本ドキュメント作成時点では、実装変更は行っていません。

## 1. 前提とゴール

- ゴール
  - ZK/MPCの正当性を担保しつつ、ゲームとして成立する機能一式を本番運用可能な品質にする。
  - サーバー非秘匿情報のみ保持（役職・個別投票など秘匿情報はサーバー非可視）を徹底する。
  - チェーン記録、テスト、UX、運用を含む全体整合をとる。

- 非ゴール（当面）
  - UIの全面リデザインを最初に行うこと。
  - 新機能を先に増やしてからセキュリティ・正当性を後追いすること。

## 2. 現状の主要ギャップ（確認済み）

### 2.1 ZK回路・MPCまわり

- `RoleAssignment` の制約が大きく未完了
  - `packages/mpc-circuits/src/traits/circuits.rs` で `RoleAssignment` 含む各回路に `TODO` が多数。
  - 役職制約やコミットメント検証を「制約数削減のためスキップ」している箇所がある。

- コミットメント整合が弱い
  - フロント側 `generateRoleAssignmentInput` でコミットメント不足時にダミーへフォールバックしている。
  - これは「proofは通るが、期待セキュリティ性質を満たさない」状態を招く。

- 2狼以上での RoleAssignment 失敗の有力因子
  - フロント側 `maxGroupSize` 計算が `+1` 固定で、人狼複数時の群サイズと不整合になる可能性が高い。

### 2.2 役職秘匿・データ露出

- サーバー側に「暗号化配布経路」はあるが、平文フォールバック経路も残っている
  - `packages/server/src/models/game.rs` の `RoleAssignment` 処理で `output.shares` がない場合に従来処理へフォールバック。
  - 本番ではこのフォールバックを禁止すべき。

### 2.3 ゲーム機能の欠落/不整合

- 襲撃回路は限定的、護衛は未実装
  - サーバー夜アクションは `Attack` のみ。
  - `Role::Guard` は型定義にあるが、実フロー未接続。

- Ready機能が強制されない
  - フロントは全員Readyで強調表示するが、開始API自体はReady必須をサーバーで厳格化していない。

- 退室/削除APIは存在するがUI導線不足
  - `room` ルートには `leave/delete` があるが、画面操作上の機能不足が体感課題。

- 部屋作成時刻表示は未実装
  - `Room` に `created_at` がなく、フロントは `Created: Unknown` を表示。

- Record/Helpがダミー寄り
  - `record` はモックデータ。
  - `help` はScaffoldのテンプレート文面が残る。

### 2.4 同時実行・リアルタイム

- バッチ処理がロック内で重く、同時リクエストに弱い
  - `Game::add_request` から `process_batch().await` を同期実行しており、ゲーム状態ロック競合が発生しやすい。

- WebSocket自動再接続がない
  - `useGameWebSocket` は close/error後の再接続戦略なし。

- フェーズ自動進行が限定的
  - 一部遷移はイベント駆動で動くが、時間ベースの汎用自動進行は未完成。

### 2.5 ブロックチェーン・環境運用

- コントラクト/サーバ連携は骨格ありだが、運用整合は未完
  - オンチェーン記録・検証呼び出しは存在するが、UI可視化や履歴面は不足。

- `deploy.yml` は `dev/staging/prod` を受けるが、Terraform環境ディレクトリは `dev` のみ確認
  - staging/prodへ切替時の整備不足リスクが高い。

- 本番で `yarn chain` 相当が動かない懸念
  - チェーン依存（`cast` 等）を含む構成に対し、実行環境差分の検証タスクが必要。

### 2.6 テスト

- `integration_test.zsh` 依存が強く、運用性が低い
- Next.js E2Eは一部進んでいるが、README上で未着手項目が残る
- 同時実行/再接続/チェーン統合の網羅テストが不足

### 2.7 セキュリティ・軽量化・i18n

- localStorage利用範囲が広く、セッション分離/残留データ管理が弱い
- メモリ計測・最適化は未体系化
- i18n基盤（言語リソース/切替機構）は未整備

## 3. 優先順位（P0-P3）

- P0: 正当性と秘匿性を壊す項目
  - RoleAssignment制約完成
  - コミットメントの回路 enforce
  - 平文フォールバック排除
  - 2狼proof失敗の修正

- P1: ゲーム成立と運用安定
  - 同時リクエスト設計見直し
  - WebSocket再接続
  - フェーズ自動進行（時間ベース）
  - 護衛含む夜行動回路

- P2: ブロックチェーン一貫性・テスト刷新
  - state/commitment/proof/resultの可視化
  - Docker中心の統合テスト
  - testnet対応とExplorer確認

- P3: UX/パフォーマンス/i18n/コード整理
  - 1画面導線、レスポンシブ、音
  - 軽量化
  - 多言語化
  - 無駄コード削減

## 4. 実装フェーズ計画

## Phase 0: 仕様固定（1週間）

- 実施
  - 仕様ドキュメントを確定: 役職秘匿モデル、回路公開入力、コミットメント仕様、phase遷移仕様。
  - 「失敗してよいフォールバック」を明確に禁止。

- 完了条件
  - RoleAssignment/Commitment/Proof/配布のシーケンス図が確定。
  - 各サービス（frontend/server/node/contracts）の責務境界が明文化される。

## Phase 1: ZK/MPC正当性ハードニング（2-3週間）

- 実施
  - RoleAssignment制約を段階的に復元。
  - 制約過大問題に対し、以下のいずれかを採用。
    - 制約分割（回路分割 + proof chaining）
    - 入力サイズ削減（公開入力圧縮、行列表現見直し）
    - MPC転送方式最適化（chunk送信、フレーミング、バックプレッシャ）
  - player commitment を全関連回路で `enforce`。
  - フロントのダミーcommitmentフォールバックを廃止し、未揃い時は明確エラー。
  - `maxGroupSize` を固定値推定ではなく `groupingParameter` 由来で厳密算出。

- 完了条件
  - 2狼以上を含むケースで RoleAssignment proof が安定通過。
  - コミットメント不正ケースでproof失敗が再現される。

## Phase 2: 秘匿配布・ゲーム機能整備（1-2週間）

- 実施
  - 役職配布の平文フォールバックを本番コードから削除。
  - サーバーは暗号文中継のみ、クライアント復号のみを強制。
  - 襲撃/護衛回路とゲームロジックを追加。
  - Ready強制（全員Ready未満ではstart不可）をサーバーで担保。
  - 退室・部屋削除UI導線追加。

- 完了条件
  - サーバーのログ/状態に役職平文が一切現れない。
  - 護衛有無を含む夜フェーズの整合テストが通る。

## Phase 3: 同時実行・リアルタイム安定化（1-2週間）

- 実施
  - ルーム単位アクターモデルまたはジョブキュー化で、`process_batch` をロック外実行に変更。
  - バッチ冪等化（request key + batch state machine）。
  - WebSocket自動再接続 + 再同期API（最後に見たイベントID以降の差分取得）。
  - 部屋一覧のリアルタイム更新（WSまたは短周期ポーリング）。
  - 時間ベースphase自動進行をサーバー主導へ統一。

- 完了条件
  - 同時リクエスト負荷試験でハング/整合崩れが発生しない。
  - 切断復帰後にゲーム状態・ログが欠落しない。

## Phase 4: ブロックチェーン統合完成（1-2週間）

- 実施
  - state hash / commitment / proof / result 記録のUI可視化。
  - `record` ページをモックから実データへ移行。
  - ローカルExplorer導線整備、可能ならtestnet Explorer対応。
  - 勝利報酬のフローを最小実装（設定・分配・確認）。
  - `dev/staging/prod` 環境差分をTerraform・Secrets・Workflowで整合化。

- 完了条件
  - 1試合の全イベントをチェーン上で追跡できる。
  - deploy workflow の environment 選択で実際に各環境へデプロイ可能。

## Phase 5: テスト刷新（並行 + 1週間）

- 実施
  - `integration_test.zsh` を Docker Compose ベースの統合テストランナーへ置換。
  - テスト層を明確化。
    - Unit: 回路ロジック、変換、state hash
    - Integration: server-node連携、commitment/proof
    - E2E: 部屋作成から終了まで（複数人、複数狼、再接続含む）
  - CIで並列実行できるように分離。

- 完了条件
  - PR時に最小回帰セットが常時実行される。
  - flaky率が許容範囲（目標 < 2%）。

## Phase 6: UX/Perf/i18n/整理（1-2週間）

- 実施
  - レスポンシブ対応（mobile/tablet）。
  - 効果音/BGM（設定でON/OFF）。
  - i18n導入（ja/en切替）。
  - localStorage利用方針再設計（部屋単位・TTL・ゲーム終了時クリーンアップ）。
  - 不要コード・古いデバッグ経路を削減。

- 完了条件
  - 主要画面がモバイルで操作可能。
  - 言語切替が全主要導線に反映。

## 5. ユーザー指摘項目との対応マップ

- RoleAssignment制約反映: Phase 1
- RoleAssignment結果をサーバー非可視化: Phase 2
- Commitmentの回路利用/enforce/proof検証: Phase 1
- テスト更新・Docker化: Phase 5
- 襲撃/護衛回路: Phase 2
- 同時リクエスト設計: Phase 3
- チェーン保存/proof投稿/結果記録/表示/報酬: Phase 4
- UX改善（音/BGM、1画面志向、Help整備）: Phase 6
- 軽量化: Phase 6
- ローカライズ: Phase 6
- 無駄コード削減: Phase 6
- セキュリティ担保確認: Phase 1-6横断（最後に監査フェーズ）
- WS自動復帰: Phase 3
- モバイル/タブレット: Phase 6
- 退室/部屋削除: Phase 2
- 2狼proof失敗: Phase 1
- 占いログのリロード問題: Phase 3/6（保存方針再設計）
- 時間経過でphase更新: Phase 3
- 本番 `yarn chain` 問題: Phase 4
- recordダミーデータ: Phase 4
- 部屋created表示不具合: Phase 2
- localStorageログ挙動: Phase 6
- Ready形骸化: Phase 2
- ルーム自動表示更新: Phase 3

## 6. リスクと先回り策

- リスク: 制約追加でMPC転送量が再度破綻
  - 先回り: 制約測定・転送サイズ測定をCIに追加し、閾値超過を検知。

- リスク: 非同期化で状態競合が増える
  - 先回り: ルーム単位シリアライズ処理 + 明示状態遷移図 + 冪等化。

- リスク: 本番環境差分（staging/prod未整備）
  - 先回り: deploy前に環境ディレクトリと秘密情報運用を先に作る。

## 7. 直近の具体アクション（次の作業開始時）

- 1. Phase 0の仕様ドキュメントを `plan/` に分割作成
  - `plan/role-assignment-spec.md`
  - `plan/commitment-enforcement-spec.md`
  - `plan/realtime-consistency-spec.md`

- 2. 2狼失敗の再現テストを先に固定
  - 失敗再現テストを先に作り、修正のDone条件にする。

- 3. RoleAssignmentのフォールバック禁止を設計決定
  - 本番パスから平文配布を排除する方針をチーム合意。

- 4. 同時実行アーキテクチャ（ルーム単位キュー）を先に決める
  - 後からの全面改修を避けるため、Phase 1着手前に決定。

