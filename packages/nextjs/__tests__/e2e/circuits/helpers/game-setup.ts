/**
 * ゲームセットアップヘルパー
 * E2Eテストで使用するゲーム準備の共通処理
 */
import { CircuitTestClient } from "./api";
import { GameInfo } from "~~/types/game";

export interface TestPlayer {
  id: string;
  name: string;
  email: string;
  token: string;
}

export class GameSetupHelper {
  /**
   * テスト用プレイヤー登録とルーム作成
   * @param numPlayers プレイヤー数
   * @returns ルームIDとプレイヤー情報の配列
   */
  static async setupPlayersAndRoom(numPlayers: number): Promise<{ roomId: string; players: TestPlayer[] }> {
    const apiClient = new CircuitTestClient();
    const players: TestPlayer[] = [];
    const timestamp = Date.now();

    console.log(`📝 Login ${numPlayers} test players...`);

    // テストでは既存ユーザーでログインする（登録は行わない）
    // accounts: test1@example.com, test2@example.com, ... password: password123
    const password = "password123";
    for (let i = 1; i <= numPlayers; i++) {
      const username = `TestPlayer${i}`;
      const email = `test${i}@example.com`;

      try {
        const authResponse = await apiClient.login(email, password);
        players.push({
          id: authResponse.user.id,
          name: authResponse.user.username || username,
          email: authResponse.user.email,
          token: authResponse.token,
        });
        console.log(`   ✅ ${username} logged in (ID: ${authResponse.user.id})`);
      } catch (error) {
        console.error(`   ❌ Failed to login ${username} (${email}):`, error);
        throw error;
      }
    }

    // ルーム作成（最初のプレイヤーが作成）
    console.log(`🏠 Creating test room...`);
    const roomName = `E2E Test Room ${timestamp}`;
    const roomId = await apiClient.createRoom(roomName, players[0].token);
    console.log(`   ✅ Room created: ${roomId} (${roomName})`);

    // 全プレイヤーがルームに参加
    console.log(`👥 Players joining room...`);
    for (const player of players) {
      try {
        await apiClient.joinRoom(roomId, player.id, player.token);
        console.log(`   ✅ ${player.name} joined`);
      } catch (error) {
        console.error(`   ❌ Failed to join room (${player.name}):`, error);
        throw error;
      }
    }

    return { roomId, players };
  }

  /**
   * ゲーム開始（全プレイヤー準備完了 → 開始）
   * @param players プレイヤー情報の配列
   * @param roomId ルームID
   */
  static async startGameWithPlayers(players: TestPlayer[], roomId: string): Promise<void> {
    const apiClient = new CircuitTestClient();

    console.log(`⏳ All players marking ready...`);

    // 全プレイヤーが準備完了
    for (const player of players) {
      try {
        await apiClient.toggleReady(roomId, player.id, player.token);
        console.log(`   ✅ ${player.name} is ready`);
      } catch (error) {
        console.error(`   ❌ Failed to toggle ready (${player.name}):`, error);
        throw error;
      }
    }

    // ゲーム開始（最初のプレイヤーが実行）
    console.log(`🎮 Starting game...`);
    try {
      await apiClient.startGame(roomId, players[0].token);
      console.log(`   ✅ Game started successfully!`);
    } catch (error) {
      console.error(`   ❌ Failed to start game:`, error);
      throw error;
    }

    // ゲーム開始後、少し待機（フェーズ変更処理のため）
    console.log(`⏳ Waiting for game phase change...`);
    await new Promise(resolve => setTimeout(resolve, 2000));
  }

  /**
   * 各プレイヤーのコミットメントを送信
   * 本番環境では GameInputGenerator.initializeGameCrypto() が自動で行う
   */
  static async submitPlayerCommitments(roomId: string, players: TestPlayer[], gameInfo: GameInfo): Promise<void> {
    console.log(`📤 Submitting commitments for all players...`);

    // GameInputGenerator を動的インポート（Next.jsのクライアントサイドモジュール）
    const GameInputGenerator = await import("~~/services/gameInputGenerator");

    for (const player of players) {
      try {
        console.log(`   🔄 Initializing crypto for ${player.name}...`);
        await GameInputGenerator.initializeGameCrypto(roomId, player.name, gameInfo);
        console.log(`   ✅ ${player.name} commitment submitted`);
      } catch (error) {
        console.error(`   ❌ Failed to submit commitment for ${player.name}:`, error);
        throw error;
      }
    }

    console.log(`✅ All commitments submitted\n`);
  }

  /**
   * 各プレイヤーの役職配布リクエストを送信
   * 本番環境では useGamePhase フックが WebSocket 通知を受けて自動で行う
   */
  static async submitRoleAssignmentRequests(roomId: string, players: TestPlayer[], gameInfo: any): Promise<void> {
    console.log(`📤 Submitting role assignment requests for all players...`);

    const GameInputGenerator = await import("~~/services/gameInputGenerator");
    const apiClient = new CircuitTestClient(roomId);

    for (const player of players) {
      try {
        console.log(`   🔄 Generating role assignment input for ${player.name}...`);
        const roleAssignmentInput = await GameInputGenerator.generateRoleAssignmentInput(roomId, player.name, gameInfo);

        console.log(`   📤 Submitting role assignment request for ${player.name}...`);
        await apiClient.submitRoleAssignment(roomId, roleAssignmentInput, players.length, player.token);
        console.log(`   ✅ ${player.name} role assignment request sent`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // 既に役職配布が完了している場合はスキップ
        if (errorMessage.includes("already been completed") || errorMessage.includes("already completed")) {
          console.log(`   ℹ️  ${player.name} role assignment already completed (skipping)`);
        } else {
          console.error(`   ❌ Failed to submit role assignment for ${player.name}:`, error);
          throw error;
        }
      }
    }

    console.log(`✅ All role assignment requests submitted\n`);
  }

  /**
   * 各プレイヤーのKeyPublicizeリクエストを送信
   * 本番環境では useGamePhase フックが役職配布完了後に自動で行う
   */
  static async submitKeyPublicizeRequests(roomId: string, players: TestPlayer[], gameInfo: any): Promise<void> {
    console.log(`📤 Submitting KeyPublicize requests for all players...`);

    const GameInputGenerator = await import("~~/services/gameInputGenerator");
    const apiClient = new CircuitTestClient(roomId);

    for (const player of players) {
      try {
        console.log(`   🔄 Generating KeyPublicize input for ${player.name}...`);
        const keyPublicizeInput = await GameInputGenerator.generateKeyPublicizeInput(roomId, player.name, gameInfo);

        console.log(`   📤 Submitting KeyPublicize request for ${player.name}...`);
        await apiClient.submitKeyPublicize(roomId, keyPublicizeInput, players.length, player.token);
        console.log(`   ✅ ${player.name} KeyPublicize request sent`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // 既にKeyPublicizeが完了している場合はスキップ
        if (errorMessage.includes("already been completed") || errorMessage.includes("already completed")) {
          console.log(`   ℹ️  ${player.name} KeyPublicize already completed (skipping)`);
        } else {
          console.error(`   ❌ Failed to submit KeyPublicize for ${player.name}:`, error);
          throw error;
        }
      }
    }

    console.log(`✅ All KeyPublicize requests submitted\n`);
  }

  /**
   * 各プレイヤーのDivinationリクエストを送信
   * 本番環境では useDivination フックが夜フェーズ時に占い師が実行
   */
  static async submitDivinationRequests(
    roomId: string,
    players: TestPlayer[],
    gameInfo: any,
    targetIds: string[],
    isDummyFlags: boolean[],
  ): Promise<void> {
    console.log(`📤 Submitting Divination requests for all players...`);

    const GameInputGenerator = await import("~~/services/gameInputGenerator");
    const apiClient = new CircuitTestClient(roomId);

    for (let i = 0; i < players.length; i++) {
      const player = players[i];
      const targetId = targetIds[i];
      const isDummy = isDummyFlags[i];

      try {
        console.log(`   🔄 Generating Divination input for ${player.name} (target: ${targetId}, dummy: ${isDummy})...`);
        const divinationInput = await GameInputGenerator.generateDivinationInput(
          roomId,
          player.name,
          gameInfo,
          targetId,
          isDummy,
        );

        console.log(`   📤 Submitting Divination request for ${player.name}...`);
        await apiClient.submitDivination(roomId, divinationInput, players.length, player.token, isDummy);
        console.log(`   ✅ ${player.name} Divination request sent`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // 既にDivinationが完了している場合はスキップ
        if (errorMessage.includes("already been completed") || errorMessage.includes("already completed")) {
          console.log(`   ℹ️  ${player.name} Divination already completed (skipping)`);
        } else {
          console.error(`   ❌ Failed to submit Divination for ${player.name}:`, error);
          throw error;
        }
      }
    }

    console.log(`✅ All Divination requests submitted\n`);
  }

  /**
   * 各プレイヤーのVotingリクエストを送信
   * 本番環境では useVoting フックが投票フェーズ時に実行
   */
  static async submitVotingRequests(
    roomId: string,
    players: TestPlayer[],
    gameInfo: any,
    targetIds: string[],
  ): Promise<void> {
    console.log(`📤 Submitting Voting requests for all players...`);

    const GameInputGenerator = await import("~~/services/gameInputGenerator");
    const apiClient = new CircuitTestClient(roomId);

    for (let i = 0; i < players.length; i++) {
      const player = players[i];
      const targetId = targetIds[i];

      try {
        console.log(`   🔄 Generating Voting input for ${player.name} (target: ${targetId})...`);
        const votingInput = await GameInputGenerator.generateVotingInput(roomId, player.name, gameInfo, targetId);

        console.log(`   📤 Submitting Voting request for ${player.name}...`);
        await apiClient.submitVoting(roomId, votingInput, players.length, player.token);
        console.log(`   ✅ ${player.name} Voting request sent`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // 既にVotingが完了している場合はスキップ
        if (errorMessage.includes("already been completed") || errorMessage.includes("already completed")) {
          console.log(`   ℹ️  ${player.name} Voting already completed (skipping)`);
        } else {
          console.error(`   ❌ Failed to submit Voting for ${player.name}:`, error);
          throw error;
        }
      }
    }

    console.log(`✅ All Voting requests submitted\n`);
  }

  /**
   * 各プレイヤーのWinningJudgementリクエストを送信
   * 本番環境では useWinningJudge フックが勝利判定フェーズ時に実行
   */
  static async submitWinningJudgementRequests(roomId: string, players: TestPlayer[], gameInfo: any): Promise<void> {
    console.log(`📤 Submitting WinningJudgement requests for all players...`);

    const GameInputGenerator = await import("~~/services/gameInputGenerator");
    const apiClient = new CircuitTestClient(roomId);

    for (const player of players) {
      try {
        console.log(`   🔄 Generating WinningJudgement input for ${player.name}...`);
        const winningJudgementInput = await GameInputGenerator.generateWinningJudgementInput(
          roomId,
          player.name,
          gameInfo,
        );

        console.log(`   📤 Submitting WinningJudgement request for ${player.name}...`);
        await apiClient.submitWinningJudgement(roomId, winningJudgementInput, players.length, player.token);
        console.log(`   ✅ ${player.name} WinningJudgement request sent`);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);

        // 既にWinningJudgementが完了している場合はスキップ
        if (errorMessage.includes("already been completed") || errorMessage.includes("already completed")) {
          console.log(`   ℹ️  ${player.name} WinningJudgement already completed (skipping)`);
        } else {
          console.error(`   ❌ Failed to submit WinningJudgement for ${player.name}:`, error);
          throw error;
        }
      }
    }

    console.log(`✅ All WinningJudgement requests submitted\n`);
  }

  /**
   * ルーム情報を取得して状態を確認
   */
  static async waitForRoomStatus(
    roomId: string,
    expectedStatus: "Open" | "InProgress" | "Closed",
    timeout = 10000,
  ): Promise<void> {
    const apiClient = new CircuitTestClient();
    const startTime = Date.now();
    const pollInterval = 1000;

    while (Date.now() - startTime < timeout) {
      const roomInfo = await apiClient.getRoomInfo(roomId);

      if (roomInfo.status === expectedStatus) {
        console.log(`   ✅ Room status is now "${expectedStatus}"`);
        return;
      }

      console.log(`   ⏳ Waiting for room status "${expectedStatus}" (current: ${roomInfo.status})`);
      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    throw new Error(`Timeout waiting for room status "${expectedStatus}"`);
  }
}
