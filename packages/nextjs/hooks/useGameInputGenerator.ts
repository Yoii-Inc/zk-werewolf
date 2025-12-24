import { useEffect, useState } from "react";
import * as GameInput from "~~/services/gameInputGenerator";
import { GameInfo } from "~~/types/game";

// ゲーム入力生成を管理するフック（簡略化版）
export const useGameInputGenerator = (roomId: string, username: string, gameInfo: GameInfo | null = null) => {
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    if (!gameInfo || !roomId || !username) return;

    // 既に初期化済みの場合はスキップ
    if (GameInput.isInitialized(roomId, username)) {
      setIsReady(true);
      return;
    }

    // 初回のみ初期化を実行
    GameInput.initializeGameCrypto(roomId, username, gameInfo)
      .then(() => {
        setIsReady(true);
        console.log("Game crypto initialized successfully");
      })
      .catch(error => {
        console.error("Failed to initialize game crypto:", error);
        setIsReady(false);
      });
    // gameInfoが後から取得される場合もあるため依存配列に含める
    // ただし初期化済みチェックで重複実行は防ぐ
  }, [roomId, username, gameInfo]);

  return {
    isReady,
    // 各入力生成関数をそのまま返す
    generateRoleAssignmentInput: () => {
      if (!gameInfo) {
        throw new Error("gameInfo is not available for generateRoleAssignmentInput");
      }
      return GameInput.generateRoleAssignmentInput(roomId, username, gameInfo);
    },
    generateDivinationInput: (targetId: string, isDummy: boolean) => {
      if (!gameInfo) {
        throw new Error("gameInfo is not available for generateDivinationInput");
      }
      return GameInput.generateDivinationInput(roomId, username, gameInfo, targetId, isDummy);
    },
    generateVotingInput: (votedForId: string) => {
      if (!gameInfo) {
        throw new Error("gameInfo is not available for generateVotingInput");
      }
      return GameInput.generateVotingInput(roomId, username, gameInfo, votedForId);
    },
    encryptVotingData: (votedForId: string) => {
      if (!gameInfo) {
        throw new Error("gameInfo is not available for encryptVotingData");
      }
      return GameInput.encryptVotingData(roomId, username, gameInfo, votedForId);
    },
    generateWinningJudgementInput: () => {
      if (!gameInfo) {
        throw new Error("gameInfo is not available for generateWinningJudgementInput");
      }
      return GameInput.generateWinningJudgementInput(roomId, username, gameInfo);
    },
  };
};

// 後方互換性のため残す（useGameCrypto）
export const useGameCrypto = (roomId: string, gameInfo: GameInfo | null = null) => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refetch = async () => {
    setLoading(true);
    setError(null);
    try {
      await GameInput.loadCryptoParams();
      return { success: true };
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load crypto params");
      return { success: false };
    } finally {
      setLoading(false);
    }
  };

  return {
    loading,
    error,
    refetch,
  };
};
