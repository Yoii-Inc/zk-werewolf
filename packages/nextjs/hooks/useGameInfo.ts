import { useEffect, useRef, useState } from "react";
import type { ChatMessage, GameInfo, PrivateGameInfo, RoomInfo } from "~~/types/game";
import { getPrivateGameInfo, setPrivateGameInfo } from "~~/utils/privateGameInfoUtils";

export const useGameInfo = (
  roomId: string,
  userId: string | undefined,
  setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>,
) => {
  const [roomInfo, setRoomInfo] = useState<RoomInfo | null>(null);
  const [gameInfo, setGameInfo] = useState<GameInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const prevGameStatusRef = useRef<string | null>(null);

  // フック内のステート更新関数と区別するため、明示的に変数に保存
  const saveToStorage = setPrivateGameInfo;

  const [privateGameInfo, setPrivateGameInfoState] = useState<PrivateGameInfo | null>(() => {
    // ページ読み込み時にセッションストレージからプライベート情報を復元
    if (typeof window !== "undefined" && userId && roomId) {
      return getPrivateGameInfo(roomId, userId);
    }
    return null;
  });

  useEffect(() => {
    const fetchRoomInfo = async () => {
      try {
        const response = await fetch(`http://localhost:8080/api/room/${roomId}`);
        if (!response.ok) {
          throw new Error("ルーム情報の取得に失敗しました");
        }
        const data = await response.json();
        setRoomInfo(data);
      } catch (error) {
        console.error("ルーム情報の取得エラー:", error);
      } finally {
        setIsLoading(false);
      }
    };

    const fetchGameInfo = async () => {
      try {
        const response = await fetch(`http://localhost:8080/api/game/${roomId}/state`);
        if (!response.ok) {
          throw new Error("ゲーム情報の取得に失敗しました");
        }
        const data = await response.json();
        console.log(data);
        setGameInfo(data);

        // ゲームの状態変化を検出（初回時または待機中→進行中への変化）
        const currentStatus = data.phase;
        const prevStatus = prevGameStatusRef.current;
        prevGameStatusRef.current = currentStatus;

        // ゲームがリセットされた場合（他のフェーズからWaitingに戻った）
        const isReset = prevStatus !== null && prevStatus !== "Waiting" && currentStatus === "Waiting";

        // ゲームが新たに開始された場合（Waitingから他のフェーズに変わった、またはprevStatusがnullで現在のステータスがWaiting以外）
        const isNewlyStarted =
          (prevStatus === "Waiting" && currentStatus !== "Waiting") ||
          (prevStatus === null && currentStatus !== "Waiting");

        if (isReset && userId) {
          console.log("Game reset detected, clearing privateGameInfo");
          // ステート更新
          setPrivateGameInfoState(null);
          sessionStorage.removeItem(`game_${roomId}_player_${userId}`);
        }
        // ゲームが新たに開始された場合
        else if (isNewlyStarted && userId) {
          console.log("Game newly started, initializing privateGameInfo for all players");

          // 自分のプレイヤー情報を特定
          const currentPlayer = data.players.find((player: any) => player.id === userId);

          if (currentPlayer) {
            // PrivateGameInfoを初期化
            const newPrivateInfo: PrivateGameInfo = {
              playerId: userId,
              playerRole: (() => {
                switch (currentPlayer.role) {
                  case "Seer":
                    return "占い師";
                  case "Werewolf":
                    return "人狼";
                  default:
                    return "村人";
                }
              })(),
              hasActed: false,
            };

            // セッションストレージに保存
            saveToStorage(roomId, newPrivateInfo);
            console.log("PrivateGameInfo initialized for non-starter player:", newPrivateInfo);

            // ステート更新
            setPrivateGameInfoState(newPrivateInfo);
          }
        }
        // 通常の更新処理
        else if (userId && roomId) {
          const updatedPrivateInfo = getPrivateGameInfo(roomId, userId);
          if (updatedPrivateInfo) {
            setPrivateGameInfoState(updatedPrivateInfo);
            console.log("PrivateGameInfo updated from session storage:", updatedPrivateInfo);
          }
        }

        if (data.chat_log?.messages) {
          const messages: ChatMessage[] = data.chat_log.messages.map(
            (msg: { id: any; player_name: any; content: any; timestamp: any; message_type: string }) => ({
              id: msg.id,
              sender: msg.player_name,
              message: msg.content,
              timestamp: msg.timestamp,
              type: msg.message_type === "System" ? "system" : "normal",
            }),
          );
          setMessages(prev => [...messages]);
        }
      } catch (error) {
        console.error("ゲーム情報の取得エラー:", error);
      } finally {
        setIsLoading(false);
      }
    };

    fetchRoomInfo();
    const roomInterval = setInterval(fetchRoomInfo, 5000);

    let gameInterval: NodeJS.Timeout | null = null;
    if (roomInfo?.status === "InProgress") {
      fetchGameInfo();
      gameInterval = setInterval(fetchGameInfo, 5000);
    }

    return () => {
      clearInterval(roomInterval);
      if (gameInterval) {
        clearInterval(gameInterval);
      }
    };
  }, [roomInfo?.status, roomId, userId, setMessages]);

  return { roomInfo, gameInfo, privateGameInfo, isLoading, setGameInfo };
};
