import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { RoleAssignmentInput } from "~~/utils/crypto/type";

/**
 * API通信ヘルパー
 * E2Eテストで使用するサーバーAPI通信のユーティリティ
 */

export interface ProofStatus {
  state: "pending" | "running" | "completed" | "failed" | "timeout";
  proofId: string;
  message?: string;
  output?: any;
}

export interface ProofOutput {
  isValid: boolean;
  output: any;
}

export interface ProofJobNodeStatus {
  state: "pending" | "running" | "completed" | "failed" | "timeout";
  attempt_count: number;
  last_error: string | null;
}

export interface ProofJobStatus {
  state: "pending" | "running" | "completed" | "failed" | "timeout";
  batch_id: string;
  room_id: string;
  attempt_count: number;
  last_error: string | null;
  job_node_status: Record<string, ProofJobNodeStatus>;
  created_at: string;
  updated_at: string;
}

export interface AuthResponse {
  user: {
    id: string;
    username: string;
    email: string;
    created_at: string;
  };
  token: string;
}

export interface RoomResponse {
  room_id?: string;
  message?: string;
}

export interface CreateRoomOptions {
  maxPlayers?: number;
  roleConfig?: {
    seer: number;
    werewolf: number;
    villager: number;
  };
}

export class CircuitTestClient {
  private baseUrl: string;
  private roomId?: string;

  constructor(roomId?: string, baseUrl = "http://127.0.0.1:8080") {
    this.baseUrl = baseUrl;
    this.roomId = roomId;
  }

  setRoomId(roomId: string): void {
    this.roomId = roomId;
  }

  /**
   * サーバーの健全性チェック
   */
  async checkHealth(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/health`);
      return response.ok;
    } catch (error) {
      console.error("Health check failed:", error);
      return false;
    }
  }

  /**
   * ユーザー登録
   */
  async register(username: string, email: string, password: string): Promise<AuthResponse> {
    const response = await fetch(`${this.baseUrl}/api/users/register`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ username, email, password }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Registration failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * ログイン
   */
  async login(email: string, password: string): Promise<AuthResponse> {
    const response = await fetch(`${this.baseUrl}/api/users/login`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ email, password }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Login failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * ルーム作成
   */
  async createRoom(name: string, token: string, options?: CreateRoomOptions): Promise<string> {
    const payload: {
      name: string;
      max_players?: number;
      role_config?: {
        Seer: number;
        Werewolf: number;
        Villager: number;
      };
    } = { name };

    if (typeof options?.maxPlayers === "number") {
      payload.max_players = options.maxPlayers;
    }
    if (options?.roleConfig) {
      payload.role_config = {
        Seer: options.roleConfig.seer,
        Werewolf: options.roleConfig.werewolf,
        Villager: options.roleConfig.villager,
      };
    }

    const response = await fetch(`${this.baseUrl}/api/room/create`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Room creation failed (${response.status}): ${errorText}`);
    }

    const text = await response.text();
    // "Room created with ID: 123" から数字部分を抽出
    const match = text.match(/Room created with ID: (\d+)/);
    if (!match) {
      throw new Error(`Failed to parse room ID from response: ${text}`);
    }

    return match[1];
  }

  /**
   * ルーム参加
   */
  async joinRoom(roomId: string, playerId: string, token: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/room/${roomId}/join/${playerId}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to join room (${response.status}): ${errorText}`);
    }
  }

  /**
   * 準備完了トグル
   */
  async toggleReady(roomId: string, playerId: string, token: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/room/${roomId}/ready/${playerId}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to toggle ready (${response.status}): ${errorText}`);
    }
  }

  /**
   * ゲーム開始
   */
  async startGame(roomId: string, token: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/start`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to start game (${response.status}): ${errorText}`);
    }
  }

  /**
   * ルーム情報取得
   */
  async getRoomInfo(roomId: string): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/room/${roomId}`);

    if (!response.ok) {
      throw new Error(`Failed to get room info: ${response.status}`);
    }

    return await response.json();
  }

  /**
   * 証明リクエストを送信
   */
  async submitProof(data: any): Promise<{ proofId: string }> {
    const response = await fetch(`${this.baseUrl}/api/game/${this.roomId}/proof`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Proof submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   *if (!this.roomId) {
      throw new Error("Room ID not set. Call setRoomId() first.");
    }
     証明ステータスを取得
   */
  async getProofStatus(proofId: string): Promise<ProofStatus> {
    if (!this.roomId) {
      throw new Error("Room ID not set. Call setRoomId() first.");
    }

    const status = await this.getProofJobStatus(this.roomId, proofId);
    if (!status) {
      return { state: "pending", proofId };
    }

    return {
      state: status.state,
      proofId,
      message: status.last_error ?? undefined,
      output: status,
    };
  }

  async getProofJobStatus(roomId: string, batchId: string): Promise<ProofJobStatus | null> {
    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof/${batchId}/status`);

    if (response.status === 404) {
      return null;
    }

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to get proof job status (${response.status}): ${errorText}`);
    }

    return (await response.json()) as ProofJobStatus;
  }

  async waitForProofJobCompletion(roomId: string, batchId: string, timeout = 180000): Promise<ProofJobStatus> {
    const startTime = Date.now();
    const pollInterval = 1000;

    while (Date.now() - startTime < timeout) {
      const status = await this.getProofJobStatus(roomId, batchId);
      if (!status) {
        await new Promise(resolve => setTimeout(resolve, pollInterval));
        continue;
      }

      if (status.state === "completed") {
        return status;
      }

      if (status.state === "failed" || status.state === "timeout") {
        const nodeStates = Object.entries(status.job_node_status)
          .map(([nodeUrl, nodeStatus]) => `${nodeUrl}:${nodeStatus.state}`)
          .join(", ");
        throw new Error(
          `Proof job ${batchId} ${status.state}. last_error=${status.last_error ?? "none"} node_states=[${nodeStates}]`,
        );
      }

      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    throw new Error(`Timeout waiting for proof job completion (${timeout}ms): ${batchId}`);
  }

  /**
   * 証明生成完了を待機
   */
  async waitForCompletion(proofId: string, timeout = 180000): Promise<ProofOutput> {
    if (!this.roomId) {
      throw new Error("Room ID not set. Call setRoomId() first.");
    }

    const status = await this.waitForProofJobCompletion(this.roomId, proofId, timeout);
    return {
      isValid: status.state === "completed",
      output: status,
    };
  }

  /**
   * バッチリクエストをリセット（デバッグ用）
   */
  async resetBatch(): Promise<void> {
    if (!this.roomId) {
      throw new Error("Room ID not set. Call setRoomId() first.");
    }
    const response = await fetch(`${this.baseUrl}/api/game/${this.roomId}/debug/reset-batch`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (!response.ok) {
      if (!this.roomId) {
        throw new Error("Room ID not set. Call setRoomId() first.");
      }
      console.warn(`Failed to reset batch: ${response.status}`);
      // エラーでも続行（デバッグエンドポイントのため）
    }
  }

  /**
   * コミットメントを送信
   */
  async submitCommitment(data: any): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/game/${this.roomId}/commitment`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      throw new Error(`Commitment submission failed: ${response.status}`);
      if (!this.roomId) {
        throw new Error("Room ID not set. Call setRoomId() first.");
      }
    }

    return await response.json();
  }

  /**
   * 役職配布リクエストを送信
   * 本番環境では useRoleAssignment フックが行う処理
   */
  async submitRoleAssignment(
    roomId: string,
    roleAssignmentInput: RoleAssignmentInput,
    playerCount: number,
    authToken?: string,
    requesterPlayerId?: string,
  ): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    // MPCEncryption.encryptRoleAssignment() の結果を使用

    const encryptedRoleAssignment = await MPCEncryption.encryptRoleAssignment(roleAssignmentInput);

    const requestBody = {
      proof_type: "RoleAssignment",
      data: {
        user_id: requesterPlayerId ? String(requesterPlayerId) : String(roleAssignmentInput.privateInput.id),
        prover_count: playerCount,
        encrypted_data: encryptedRoleAssignment,
        public_key: roleAssignmentInput.publicKey,
      },
    };

    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof`, {
      method: "POST",
      headers,
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Role assignment submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * KeyPublicizeリクエストを送信
   * 本番環境では useKeyPublicize フックが行う処理
   */
  async submitKeyPublicize(
    roomId: string,
    keyPublicizeInput: any,
    playerCount: number,
    authToken?: string,
  ): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    // MPCEncryption.encryptKeyPublicize() の結果を使用
    const encryptedKeyPublicize = await MPCEncryption.encryptKeyPublicize(keyPublicizeInput);

    const requestBody = {
      proof_type: "KeyPublicize",
      data: {
        user_id: String(keyPublicizeInput.privateInput.id),
        prover_count: playerCount,
        encrypted_data: encryptedKeyPublicize,
      },
    };

    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof`, {
      method: "POST",
      headers,
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`KeyPublicize submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * Divinationリクエストを送信
   * 本番環境では useDivination フックが行う処理
   */
  async submitDivination(
    roomId: string,
    divinationInput: any,
    playerCount: number,
    authToken?: string,
    isDummy = false,
  ): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    // MPCEncryption.encryptDivination() の結果を使用
    const encryptedDivination = await MPCEncryption.encryptDivination(divinationInput);

    const requestBody = {
      proof_type: "Divination",
      data: {
        user_id: String(divinationInput.privateInput.id),
        prover_count: playerCount,
        encrypted_data: encryptedDivination,
        is_dummy: isDummy,
      },
    };

    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof`, {
      method: "POST",
      headers,
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Divination submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * AnonymousVotingリクエストを送信
   * 本番環境では useVoting フックが行う処理
   */
  async submitVoting(roomId: string, votingInput: any, playerCount: number, authToken?: string): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    // MPCEncryption.encryptAnonymousVoting() の結果を使用
    const encryptedVoting = await MPCEncryption.encryptAnonymousVoting(votingInput);

    const requestBody = {
      proof_type: "AnonymousVoting",
      data: {
        user_id: String(votingInput.privateInput.id),
        prover_count: playerCount,
        encrypted_data: encryptedVoting,
      },
    };

    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof`, {
      method: "POST",
      headers,
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Voting submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * WinningJudgementリクエストを送信
   * 本番環境では useWinningJudge フックが行う処理
   */
  async submitWinningJudgement(
    roomId: string,
    winningJudgementInput: any,
    playerCount: number,
    authToken?: string,
  ): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    if (authToken) {
      headers["Authorization"] = `Bearer ${authToken}`;
    }

    // MPCEncryption.encryptWinningJudgement() の結果を使用
    const encryptedWinningJudgement = await MPCEncryption.encryptWinningJudgement(winningJudgementInput);

    const requestBody = {
      proof_type: "WinningJudge",
      data: {
        user_id: String(winningJudgementInput.privateInput.id),
        prover_count: playerCount,
        encrypted_data: encryptedWinningJudgement,
      },
    };

    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/proof`, {
      method: "POST",
      headers,
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`WinningJudgement submission failed (${response.status}): ${errorText}`);
    }

    return await response.json();
  }

  /**
   * ゲーム状態を取得
   */
  async getGameState(roomId: string): Promise<GameInfo> {
    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/state`);

    if (!response.ok) {
      throw new Error(`Failed to get game state: ${response.status}`);
    }

    return await response.json();
  }

  /**
   * フェーズを次に進める（デバッグ用エンドポイント）
   */
  async advancePhase(roomId: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/phase/next`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to advance phase (${response.status}): ${errorText}`);
    }
  }
}
