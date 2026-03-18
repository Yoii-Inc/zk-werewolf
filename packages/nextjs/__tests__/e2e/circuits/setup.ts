/**
 * E2Eテスト共通セットアップ
 * 各テストファイルで使用する共通の初期化処理
 */
import { CircuitTestClient } from "./helpers/api";
import { GameSetupHelper, type TestPlayer } from "./helpers/game-setup";
import type { CryptoParameters } from "~~/types/game";
import { CryptoManager } from "~~/utils/crypto/encryption";
import { getPrivateGameInfo, setPrivateGameInfo, updatePrivateGameInfo } from "~~/utils/privateGameInfoUtils";

let isTearingDown = false;

export interface TestSetupOptions {
  numPlayers?: number;
  werewolfCount?: number;
  roomNamePrefix?: string;
}

// ============================================================================
// localStorage / sessionStorage モック（Node.js環境用）
// ============================================================================
class StorageMock {
  private store: Map<string, string> = new Map();

  getItem(key: string): string | null {
    return this.store.get(key) || null;
  }

  setItem(key: string, value: string): void {
    this.store.set(key, value);
  }

  removeItem(key: string): void {
    this.store.delete(key);
  }

  clear(): void {
    this.store.clear();
  }

  key(index: number): string | null {
    const keys = Array.from(this.store.keys());
    return keys[index] || null;
  }

  get length(): number {
    return this.store.size;
  }
}

// グローバルにlocalStorageとsessionStorageを設定
if (typeof window === "undefined") {
  (global as any).localStorage = new StorageMock();
  (global as any).sessionStorage = new StorageMock();
  console.log("✅ localStorage and sessionStorage mocks initialized for E2E tests");
}

// グローバル型定義拡張
// Note: `var` is required in global declarations (not `let`/`const`)
// eslint-disable-next-line @typescript-eslint/no-namespace
declare global {
  // eslint-disable-next-line no-var
  var cryptoParams: CryptoParameters & {
    playerRandomness?: any[]; // テスト用: プレイヤーごとのランダムネス
  };
  // eslint-disable-next-line no-var
  var apiClient: CircuitTestClient;
  // eslint-disable-next-line no-var
  var testRoomId: string;
  // eslint-disable-next-line no-var
  var testPlayers: TestPlayer[];
  // eslint-disable-next-line no-var
  var testSockets: any[] | undefined;
  // eslint-disable-next-line no-var
  var roleAssignmentDeliveries:
    | Array<{ receiverPlayerId: string; targetPlayerId?: string; batchId?: string }>
    | undefined;
}

export { GameSetupHelper };

function getMpcNodePublicKeys(): string[] {
  return [
    process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "",
    process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "",
    process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "",
  ];
}

const BN254_SCALAR_MODULUS = BigInt("21888242871839275222246405745257275088548364400416034343698204186575808495617");

const normalizeFieldElement = (value: bigint): bigint => {
  const reduced = value % BN254_SCALAR_MODULUS;
  return reduced >= 0n ? reduced : reduced + BN254_SCALAR_MODULUS;
};

function decodeRoleName(roleValue: bigint): "Villager" | "Seer" | "Werewolf" {
  const roleId = normalizeFieldElement(roleValue) % 3n;
  if (roleId === 1n) return "Seer";
  if (roleId === 2n) return "Werewolf";
  return "Villager";
}

const roleShareBuffers = new Map<
  string,
  {
    requiredShares: number;
    roleSharesByNode: Map<number, bigint>;
    werewolfMaskSharesByNode: Map<number, bigint>;
    completed: boolean;
  }
>();

function decodeWerewolfTeammateIdsForTest(
  maskValue: bigint,
  myRole: "Villager" | "Seer" | "Werewolf",
  myPlayerId: string,
  playerOrderIds?: string[],
): string[] {
  if (myRole !== "Werewolf") {
    return [];
  }

  const fallbackPlayers = global.testPlayers ?? [];
  const orderedPlayerIds =
    playerOrderIds && playerOrderIds.length > 0 ? playerOrderIds : fallbackPlayers.map(player => player.id);
  const normalizedMask = normalizeFieldElement(maskValue);
  const teammateIds: string[] = [];
  for (let index = 0; index < orderedPlayerIds.length; index += 1) {
    const bit = (normalizedMask >> BigInt(index)) & 1n;
    if (bit !== 1n) continue;
    const teammateId = orderedPlayerIds[index];
    if (!teammateId || teammateId === myPlayerId) continue;
    teammateIds.push(teammateId);
  }
  return Array.from(new Set(teammateIds));
}

function reflectRoleAssignmentForPlayer(roomId: string, playerId: string, payload: any): void {
  const targetPlayerId = payload?.target_player_id as string | undefined;
  if (targetPlayerId && targetPlayerId !== playerId) {
    return;
  }

  const encryptedRoleShare = payload?.result_data?.encrypted_role_share;
  if (!encryptedRoleShare) {
    return;
  }
  const playerOrderIds =
    Array.isArray(payload?.result_data?.player_order) && payload.result_data.player_order.length > 0
      ? payload.result_data.player_order.map((id: unknown) => String(id))
      : undefined;

  const encrypted = encryptedRoleShare.encrypted as string | undefined;
  const nonce = encryptedRoleShare.nonce as string | undefined;
  const nodeIdRaw = encryptedRoleShare.node_id as number | string | undefined;
  const requiredSharesRaw = encryptedRoleShare.required_shares as number | string | undefined;
  const shareEncoding =
    (encryptedRoleShare.role_share_encoding as string | undefined) ||
    (encryptedRoleShare.share_encoding as string | undefined);
  const werewolfMaskShareEncoding = encryptedRoleShare.werewolf_mates_mask_share_encoding as string | undefined;
  const nodeId = typeof nodeIdRaw === "string" ? Number(nodeIdRaw) : nodeIdRaw;
  const requiredShares = typeof requiredSharesRaw === "string" ? Number(requiredSharesRaw) : requiredSharesRaw;
  const batchId = payload?.batch_id as string | undefined;

  if (!encrypted || !nonce || typeof nodeId !== "number") {
    return;
  }
  if (!Number.isFinite(requiredShares) || (requiredShares ?? 0) <= 0) {
    return;
  }
  if (shareEncoding && shareEncoding !== "bn254_fr_decimal_string") {
    return;
  }
  if (werewolfMaskShareEncoding && werewolfMaskShareEncoding !== "player_index_bitmask_lsb0") {
    return;
  }
  if (!batchId) {
    return;
  }

  const bufferKey = `${roomId}:${playerId}:${batchId}`;
  const existingBuffer = roleShareBuffers.get(bufferKey) ?? {
    requiredShares: requiredShares as number,
    roleSharesByNode: new Map<number, bigint>(),
    werewolfMaskSharesByNode: new Map<number, bigint>(),
    completed: false,
  };
  if (existingBuffer.completed || existingBuffer.roleSharesByNode.has(nodeId)) {
    return;
  }

  const senderPublicKey = getMpcNodePublicKeys()[nodeId];
  if (!senderPublicKey) {
    console.warn(`   ⚠️  Missing MPC node public key for node_id=${nodeId}`);
    return;
  }

  const cryptoManager = new CryptoManager(playerId);
  if (!cryptoManager.hasKeyPair()) {
    console.warn(`   ⚠️  Missing keypair for player ${playerId}; cannot decrypt role assignment yet`);
    return;
  }

  try {
    const decryptedBinary = cryptoManager.decryptBinary(encrypted, nonce, senderPublicKey);
    const decryptedString = new TextDecoder("utf-8").decode(decryptedBinary);
    const parsedShare = JSON.parse(decryptedString) as {
      role_share?: string;
      werewolf_mates_mask_share?: string;
    };
    if (!parsedShare || typeof parsedShare.role_share !== "string") {
      throw new Error("Invalid role payload");
    }
    const werewolfMaskShareString =
      typeof parsedShare.werewolf_mates_mask_share === "string" ? parsedShare.werewolf_mates_mask_share : "0";
    const shareValue = normalizeFieldElement(BigInt(parsedShare.role_share));
    const werewolfMaskShareValue = normalizeFieldElement(BigInt(werewolfMaskShareString));
    existingBuffer.requiredShares = Math.max(existingBuffer.requiredShares, requiredShares as number);
    existingBuffer.roleSharesByNode.set(nodeId, shareValue);
    existingBuffer.werewolfMaskSharesByNode.set(nodeId, werewolfMaskShareValue);
    roleShareBuffers.set(bufferKey, existingBuffer);

    if (existingBuffer.roleSharesByNode.size < existingBuffer.requiredShares) {
      return;
    }

    let combinedShare = 0n;
    for (const share of existingBuffer.roleSharesByNode.values()) {
      combinedShare = normalizeFieldElement(combinedShare + share);
    }

    let combinedWerewolfMask = 0n;
    for (const share of existingBuffer.werewolfMaskSharesByNode.values()) {
      combinedWerewolfMask = normalizeFieldElement(combinedWerewolfMask + share);
    }
    const roleName = decodeRoleName(combinedShare);
    const werewolfTeammateIds = decodeWerewolfTeammateIdsForTest(
      combinedWerewolfMask,
      roleName,
      playerId,
      playerOrderIds,
    );
    const existingInfo = getPrivateGameInfo(roomId, playerId);

    if (!existingInfo) {
      setPrivateGameInfo(roomId, {
        playerId,
        playerRole: roleName as any,
        werewolfTeammateIds,
        hasActed: false,
      });
    } else if (
      existingInfo.playerRole !== roleName ||
      JSON.stringify(existingInfo.werewolfTeammateIds ?? []) !== JSON.stringify(werewolfTeammateIds)
    ) {
      updatePrivateGameInfo(roomId, playerId, { playerRole: roleName as any, werewolfTeammateIds });
    }

    existingBuffer.completed = true;
    roleShareBuffers.set(bufferKey, existingBuffer);

    console.log(`   🔐 [${playerId}] Role reflected from encrypted assignment: ${roleName}`);
  } catch (error) {
    console.warn(
      `   ⚠️  Failed to reflect role assignment for player ${playerId}:`,
      error instanceof Error ? error.message : error,
    );
  }
}

/**
 * テスト用のWebSocket接続を開く
 * 本番環境と同じように、メッセージハンドラを設定して通知を受け取れるようにする
 */
async function openTestWebSockets(roomId: string, players: TestPlayer[]): Promise<any[]> {
  console.log("🔌 Opening WebSocket connections...");

  let WS: any = null;
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires, @typescript-eslint/no-unsafe-assignment
    WS = require("ws");
  } catch (e) {
    WS = (global as any).WebSocket || null;
  }

  if (!WS) {
    console.warn("⚠️  'ws' package not available and no global WebSocket; skipping WebSocket connections");
    return [];
  }

  const sockets: any[] = [];
  const wsConstructor = WS.WebSocket ? WS.WebSocket : WS;
  const wsBaseUrl = `${process.env.NEXT_PUBLIC_WS_URL || "ws://127.0.0.1:8080/api"}/room/${roomId}/ws`;

  console.log(`   Connecting to: ${wsBaseUrl}`);

  // 各プレイヤーのWebSocket接続を並列で開く
  const connectionPromises = players.map(
    player =>
      new Promise<any>((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error(`WebSocket connection timeout for ${player.name}`));
        }, 5000);

        try {
          const wsUrl = `${wsBaseUrl}?player_id=${encodeURIComponent(player.id)}`;
          // eslint-disable-next-line @typescript-eslint/no-unsafe-call
          const socket: any = new wsConstructor(wsUrl);

          const onOpen = () => {
            clearTimeout(timeout);
            console.log(`   ✅ WebSocket connected for ${player.name}`);
            resolve(socket);
          };

          const onError = (error: any) => {
            clearTimeout(timeout);
            console.warn(`   ⚠️  WebSocket error for ${player.name}:`, error);
            reject(error);
          };

          // メッセージハンドラ - 本番環境と同じようにフェーズ変更などの通知を処理
          const onMessage = (event: any) => {
            if (isTearingDown) return;
            try {
              // Node.js 'ws' の場合は event.data が Buffer なので文字列に変換
              const dataStr = typeof event === "string" ? event : event.data?.toString() || event.toString();
              const rawData = JSON.parse(dataStr);
              const data = rawData?.payload && rawData?.event_id ? rawData.payload : rawData;

              console.log(`   📩 [${player.name}] WebSocket message:`, data.message_type);

              // フェーズ変更通知の場合
              if (data.message_type === "phase_change") {
                console.log(`   🔄 [${player.name}] Phase change: ${data.from_phase} → ${data.to_phase}`);
              }

              // コミットメント準備完了通知の場合
              if (data.message_type === "commitments_ready") {
                console.log(
                  `   ✅ [${player.name}] Commitments ready: ${data.commitments_count}/${data.total_players}`,
                );
              }

              // 計算結果通知の場合 - 役職配布の結果をログに記録
              if (data.message_type === "computation_result") {
                console.log(`   🧮 [${player.name}] Computation result: ${data.computation_type}`);

                // 役職配布の場合、計算結果を受け取ったことをログに記録
                // 注: E2Eテストでは実際の復号化はuseComputationResultsフックが行う
                // ここではsessionStorageモックが正常に動作することを確認するためのログ出力のみ
                if (data.computation_type === "role_assignment" && data.target_player_id) {
                  if (!global.roleAssignmentDeliveries) {
                    global.roleAssignmentDeliveries = [];
                  }
                  global.roleAssignmentDeliveries.push({
                    receiverPlayerId: player.id,
                    targetPlayerId: String(data.target_player_id),
                    batchId: typeof data.batch_id === "string" ? data.batch_id : undefined,
                  });
                  console.log(
                    `   💾 [${player.name}] Role assignment result received for player_id: ${data.target_player_id}`,
                  );
                  console.log(
                    `   ℹ️  Note: Role decryption will be handled by useComputationResults hook with sessionStorage mock`,
                  );
                }

                if (data.computation_type === "role_assignment") {
                  reflectRoleAssignmentForPlayer(roomId, player.id, data);
                }
              }

              // ゲームリセット通知の場合
              if (data.message_type === "game_reset") {
                console.log(`   🔄 [${player.name}] Game reset notification`);
              }
            } catch (e) {
              console.warn(`   ⚠️  Failed to parse WebSocket message for ${player.name}:`, e);
            }
          };

          if (socket.addEventListener) {
            // ブラウザ互換のWebSocket API
            socket.addEventListener("open", onOpen);
            socket.addEventListener("error", onError);
            socket.addEventListener("message", onMessage);
            socket.addEventListener("close", () => {
              if (!isTearingDown) {
                console.log(`   🔌 WebSocket closed for ${player.name}`);
              }
            });
          } else {
            // Node.js 'ws' パッケージ
            socket.on("open", onOpen);
            socket.on("error", onError);
            socket.on("message", onMessage);
            socket.on("close", () => {
              if (!isTearingDown) {
                console.log(`   🔌 WebSocket closed for ${player.name}`);
              }
            });
          }

          sockets.push(socket);
        } catch (e) {
          clearTimeout(timeout);
          reject(e);
        }
      }),
  );

  try {
    await Promise.all(connectionPromises);
    console.log(`✅ All ${players.length} WebSocket connections established\n`);
    return sockets;
  } catch (error) {
    console.error("❌ Failed to establish WebSocket connections:", error);
    // 部分的に成功した接続をクリーンアップ
    sockets.forEach(socket => {
      try {
        if (socket?.terminate) socket.terminate();
        else if (socket?.close) socket.close();
      } catch (e) {
        // ignore
      }
    });
    throw error;
  }
}

/**
 * WebSocket接続が正しく確立されているかチェック
 */
export async function checkWebSocketConnections(): Promise<void> {
  console.log("\n🔍 Checking WebSocket connections...");

  if (!global.testSockets || global.testSockets.length === 0) {
    throw new Error("❌ No WebSocket connections found");
  }

  const allConnected = global.testSockets.every(socket => {
    if (!socket) return false;
    // Node.js 'ws' の場合は readyState をチェック
    const readyState = socket.readyState;
    const OPEN = socket.OPEN || 1; // WebSocket.OPEN = 1
    return readyState === OPEN;
  });

  if (!allConnected) {
    const states = global.testSockets.map(s => s?.readyState || "unknown");
    throw new Error(`❌ Not all WebSockets are connected. States: ${states.join(", ")}`);
  }

  console.log(`✅ All ${global.testSockets.length} WebSocket connections are active\n`);
}

/**
 * サービスの健全性をチェック
 */
async function checkServicesHealth(): Promise<void> {
  console.log("🔍 Checking services health...");

  const maxRetries = 30;
  const retryInterval = 2000; // 2秒

  for (let i = 0; i < maxRetries; i++) {
    try {
      const serverResponse = await fetch("http://127.0.0.1:8080/health");
      if (serverResponse.ok) {
        console.log("✅ Server is healthy");
        console.log("⚠️  Note: MPC nodes do not have health endpoints");
        console.log("   Assuming nodes are running if server is healthy\n");
        return;
      }
    } catch (error) {
      // サーバーがまだ起動していない
    }

    if (i < maxRetries - 1) {
      console.log(`⏳ Waiting for server to be ready... (${i + 1}/${maxRetries})`);
      await new Promise(resolve => setTimeout(resolve, retryInterval));
    }
  }

  throw new Error(
    `❌ Server is not healthy after ${maxRetries} retries.\n` +
      `Please ensure services are running:\n` +
      `  docker-compose up -d backend zk-mpc-node-0 zk-mpc-node-1 zk-mpc-node-2`,
  );
}

/**
 * テスト共通セットアップ
 */
export function createTestSetup(options?: TestSetupOptions) {
  const numPlayers = options?.numPlayers ?? 4;
  const roomNamePrefix = options?.roomNamePrefix ?? "E2E Test Room";

  return {
    /**
     * 全テストの前に1回だけ実行
     */
    beforeAll: async (): Promise<void> => {
      isTearingDown = false;
      console.log("\n🚀 Starting E2E Circuit Tests Setup...\n");
      console.log(
        `🧩 Scenario config: players=${numPlayers}, werewolf=${options?.werewolfCount ?? "auto"}, room="${roomNamePrefix}"`,
      );

      // サービス起動確認
      await checkServicesHealth();

      // テスト用プレイヤー・ルームのセットアップ
      console.log("👥 Setting up test players and room...");
      const { roomId, players } = await GameSetupHelper.setupPlayersAndRoom(numPlayers, {
        werewolfCount: options?.werewolfCount,
        roomNamePrefix,
      });
      global.testRoomId = roomId;
      global.testPlayers = players;
      console.log(`✅ Room created: ${roomId}, Players: ${players.length}\n`);

      // APIクライアント初期化
      global.apiClient = new CircuitTestClient(roomId);
      console.log("✅ API client initialized\n");

      // WebSocket接続（ゲーム開始前に確立する必要がある）
      global.testSockets = await openTestWebSockets(roomId, players);

      // ゲーム開始
      console.log("🎮 Starting game...");
      await GameSetupHelper.startGameWithPlayers(players, roomId);
      console.log("✅ Game started successfully!\n");

      // 暗号パラメータをサーバーから取得（本番環境と同じ）
      console.log("📦 Loading crypto parameters from server...");
      const gameState = await global.apiClient.getGameState(roomId);
      const GameInputGenerator = await import("~~/services/gameInputGenerator");
      global.cryptoParams = await GameInputGenerator.loadCryptoParams(gameState);

      // プレイヤーごとのランダムネスを生成
      console.log("🎲 Generating player randomness...");
      const MPCEncryption = (await import("~~/utils/crypto/InputEncryption")).MPCEncryption;
      global.cryptoParams.playerRandomness = await Promise.all(
        players.map(async () => {
          const rand = await MPCEncryption.frRand();
          return rand;
        }),
      );
      console.log("✅ Player randomness generated");

      console.log("✅ Crypto parameters loaded from server\n");

      console.log("✅ Setup completed!\n");
      global.roleAssignmentDeliveries = [];
    },

    /**
     * 各テストの前に実行
     */
    beforeEach: async (): Promise<void> => {
      roleShareBuffers.clear();
      global.roleAssignmentDeliveries = [];
      // バッチリセット（エラーが出ても続行）
      try {
        await global.apiClient.resetBatch();
      } catch (error) {
        console.warn("⚠️  Failed to reset batch (continuing anyway):", error);
      }
    },

    /**
     * 全テストの後にクリーンアップ
     */
    afterAll: async (): Promise<void> => {
      isTearingDown = true;
      console.log("\n🧹 Cleaning up test environment...");
      // Close any test WebSocket connections opened in beforeAll
      try {
        if (global.testSockets && Array.isArray(global.testSockets)) {
          console.log(`🔌 Closing ${global.testSockets.length} test WebSocket(s)...`);
          for (const s of global.testSockets) {
            try {
              if (!s) continue;
              // ws (Node) has terminate/close; browser WebSocket has close()
              if (typeof s.terminate === "function") {
                s.terminate();
              } else if (typeof s.close === "function") {
                s.close();
              }
            } catch (e) {
              console.warn("⚠️ Error while closing socket:", e);
            }
          }
          global.testSockets = [];
        }
      } catch (e) {
        console.warn("⚠️ Error during test sockets cleanup:", e);
      }

      // 必要に応じてルームの削除などを実装
      console.log("✅ Cleanup completed\n");
    },
  };
}

export const testSetup = createTestSetup({ numPlayers: 4, werewolfCount: 1, roomNamePrefix: "E2E Smoke Room" });

/**
 * ヘルパー関数をエクスポート
 */
export { CryptoHelper } from "./helpers/crypto";
export { CircuitTestClient } from "./helpers/api";
export type { ProofOutput, ProofStatus } from "./helpers/api";
