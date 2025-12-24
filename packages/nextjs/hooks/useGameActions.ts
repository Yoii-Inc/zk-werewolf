import { useState } from "react";
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
        throw new Error("Failed to start game");
      }

      // Initialize PrivateGameInfo with null role (undetermined) when game starts
      if (userId) {
        try {
          initializePrivateGameInfo(roomId, userId);
          console.log("PrivateGameInfo initialized with null role in session storage");

          addMessage({
            id: Date.now().toString(),
            sender: "System",
            message: "Game has started. Please wait for role assignment...",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (storageError) {
          console.error("PrivateGameInfo initialization error:", storageError);
        }
      }

      return true;
    } catch (error) {
      console.error("Game start error:", error);
      return false;
    } finally {
      setIsStarting(false);
    }
  };

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
        throw new Error("Failed to change role");
      }

      // If own role is changed, update privateGameInfo as well
      if (playerId === userId) {
        // Convert string role name to PrivateGameInfo type
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
        sender: "System",
        message: `${gameInfo?.players.find(p => p.id === playerId)?.name || "Unknown"}'s role changed to ${newRole}`,
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("Role change error:", error);
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
        sender: "System",
        message: "Phase advanced successfully",
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("Phase progress error:", error);
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
        sender: "System",
        message: "Game has been reset successfully",
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("Game reset error:", error);
      return false;
    }
  };

  const resetBatchRequest = async () => {
    try {
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/debug/reset-batch`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("Failed to reset batch request");
      }

      const result = await response.json();

      addMessage({
        id: Date.now().toString(),
        sender: "System",
        message: `Batch request has been reset (new batch ID: ${result.batch_id})`,
        timestamp: new Date().toISOString(),
        type: "system",
      });

      return true;
    } catch (error) {
      console.error("Batch request reset error:", error);
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
