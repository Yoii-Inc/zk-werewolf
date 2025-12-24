import { useCallback } from "react";
import { Player } from "../app/types";
import * as GameInput from "~~/services/gameInputGenerator";
import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";

export const useBackgroundNightAction = () => {
  const handleBackgroundNightAction = useCallback(
    async (roomId: string, myId: string, players: Player[], username: string, gameInfo: GameInfo) => {
      try {
        // ダミーターゲットとして最初の生存プレイヤーを選択
        const dummyTargetId = players.find(p => !p.is_dead && p.id !== myId)?.id || players[0].id;

        console.log(`Generating dummy divination for player ${myId}, target: ${dummyTargetId}`);
        const divinationData = await GameInput.generateDivinationInput(roomId, username, gameInfo, dummyTargetId, true);

        if (!divinationData) {
          throw new Error("Failed to generate divination data");
        }

        console.log("Encrypting divination data...");
        // 暗号化
        const encryptedDivination = await MPCEncryption.encryptDivination(divinationData);

        const alivePlayerCount = players.filter(player => !player.is_dead).length;

        console.log(`Sending dummy divination request to server (alive players: ${alivePlayerCount})`);
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
              },
            }),
          },
        );

        if (!response.ok) {
          const errorText = await response.text();
          console.error("Server response:", errorText);
          throw new Error(`Failed to send night action: ${response.status} ${errorText}`);
        }

        console.log("Dummy divination request sent successfully");
      } catch (error) {
        console.error("Background night action error:", error);
        throw error; // エラーを再スロー
      }
    },
    [],
  );

  return { handleBackgroundNightAction };
};
