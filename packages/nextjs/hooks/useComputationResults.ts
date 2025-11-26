import { useCallback, useEffect, useState } from "react";
import type { ChatMessage } from "~~/types/game";

interface ComputationResult {
  computationType: string;
  resultData: any;
  targetPlayerId?: string;
  batchId: string;
  timestamp: string;
}

interface DivinationResult {
  ciphertext: any;
  status: string;
}

interface RoleAssignmentResult {
  role_assignments: Array<{
    player_id: string;
    player_name: string;
    role: string;
  }>;
  status: string;
}

interface WinningJudgeResult {
  game_result: "VillagerWin" | "WerewolfWin" | "InProgress";
  game_state_value: string;
  status: string;
}

interface AnonymousVotingResult {
  executed_player_id: string;
  executed_player_name: string;
  status: string;
}

export const useComputationResults = (roomId: string, playerId: string, addMessage: (message: ChatMessage) => void) => {
  const [divinationResult, setDivinationResult] = useState<DivinationResult | null>(null);
  const [roleAssignmentResult, setRoleAssignmentResult] = useState<RoleAssignmentResult | null>(null);
  const [winningJudgeResult, setWinningJudgeResult] = useState<WinningJudgeResult | null>(null);
  const [votingResult, setVotingResult] = useState<AnonymousVotingResult | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  // WebSocketからの計算結果通知を処理
  useEffect(() => {
    const handleComputationResult = (event: Event) => {
      const customEvent = event as CustomEvent;
      const result: ComputationResult = customEvent.detail;

      console.log(`計算結果受信: ${result.computationType}`, result);

      // 対象プレイヤーのチェック（指定がある場合）
      if (result.targetPlayerId && result.targetPlayerId !== playerId) {
        return; // 自分宛てでない場合はスキップ
      }

      setIsProcessing(true);

      try {
        switch (result.computationType) {
          case "divination":
            handleDivinationResult(result.resultData);
            break;
          case "role_assignment":
            handleRoleAssignmentResult(result.resultData);
            break;
          case "winning_judge":
            handleWinningJudgeResult(result.resultData);
            break;
          case "anonymous_voting":
            handleVotingResult(result.resultData);
            break;
          default:
            console.warn("Unknown computation type:", result.computationType);
        }
      } catch (error) {
        console.error("計算結果の処理エラー:", error);
        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: `計算結果の処理に失敗しました: ${result.computationType}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      } finally {
        setIsProcessing(false);
      }
    };

    window.addEventListener("computationResultNotification", handleComputationResult);

    return () => {
      window.removeEventListener("computationResultNotification", handleComputationResult);
    };
  }, [playerId, addMessage]);

  // 占い結果の処理
  const handleDivinationResult = useCallback(
    (data: DivinationResult) => {
      setDivinationResult(data);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "占い結果が準備されました。復号化してください。",
        timestamp: new Date().toISOString(),
        type: "system",
      });
    },
    [addMessage],
  );

  // 役職配布結果の処理
  const handleRoleAssignmentResult = useCallback(
    (data: RoleAssignmentResult) => {
      setRoleAssignmentResult(data);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "役職配布が完了しました。",
        timestamp: new Date().toISOString(),
        type: "system",
      });
    },
    [addMessage],
  );

  // 勝利判定結果の処理
  const handleWinningJudgeResult = useCallback(
    (data: WinningJudgeResult) => {
      setWinningJudgeResult(data);

      if (data.game_result !== "InProgress") {
        const resultMessage = data.game_result === "VillagerWin" ? "村人陣営の勝利です！" : "人狼陣営の勝利です！";

        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: resultMessage,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    },
    [addMessage],
  );

  // 投票結果の処理
  const handleVotingResult = useCallback(
    (data: AnonymousVotingResult) => {
      setVotingResult(data);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: `${data.executed_player_name}が処刑されました。`,
        timestamp: new Date().toISOString(),
        type: "system",
      });
    },
    [addMessage],
  );

  // 占い結果の復号化処理
  const decryptDivinationResult = useCallback(
    async (privateKey: string) => {
      if (!divinationResult) {
        throw new Error("占い結果がありません");
      }

      try {
        const response = await fetch(`/api/game/${roomId}/divination/decrypt`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            player_id: playerId,
            private_key: privateKey,
          }),
        });

        if (!response.ok) {
          throw new Error(`復号化に失敗しました: ${response.statusText}`);
        }

        const decryptedResult = await response.json();

        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: `占い結果: ${decryptedResult.is_werewolf ? "人狼です" : "人狼ではありません"}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });

        return decryptedResult;
      } catch (error) {
        console.error("復号化エラー:", error);
        throw error;
      }
    },
    [divinationResult, roomId, playerId, addMessage],
  );

  return {
    divinationResult,
    roleAssignmentResult,
    winningJudgeResult,
    votingResult,
    isProcessing,
    decryptDivinationResult,
  };
};
