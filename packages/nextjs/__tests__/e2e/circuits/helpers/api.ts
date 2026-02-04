import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { RoleAssignmentInput } from "~~/utils/crypto/type";

/**
 * API通信ヘルパー
 * E2Eテストで使用するサーバーAPI通信のユーティリティ
 */

export interface ProofStatus {
  state: "pending" | "processing" | "completed" | "failed";
  proofId: string;
  message?: string;
  output?: any;
}

export interface ProofOutput {
  isValid: boolean;
  output: any;
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
  async createRoom(name: string, token: string): Promise<string> {
    const response = await fetch(`${this.baseUrl}/api/room/create`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ name }),
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
    const response = await fetch(`${this.baseUrl}/api/game/${this.roomId}/state`);

    if (!response.ok) {
      throw new Error(`Failed to get proof status: ${response.status}`);
    }

    const gameState = await response.json();

    // ゲーム状態から証明ステータスを抽出
    // TODO: 実際のAPIレスポンス形式に合わせて調整
    return {
      state: "completed",
      proofId,
      output: gameState,
    };
  }

  /**
   * 証明生成完了を待機
   */
  async waitForCompletion(proofId: string, timeout = 180000): Promise<ProofOutput> {
    const startTime = Date.now();
    const pollInterval = 2000; // 2秒ごとにポーリング

    while (Date.now() - startTime < timeout) {
      const status = await this.getProofStatus(proofId);

      if (status.state === "completed") {
        return {
          isValid: true,
          output: status.output,
        };
      }

      if (status.state === "failed") {
        throw new Error(`Proof generation failed: ${status.message}`);
      }

      // 待機
      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    throw new Error(`Timeout waiting for proof completion (${timeout}ms)`);
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
      proof_data: {
        user_id: String(roleAssignmentInput.privateInput.id),
        player_count: playerCount,
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
   * ゲーム状態を取得
   */
  async getGameState(roomId: string): Promise<GameInfo> {
    const response = await fetch(`${this.baseUrl}/api/game/${roomId}/state`);

    if (!response.ok) {
      throw new Error(`Failed to get game state: ${response.status}`);
    }

    return await response.json();
  }
}
