import { useCallback } from "react";
import { Player } from "../app/types";
import * as GameInput from "~~/services/gameInputGenerator";
import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { getPrivateGameInfo } from "~~/utils/privateGameInfoUtils";

const pendingDivinationTargetKey = (roomId: string, dayCount: number): string =>
  `pending_divination_target_${roomId}_${dayCount}`;

export const useBackgroundNightAction = () => {
  const handleBackgroundNightAction = useCallback(
    async (roomId: string, myId: string, players: Player[], username: string, gameInfo: GameInfo) => {
      try {
        const dayCount = gameInfo.day_count ?? 0;
        const privateGameInfo = getPrivateGameInfo(roomId, myId);
        const isSeer = privateGameInfo?.playerRole === "Seer";

        const pendingTargetId = localStorage.getItem(pendingDivinationTargetKey(roomId, dayCount));

        // ダミーターゲットとして最初の生存プレイヤーを選択
        const dummyTargetId = players.find(p => !p.is_dead && p.id !== myId)?.id || players[0].id;
        const hasSeerSelection = isSeer && typeof pendingTargetId === "string" && pendingTargetId.length > 0;
        const selectedTargetId = hasSeerSelection ? (pendingTargetId as string) : dummyTargetId;
        const isDummy = !hasSeerSelection;

        if (hasSeerSelection) {
          const targetPlayerName = players.find(p => p.id === selectedTargetId)?.name || "Unknown";
          localStorage.setItem(`divination_target_${roomId}`, selectedTargetId);
          localStorage.setItem(`divination_target_name_${roomId}`, targetPlayerName);
          localStorage.removeItem(pendingDivinationTargetKey(roomId, dayCount));
          console.log(
            `Submitting synchronized divination for Seer ${myId}, day=${dayCount}, target=${selectedTargetId}`,
          );
        } else {
          localStorage.removeItem(`divination_target_${roomId}`);
          localStorage.removeItem(`divination_target_name_${roomId}`);
          console.log(`Submitting dummy divination for player ${myId}, day=${dayCount}`);
        }

        const divinationData = await GameInput.generateDivinationInput(
          roomId,
          username,
          gameInfo,
          selectedTargetId,
          isDummy,
        );

        console.log("Divination data for synchronized submission:", divinationData);

        if (!divinationData) {
          throw new Error("Failed to generate divination data");
        }

        console.log("Encrypting divination data...");
        // 暗号化
        const encryptedDivination = await MPCEncryption.encryptDivination(divinationData);

        const alivePlayerCount = players.filter(player => !player.is_dead).length;

        console.log(
          `Sending synchronized divination request to server (alive players: ${alivePlayerCount}, is_dummy: ${isDummy})`,
        );
        // バックエンドにリクエスト送信
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/proof`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({
              proof_type: "Divination",
              data: {
                user_id: String(divinationData.privateInput.id),
                prover_count: alivePlayerCount,
                encrypted_data: encryptedDivination,
                is_dummy: isDummy,
              },
            }),
          },
        );

        if (!response.ok) {
          const errorText = await response.text();
          console.error("Server response:", errorText);
          throw new Error(`Failed to send night action: ${response.status} ${errorText}`);
        }

        console.log("Synchronized divination request sent successfully");
      } catch (error) {
        console.error("Background night action error:", error);
        throw error; // エラーを再スロー
      }
    },
    [],
  );

  return { handleBackgroundNightAction };
};
