import { useState } from "react";
import { NightAction, NightActionRequest } from "~~/app/room/types";
import type { ChatMessage, GameInfo, PrivateGameInfo } from "~~/types/game";
import {
  clearPrivateGameInfo,
  initializePrivateGameInfo,
  setPrivateGameInfo as saveToStorage,
  updateHasActed,
  updatePrivateGameInfo,
} from "~~/utils/privateGameInfoUtils";

export const useGameActions = (
  roomId: string,
  addMessage: (message: ChatMessage) => void,
  gameInfo: GameInfo | null,
  userId?: string,
) => {
  const [isStarting, setIsStarting] = useState(false);

  const startGame = async () => {
    setIsStarting(true);
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/start`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("ゲームの開始に失敗しました");
      }

      // ゲーム開始時にPrivateGameInfoを役職null（未決定）で初期化
      if (userId) {
        try {
          initializePrivateGameInfo(roomId, userId);
          console.log("PrivateGameInfo initialized with null role in session storage");

          addMessage({
            id: Date.now().toString(),
            sender: "システム",
            message: "ゲームが開始されました。役職配布をお待ちください...",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (storageError) {
          console.error("PrivateGameInfo初期化エラー:", storageError);
        }
      }

      return true;
    } catch (error) {
      console.error("ゲーム開始エラー:", error);
      return false;
    } finally {
      setIsStarting(false);
    }
  };

  //   const handleNightAction = async (targetPlayerId: string, userRole?: string) => {
  //     try {
  //       if (!gameInfo) {
  //         throw new Error("ゲーム情報が取得できません");
  //       }

  //       const action: NightAction = (() => {
  //         switch (userRole) {
  //           case "Werewolf":
  //             return { Attack: { target_id: targetPlayerId } };
  //           case "Seer":
  //             return { Divine: { target_id: targetPlayerId } };
  //           default:
  //             throw new Error("夜の行動を実行できない役職です");
  //         }
  //       })();

  //       const request: NightActionRequest = {
  //         player_id: userId ?? "",
  //         action: action,
  //       };

  //       const response = await fetch(`http://localhost:8080/api/game/${roomId}/actions/night-action`, {
  //         method: "POST",
  //         headers: {
  //           "Content-Type": "application/json",
  //         },
  //         body: JSON.stringify(request),
  //       });

  //       if (!response.ok) {
  //         throw new Error("夜の行動の送信に失敗しました");
  //       }

  //       // PrivateGameInfoのhasActedフラグを更新
  //       if (userId) {
  //         updateHasActed(roomId, userId, true);
  //         console.log("PrivateGameInfo: hasActed updated to true after night action");
  //       }

  //       addMessage({
  //         id: Date.now().toString(),
  //         sender: "システム",
  //         message: "夜の行動を実行しました",
  //         timestamp: new Date().toISOString(),
  //         type: "system",
  //       });

  //       return true;
  //     } catch (error) {
  //       console.error("夜の行動エラー:", error);
  //       return false;
  //     }
  //   };

  //   const handleVote = async (targetId: string) => {
  //     try {
  //       const response = await fetch(`http://localhost:8080/api/game/${roomId}/actions/vote`, {
  //         method: "POST",
  //         headers: {
  //           "Content-Type": "application/json",
  //         },
  //         body: JSON.stringify({
  //           voter_id: userId,
  //           target_id: targetId,
  //         }),
  //       });

  //       if (!response.ok) {
  //         throw new Error("投票の送信に失敗しました");
  //       }

  //       // PrivateGameInfoのhasActedフラグを更新
  //       if (userId) {
  //         updateHasActed(roomId, userId, true);
  //         console.log("PrivateGameInfo: hasActed updated to true after voting");
  //       }

  //       addMessage({
  //         id: Date.now().toString(),
  //         sender: "システム",
  //         message: "投票を実行しました",
  //         timestamp: new Date().toISOString(),
  //         type: "system",
  //       });

  //       return true;
  //     } catch (error) {
  //       console.error("投票エラー:", error);
  //       return false;
  //     }
  //   };

  const handleChangeRole = async (playerId: string, newRole: string) => {
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/debug/change-role`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          player_id: playerId,
          new_role: newRole,
        }),
      });

      if (!response.ok) {
        throw new Error("役職の変更に失敗しました");
      }

      // 自分自身の役職が変更された場合は、privateGameInfoも更新する
      if (playerId === userId) {
        // 文字列の役職名をPrivateGameInfoの型に変換
        const roleType = (() => {
          switch (newRole) {
            case "Seer":
              return "Seer";
            case "Werewolf":
              return "Werewolf";
            default:
              return "Villager";
          }
        })();

        // privateGameInfoを更新
        updatePrivateGameInfo(roomId, playerId, { playerRole: roleType });
        console.log(`Self role changed to ${newRole}, privateGameInfo updated`);
      }

      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: `${gameInfo?.players.find(p => p.id === playerId)?.name || "Unknown"}の役職が${newRole}に変更されました`,
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("役職変更エラー:", error);
      return false;
    }
  };

  const nextPhase = async () => {
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/phase/next`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("フェーズの進行に失敗しました");
      }

      // フェーズが進行したらhasActedフラグをリセット
      if (userId) {
        updateHasActed(roomId, userId, false);
        console.log("PrivateGameInfo: hasActed reset to false after phase change");
      }

      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "フェーズが進行しました",
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("フェーズ進行エラー:", error);
      return false;
    }
  };

  const resetGame = async () => {
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/debug/reset`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("ゲームのリセットに失敗しました");
      }

      // PrivateGameInfoをセッションストレージから削除
      if (userId) {
        clearPrivateGameInfo(roomId, userId);
        console.log("PrivateGameInfo cleared from session storage");
      }

      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "Game has been reset",
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("ゲームリセットエラー:", error);
      return false;
    }
  };

  const resetBatchRequest = async () => {
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/debug/reset-batch`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("バッチリクエストのリセットに失敗しました");
      }

      const result = await response.json();

      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: `バッチリクエストがリセットされました (新しいバッチID: ${result.batch_id})`,
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("バッチリクエストリセットエラー:", error);
      return false;
    }
  };

  return {
    isStarting,
    startGame,
    // handleNightAction,
    // handleVote,
    handleChangeRole,
    nextPhase,
    resetGame,
    resetBatchRequest,
  };
};
