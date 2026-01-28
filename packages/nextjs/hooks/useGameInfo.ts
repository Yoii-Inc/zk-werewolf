import { useEffect, useRef, useState } from "react";
import type { ChatMessage, GameInfo, PrivateGameInfo, RoomInfo } from "~~/types/game";
import { getPrivateGameInfo, setPrivateGameInfo } from "~~/utils/privateGameInfoUtils";

export const useGameInfo = (
  roomId: string,
  userId: string | undefined,
  setMessages: (messages: ChatMessage[]) => void,
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
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/room/${roomId}`,
        );
        if (!response.ok) {
          throw new Error("Failed to fetch room info");
        }
        const data = await response.json();
        setRoomInfo(data);
      } catch (error) {
        console.error("Room info get error:", error);
      } finally {
        setIsLoading(false);
      }
    };

    const fetchGameInfo = async () => {
      try {
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/state`,
        );
        if (!response.ok) {
          throw new Error("Failed to fetch game info");
        }
        const data = await response.json();
        console.log(data);
        setGameInfo(data);

        // ゲームの状態変化を検出（初回時または待機中→進行中への変化）
        const currentStatus = data.phase;
        const prevStatus = prevGameStatusRef.current;
        prevGameStatusRef.current = currentStatus;

        // ゲームが新たに開始された場合（Waitingから他のフェーズに変わった、またはprevStatusがnullで現在のステータスがWaiting以外）
        const isNewlyStarted =
          (prevStatus === "Waiting" && currentStatus !== "Waiting") ||
          (prevStatus === null && currentStatus !== "Waiting");

        // ゲームが新たに開始された場合
        if (isNewlyStarted && userId) {
          console.log("Game newly started, initializing privateGameInfo for all players");

          // 自分のプレイヤー情報を特定
          const currentPlayer = data.players.find((player: any) => player.id === userId);

          if (currentPlayer) {
            // 既存のPrivateGameInfoを確認
            const existingPrivateInfo = getPrivateGameInfo(roomId, userId);

            // すでにRoleが割り当てられている場合は上書きしない
            if (existingPrivateInfo && existingPrivateInfo.playerRole !== null) {
              console.log(
                "PrivateGameInfo already exists with role assigned, skipping initialization:",
                existingPrivateInfo,
              );
              setPrivateGameInfoState(existingPrivateInfo);
            } else {
              // PrivateGameInfoを初期化（roleはMPC計算結果から後で設定）
              const newPrivateInfo: PrivateGameInfo = {
                playerId: userId,
                playerRole: null as any, // Roleはまだ未決定
                hasActed: false,
              };

              // セッションストレージに保存
              saveToStorage(roomId, newPrivateInfo);
              console.log("PrivateGameInfo initialized for player (role not yet assigned):", newPrivateInfo);

              // ステート更新
              setPrivateGameInfoState(newPrivateInfo);
            }
          }
        }
        // 通常の更新処理
        else if (userId && roomId) {
          const updatedPrivateInfo = getPrivateGameInfo(roomId, userId);
          if (updatedPrivateInfo) {
            setPrivateGameInfoState(updatedPrivateInfo);
            // console.log("PrivateGameInfo updated from session storage:", updatedPrivateInfo);
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
              source: "server" as const, // サーバー側メッセージ
            }),
          );
          setMessages(messages); // マージはuseGameChat内で行われる
        }
      } catch (error) {
        console.error("Game info get error:", error);
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
