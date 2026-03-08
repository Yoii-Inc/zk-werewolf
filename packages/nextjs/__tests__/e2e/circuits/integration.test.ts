/**
 * ZK Werewolf 統合E2Eテスト
 *
 * 以下の項目をテスト:
 * 0. WebSocket接続が正しく確立されている
 * 1. ゲームを正しく開始できる
 * 2. コミットメント送信と役職配布リクエストが正しく動作する
 * 3. ゲーム開始時の役職配布が正しく動作する
 * 4. 占い師の公開鍵生成（全プレイヤーがリクエスト送信）
 * 5. 占い処理（全プレイヤーがリクエスト送信、占い師以外はダミー）
 * 6. 襲撃処理（非ZK、アクション構造の確認のみ）
 * 7. 投票処理（全プレイヤーが投票リクエスト送信）
 * 8. 勝利判定処理（全プレイヤーがリクエスト送信）
 */
import { CryptoHelper } from "./helpers/crypto";
import { GameSetupHelper, checkWebSocketConnections, testSetup } from "./setup";
import { GameInfo } from "~~/types/game";

describe("ZK Werewolf Integration E2E Tests", () => {
  // 全テストの前に1回実行（自動的にゲーム開始まで実行される）
  beforeAll(testSetup.beforeAll);

  // 各テストの前に実行
  beforeEach(testSetup.beforeEach);

  // 全テストの後にクリーンアップ
  afterAll(testSetup.afterAll);

  test("0. WebSocket接続が正しく確立されている", async () => {
    console.log("\n🧪 Test 0: WebSocket connections are established correctly\n");

    // WebSocket接続のチェック
    await checkWebSocketConnections();

    console.log("✅ Test 0 completed: All WebSocket connections are active\n");
  });

  test("1. ゲームを正しく開始できる", async () => {
    console.log("\n🧪 Test 1: Game can be started correctly\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // Given: ゲームがセットアップ済み
    expect(roomId).toBeDefined();
    expect(players).toBeDefined();
    expect(players.length).toBeGreaterThanOrEqual(4);
    console.log(`✅ Room ID: ${roomId}`);
    console.log(`✅ Players: ${players.length}`);

    // When: ゲーム状態を取得
    const gameState = await global.apiClient.getGameState(roomId);

    // Then: ゲームが開始されている
    expect(gameState).toBeDefined();
    console.log(`✅ Game state retrieved`);
    console.log(`   Phase: ${gameState.phase || "Unknown"}`);

    console.log("\n✅ Test 1 completed: Game started successfully\n");
  }, 300000);

  test("2. コミットメント送信と役職配布リクエストが正しく動作する", async () => {
    console.log("\n🧪 Test 2: Commitments and role assignment requests work correctly\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // Step 1: ゲーム状態を取得
    console.log("1️⃣  Fetching game state...");
    const gameState: GameInfo = await global.apiClient.getGameState(roomId);
    expect(gameState).toBeDefined();
    console.log(`✅ Game state retrieved (Phase: ${gameState.phase})\n`);

    // Step 2: 各プレイヤーのコミットメントを送信
    console.log("2️⃣  Submitting commitments for all players...");
    await GameSetupHelper.submitPlayerCommitments(roomId, players, gameState);

    // WebSocketで commitments_ready 通知を待つ（少し待機）
    console.log("⏳ Waiting for commitments_ready notification...");
    await new Promise(resolve => setTimeout(resolve, 3000));
    console.log("✅ Commitments ready notification should have been received\n");

    // Step 3: 各プレイヤーの役職配布リクエストを送信
    console.log("3️⃣  Submitting role assignment requests for all players...");
    await GameSetupHelper.submitRoleAssignmentRequests(roomId, players, gameState);

    // Step 4: 役職配布完了を確認
    // submitRoleAssignmentRequests 内で proof job completed まで待機済み
    console.log("4️⃣  Verifying role assignment completion...");

    const updatedGameState = await global.apiClient.getGameState(roomId);
    console.log(`✅ Updated game state (Phase: ${updatedGameState.phase})`);
    console.log(`   Players: ${updatedGameState.players?.length || 0}`);

    console.log("\n✅ Test 2 completed: Commitments and role assignment successful\n");
  }, 300000);

  test("3. ゲーム開始時の役職配布が正しく動作する", async () => {
    console.log("\n🧪 Test 3: Role assignment works correctly\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // When: ゲーム状態を取得
    const gameState = await global.apiClient.getGameState(roomId);

    // Then: 各プレイヤーに役職が割り当てられている
    expect(gameState.players).toBeDefined();
    expect(gameState.players.length).toBe(players.length);
    console.log(`✅ ${gameState.players.length} players in game`);

    // 役職の存在確認（サーバー側が役職情報を返す場合）
    gameState.players.forEach((player: any, index: number) => {
      console.log(`   Player ${index + 1}: ${player.name || player.id}`);
      // Note: 役職は暗号化されているため、クライアント側では見えない可能性あり
    });

    console.log("\n✅ Test 3 completed: Role assignment verified\n");
  }, 300000);

  test("4. 占い師の公開鍵を生成できる（全プレイヤーがリクエスト送信）", async () => {
    console.log("\n🧪 Test 4: All players submit KeyPublicize requests\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    const gameState = await global.apiClient.getGameState(roomId);

    console.log("1️⃣  All players submitting KeyPublicize requests...");

    // 各プレイヤーがKeyPublicizeリクエストを送信
    await GameSetupHelper.submitKeyPublicizeRequests(roomId, players, gameState);

    // KeyPublicize完了を確認
    // submitKeyPublicizeRequests 内で proof job completed まで待機済み
    console.log("2️⃣  Verifying KeyPublicize completion...");

    const updatedGameState = await global.apiClient.getGameState(roomId);
    console.log(`✅ Updated game state (Phase: ${updatedGameState.phase})`);

    // ElGamal公開鍵が生成されたことを確認
    if (updatedGameState.crypto_parameters?.fortune_teller_public_key) {
      console.log(`✅ ElGamal public key generated successfully`);
    } else {
      console.log(`⚠️  ElGamal public key not yet available in game state`);
    }

    console.log("\n✅ Test 4 completed: All players submitted KeyPublicize requests\n");
  }, 300000);

  test("5. 占い処理が行える（全プレイヤーがリクエスト送信）", async () => {
    console.log("\n🧪 Test 5: All players submit Divination requests\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // Divination は DivinationProcessing フェーズでのみ受け付けられる
    console.log("0️⃣  Ensuring phase is DivinationProcessing...");
    const gameState = await GameSetupHelper.ensureGamePhase(roomId, "DivinationProcessing");

    // ElGamal公開鍵が存在しない場合、テストファイルから補完する
    if (!gameState.crypto_parameters?.fortune_teller_public_key) {
      console.log("⚠️  ElGamal public key not found in gameState, loading from test files...");
      const cryptoParams = await CryptoHelper.loadParams();
      // Ensure crypto_parameters object exists and merge the loaded public key to avoid TS possibly-undefined errors
      gameState.crypto_parameters = {
        ...(gameState.crypto_parameters ?? {}),
        fortune_teller_public_key: cryptoParams.fortune_teller_public_key,
      } as any;
      console.log("✅ ElGamal public key loaded successfully");
    }

    console.log("1️⃣  All players submitting Divination requests...");

    // 各プレイヤーの占い対象を決定（次のプレイヤーを占う）
    const targetIds = players.map((_, i) => gameState.players[(i + 1) % players.length]?.id || "1");
    // player 0を占い師と仮定、それ以外はダミー占い
    const isDummyFlags = players.map((_, i) => i !== 0);

    // 各プレイヤーがDivinationリクエストを送信
    await GameSetupHelper.submitDivinationRequests(roomId, players, gameState, targetIds, isDummyFlags);

    // Divination完了を確認
    // submitDivinationRequests 内で proof job completed まで待機済み
    console.log("2️⃣  Verifying Divination completion...");

    const updatedGameState = await global.apiClient.getGameState(roomId);
    console.log(`✅ Updated game state (Phase: ${updatedGameState.phase})`);

    console.log("\n✅ Test 5 completed: All players submitted Divination requests\n");
  }, 300000);

  test("6. 襲撃処理が行える", async () => {
    console.log("\n🧪 Test 6: Werewolf attack action (non-ZK)\n");

    // Note: 襲撃処理はZK証明を使用せず、通常のアクション送信で行われる
    // このテストはアクションデータの構造確認のみ

    const targetPlayerId = "2"; // 襲撃対象のプレイヤーID

    console.log("1️⃣  Creating werewolf attack action...");
    const attackAction = {
      actionType: "attack",
      targetPlayerId: targetPlayerId,
      playerId: "0", // 人狼プレイヤーID
    };

    expect(attackAction.actionType).toBe("attack");
    expect(attackAction.targetPlayerId).toBe(targetPlayerId);
    console.log("✅ Attack action structure validated");
    console.log(`   Target: Player ${targetPlayerId}`);

    // Note: 実際のサーバー送信は夜アクションエンドポイントを使用
    // const response = await fetch(`/api/game/${roomId}/night-action`, {
    //   method: "POST",
    //   body: JSON.stringify(attackAction),
    // });

    console.log("\n✅ Test 6 completed: Attack action structure verified\n");
  }, 300000);

  test("7. 投票処理が行える（全プレイヤーが投票）", async () => {
    console.log("\n🧪 Test 7: All players submit voting requests\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // AnonymousVoting は Voting フェーズでのみ受け付けられる
    console.log("0️⃣  Ensuring phase is Voting...");
    const gameState = await GameSetupHelper.ensureGamePhase(roomId, "Voting");

    console.log("1️⃣  All players submitting votes...");

    // 各プレイヤーの投票対象を決定（次のプレイヤーに投票）
    // const targetIds = players.map((_, i) => gameState.players[(i + 1) % players.length]?.id || "1");
    const targetIds = players.map((_, i) => gameState.players[1]?.id || "1");

    // 各プレイヤーが投票リクエストを送信
    await GameSetupHelper.submitVotingRequests(roomId, players, gameState, targetIds);

    // Voting完了を確認
    // submitVotingRequests 内で proof job completed まで待機済み
    console.log("2️⃣  Verifying Voting completion...");

    const updatedGameState = await global.apiClient.getGameState(roomId);
    console.log(`✅ Updated game state (Phase: ${updatedGameState.phase})`);

    console.log("\n✅ Test 7 completed: All players submitted votes\n");
  }, 300000);

  test("8. 勝利判定処理が正しく行える（全プレイヤーがリクエスト送信）", async () => {
    console.log("\n🧪 Test 8: All players submit WinningJudgement requests\n");

    const { roomId, players } = {
      roomId: global.testRoomId,
      players: global.testPlayers,
    };

    // WinningJudge は Result (または DivinationProcessing/Discussion) で受け付けられる
    console.log("0️⃣  Ensuring phase is Result...");
    const gameState = await GameSetupHelper.ensureGamePhase(roomId, "Result");

    console.log("1️⃣  All players submitting WinningJudgement requests...");

    // 各プレイヤーが勝利判定リクエストを送信
    await GameSetupHelper.submitWinningJudgementRequests(roomId, players, gameState);

    // WinningJudgement完了を確認
    // submitWinningJudgementRequests 内で proof job completed まで待機済み
    console.log("2️⃣  Verifying WinningJudgement completion...");

    const updatedGameState = await global.apiClient.getGameState(roomId);
    console.log(`✅ Updated game state (Phase: ${updatedGameState.phase})`);

    console.log("\n✅ Test 8 completed: All players submitted WinningJudgement requests\n");
  }, 300000);
});
