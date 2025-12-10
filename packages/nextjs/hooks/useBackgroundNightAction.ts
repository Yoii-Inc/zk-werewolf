import { useCallback } from "react";
import { Player } from "../app/types";
import { useGameInputGenerator } from "./useGameInputGenerator";
import { GameInputGenerator } from "~~/services/gameInputGenerator";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";

export const useBackgroundNightAction = () => {
  const handleBackgroundNightAction = useCallback(
    async (roomId: string, myId: string, players: Player[], inputGenerator: GameInputGenerator) => {
      try {
        // inputGeneratorを使用してダミー占いデータを生成
        // ダミーターゲットとして最初の生存プレイヤーを選択
        const dummyTargetId = players.find(p => !p.is_dead && p.id !== myId)?.id || players[0].id;

        const { input: divinationData } = await inputGenerator.getDivinationInput(dummyTargetId);

        if (!divinationData) {
          throw new Error("Failed to generate divination data");
        }

        // 暗号化
        const encryptedDivination = await MPCEncryption.encryptDivination(divinationData);

        const alivePlayerCount = players.filter(player => !player.is_dead).length;

        // バックエンドにリクエスト送信
        const response = await fetch(`http://localhost:8080/api/game/${roomId}/proof`, {
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
        });

        if (!response.ok) {
          throw new Error("Failed to send night action");
        }
      } catch (error) {
        console.error("Background night action error:", error);
      }
    },
    [],
  );

  return { handleBackgroundNightAction };
};
