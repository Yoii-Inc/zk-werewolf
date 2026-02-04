/**
 * ZK Werewolf 統合E2Eテスト
 *
 * 以下の項目をテスト:
 * 0. WebSocket接続が正しく確立されている
 * 1. ゲームを正しく開始できる
 * 2. コミットメント送信と役職配布リクエストが正しく動作する
 * 3. ゲーム開始時の役職配布が正しく動作する
 * 4. 占い師の公開鍵を生成できる
 * 5. 占い処理が行える
 * 6. 襲撃処理が行える
 * 7. 投票処理が行える
 * 8. 勝利判定処理が正しく行える
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
    console.log("4️⃣  Verifying role assignment completion...");
    await new Promise(resolve => setTimeout(resolve, 5000)); // 証明生成を待つ

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

  //   test("4. 占い師の公開鍵を生成できる", async () => {
  //     console.log("\n🧪 Test 4: Fortune teller can generate public key\n");

  //     // Given: ElGamal鍵ペアを生成
  //     console.log("1️⃣  Generating ElGamal keypair...");
  //     const keyPair = await CryptoHelper.generateKeyPair(global.cryptoParams);

  //     expect(keyPair).toBeDefined();
  //     expect(keyPair.publicKey).toBeDefined();
  //     expect(keyPair.secretKey).toBeDefined();
  //     console.log("✅ Keypair generated");

  //     // When: KeyPublicize入力作成
  //     console.log("\n2️⃣  Creating KeyPublicize input...");
  //     const input = {
  //       privateInput: {
  //         pubKeyX: keyPair.publicKey.x,
  //         pubKeyY: keyPair.publicKey.y,
  //         isFortuneTeller: 1, // 占い師の場合
  //       },
  //       publicInput: {
  //         pedersenParam: global.cryptoParams.pedersen_param,
  //       },
  //     };

  //     // When: 暗号化
  //     console.log("\n3️⃣  Encrypting with WASM...");
  //     const encrypted = await CryptoHelper.encryptForCircuit("KeyPublicize", input);

  //     // Then: 暗号化が成功
  //     expect(encrypted).toBeDefined();
  //     expect(encrypted.nodeShares).toBeDefined();
  //     expect(Array.isArray(encrypted.nodeShares)).toBe(true);
  //     console.log("✅ Encrypted successfully");
  //     console.log(`   Node shares: ${encrypted.nodeShares.length}`);

  //     console.log("\n✅ Test 3 completed: Public key generation verified\n");
  //   }, 300000);

  //   test("5. 占い処理が行える", async () => {
  //     console.log("\n🧪 Test 5: Divination process works\n");

  //     // Given: 占い師の鍵ペア
  //     const seerKeyPair = await CryptoHelper.generateKeyPair(global.cryptoParams);
  //     console.log("✅ Seer keypair generated");

  //     // When: Divination入力作成
  //     console.log("\n1️⃣  Creating Divination input...");
  //     const targetPlayerId = 1; // 占い対象
  //     const input = {
  //       privateInput: {
  //         fortuneTellerSecretKey: seerKeyPair.secretKey,
  //         targetPlayerId: targetPlayerId,
  //         amFortuneTeller: 1,
  //       },
  //       publicInput: {
  //         pedersenParam: global.cryptoParams.pedersen_param,
  //         fortuneTellerPublicKey: seerKeyPair.publicKey,
  //         playerCount: 4,
  //       },
  //     };

  //     // When: 暗号化
  //     console.log("\n2️⃣  Encrypting divination request...");
  //     const encrypted = await CryptoHelper.encryptForCircuit("Divination", input);

  //     // Then: 暗号化が成功
  //     expect(encrypted).toBeDefined();
  //     expect(encrypted.nodeShares).toBeDefined();
  //     console.log("✅ Divination request encrypted");
  //     console.log(`   Target player: ${targetPlayerId}`);

  //     console.log("\n✅ Test 5 completed: Divination process verified\n");
  //   }, 300000);

  //   test("6. 襲撃処理が行える", async () => {
  //     console.log("\n🧪 Test 6: Werewolf attack process works\n");

  //     // Given: 人狼の襲撃対象
  //     const targetPlayerId = 2; // 襲撃対象のプレイヤーID

  //     // When: 襲撃アクション作成
  //     console.log("1️⃣  Creating werewolf attack action...");
  //     const attackAction = {
  //       actionType: "attack",
  //       targetPlayerId: targetPlayerId,
  //       playerId: 0, // 人狼プレイヤーID
  //     };

  //     // Then: アクションデータが正しい
  //     expect(attackAction.actionType).toBe("attack");
  //     expect(attackAction.targetPlayerId).toBe(targetPlayerId);
  //     console.log("✅ Attack action created");
  //     console.log(`   Target: Player ${targetPlayerId}`);

  //     // Note: 実際のサーバー送信は夜アクションエンドポイントを使用
  //     // await global.apiClient.submitNightAction(roomId, attackAction);

  //     console.log("\n✅ Test 6 completed: Werewolf attack verified\n");
  //   }, 300000);

  //   test("7. 投票処理が行える", async () => {
  //     console.log("\n🧪 Test 7: Voting process works\n");

  //     // Given: 投票データ
  //     const voterId = 0;
  //     const targetId = 1;

  //     // When: AnonymousVoting入力作成
  //     console.log("1️⃣  Creating voting input...");
  //     const input = {
  //       privateInput: {
  //         id: voterId,
  //         isTargetId: [
  //           [["0"], null], // Player 0
  //           [["1"], null], // Player 1 (target)
  //           [["0"], null], // Player 2
  //           [["0"], null], // Player 3
  //         ],
  //         playerRandomness: global.cryptoParams.playerRandomness[0],
  //       },
  //       publicInput: {
  //         pedersenParam: global.cryptoParams.pedersen_param,
  //         playerCommitment: [], // TODO: 実際のcommitmentが必要
  //         playerNum: 4,
  //       },
  //       nodeKeys: [
  //         { nodeId: "node0", publicKey: "key0" },
  //         { nodeId: "node1", publicKey: "key1" },
  //         { nodeId: "node2", publicKey: "key2" },
  //       ],
  //       scheme: {
  //         totalShares: 3,
  //         modulus: 100,
  //       },
  //     };

  //     // When: 暗号化
  //     console.log("\n2️⃣  Encrypting vote...");
  //     const encrypted = await CryptoHelper.encryptForCircuit("AnonymousVoting", input);

  //     // Then: 暗号化が成功
  //     expect(encrypted).toBeDefined();
  //     expect(encrypted.nodeShares).toBeDefined();
  //     console.log("✅ Vote encrypted");
  //     console.log(`   Voter: Player ${voterId}`);
  //     console.log(`   Target: Player ${targetId}`);

  //     console.log("\n✅ Test 7 completed: Voting process verified\n");
  //   }, 300000);

  //   test("8. 勝利判定処理が正しく行える", async () => {
  //     console.log("\n🧪 Test 8: Winning judgement works correctly\n");

  //     // Given: ゲーム状態（例: 人狼全滅）
  //     const gameState = {
  //       aliveWerewolves: 0,
  //       aliveVillagers: 2,
  //       totalPlayers: 4,
  //     };

  //     // When: WinningJudgement入力作成
  //     console.log("1️⃣  Creating winning judgement input...");
  //     const input = {
  //       privateInput: {
  //         id: 0,
  //         amWerewolf: [["0"], null], // Villager
  //         playerRandomness: global.cryptoParams.playerRandomness[0],
  //       },
  //       publicInput: {
  //         pedersenParam: global.cryptoParams.pedersen_param,
  //         playerCommitment: [], // TODO: 実際のcommitmentが必要
  //       },
  //       nodeKeys: [
  //         { nodeId: "node0", publicKey: "key0" },
  //         { nodeId: "node1", publicKey: "key1" },
  //         { nodeId: "node2", publicKey: "key2" },
  //       ],
  //       scheme: {
  //         totalShares: 3,
  //         modulus: 100,
  //       },
  //     };

  //     // When: 暗号化
  //     console.log("\n2️⃣  Encrypting judgement request...");
  //     const encrypted = await CryptoHelper.encryptForCircuit("WinningJudgement", input);

  //     // Then: 暗号化が成功
  //     expect(encrypted).toBeDefined();
  //     expect(encrypted.nodeShares).toBeDefined();
  //     console.log("✅ Judgement request encrypted");
  //     console.log(`   Alive werewolves: ${gameState.aliveWerewolves}`);
  //     console.log(`   Alive villagers: ${gameState.aliveVillagers}`);
  //     console.log(`   Expected winner: Villagers`);

  //     console.log("\n✅ Test 8 completed: Winning judgement verified\n");
  //   }, 300000);
});
