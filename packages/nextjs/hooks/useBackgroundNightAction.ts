import { useCallback } from "react";
import { Player } from "../app/types";
import * as GameInput from "~~/services/gameInputGenerator";
import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { getPrivateGameInfo } from "~~/utils/privateGameInfoUtils";

const pendingDivinationTargetKey = (roomId: string, dayCount: number): string =>
  `pending_divination_target_${roomId}_${dayCount}`;

const divinationTargetIdByDayKey = (roomId: string, dayCount: number): string =>
  `divination_target_${roomId}_${dayCount}`;

const divinationTargetNameByDayKey = (roomId: string, dayCount: number): string =>
  `divination_target_name_${roomId}_${dayCount}`;

const latestDivinationTargetIdKey = (roomId: string): string => `divination_target_${roomId}`;

const latestDivinationTargetNameKey = (roomId: string): string => `divination_target_name_${roomId}`;

const hasFortuneTellerPublicKey = (info: GameInfo | null | undefined): boolean => {
  const key = info?.crypto_parameters?.fortune_teller_public_key as { x?: unknown[]; y?: unknown[] } | undefined;
  if (!key) return false;
  return Array.isArray(key.x) && key.x.length > 0 && Array.isArray(key.y) && key.y.length > 0;
};

const fetchLatestGameState = async (roomId: string): Promise<GameInfo | null> => {
  try {
    const response = await fetch(
      `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/state`,
    );
    if (!response.ok) return null;
    return (await response.json()) as GameInfo;
  } catch (error) {
    console.warn("Failed to fetch latest game state while waiting for divination public key:", error);
    return null;
  }
};

const waitForDivinationPublicKey = async (
  roomId: string,
  initialGameInfo: GameInfo,
  timeoutMs = 12000,
  pollIntervalMs = 300,
): Promise<GameInfo> => {
  if (hasFortuneTellerPublicKey(initialGameInfo)) {
    return initialGameInfo;
  }

  const deadline = Date.now() + timeoutMs;
  let latest = initialGameInfo;

  while (Date.now() < deadline) {
    const fetched = await fetchLatestGameState(roomId);
    if (fetched) {
      latest = fetched;
      if (hasFortuneTellerPublicKey(fetched)) {
        console.log("Divination public key is ready, proceeding with synchronized divination");
        return fetched;
      }
    }
    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
  }

  throw new Error("Fortune teller public key is not ready yet. Please retry synchronized divination shortly.");
};

export const useBackgroundNightAction = () => {
  const handleBackgroundNightAction = useCallback(
    async (roomId: string, myId: string, players: Player[], username: string, gameInfo: GameInfo) => {
      try {
        const gameInfoWithDivinationKey = await waitForDivinationPublicKey(roomId, gameInfo);
        const currentPlayers =
          Array.isArray(gameInfoWithDivinationKey.players) && gameInfoWithDivinationKey.players.length > 0
            ? gameInfoWithDivinationKey.players
            : players;
        const dayCount = gameInfoWithDivinationKey.day_count ?? 0;
        const privateGameInfo = getPrivateGameInfo(roomId, myId);
        const isSeer = privateGameInfo?.playerRole === "Seer";

        const pendingTargetId = localStorage.getItem(pendingDivinationTargetKey(roomId, dayCount));

        // ダミーターゲットとして最初の生存プレイヤーを選択
        const dummyTargetId = currentPlayers.find(p => !p.is_dead && p.id !== myId)?.id || currentPlayers[0].id;
        const hasSeerSelection = isSeer && typeof pendingTargetId === "string" && pendingTargetId.length > 0;
        const selectedTargetId = hasSeerSelection ? (pendingTargetId as string) : dummyTargetId;
        const isDummy = !hasSeerSelection;

        if (hasSeerSelection) {
          const targetPlayerName =
            currentPlayers.find(p => String(p.id) === String(selectedTargetId))?.name || "Unknown";

          localStorage.setItem(divinationTargetIdByDayKey(roomId, dayCount), selectedTargetId);
          localStorage.setItem(divinationTargetNameByDayKey(roomId, dayCount), targetPlayerName);

          // 最新キーは既存参照との後方互換のために維持
          localStorage.setItem(latestDivinationTargetIdKey(roomId), selectedTargetId);
          localStorage.setItem(latestDivinationTargetNameKey(roomId), targetPlayerName);

          localStorage.removeItem(pendingDivinationTargetKey(roomId, dayCount));
          console.log(
            `Submitting synchronized divination for Seer ${myId}, day=${dayCount}, target=${selectedTargetId}`,
          );
        } else {
          // ダミー送信時に最新キーを消すと、結果受信前に対象名が失われるケースがある。
          // day単位キー/最新キーは保持して結果表示で参照する。
          console.log(`Submitting dummy divination for player ${myId}, day=${dayCount}`);
        }

        const divinationData = await GameInput.generateDivinationInput(
          roomId,
          username,
          gameInfoWithDivinationKey,
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

        const alivePlayerCount = currentPlayers.filter(player => !player.is_dead).length;

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
