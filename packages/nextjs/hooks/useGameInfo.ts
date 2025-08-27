import { useEffect, useState } from "react";
import type { ChatMessage, GameInfo, RoomInfo } from "~~/types/game";

export const useGameInfo = (roomId: string, setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>) => {
  const [roomInfo, setRoomInfo] = useState<RoomInfo | null>(null);
  const [gameInfo, setGameInfo] = useState<GameInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);

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
  }, [roomInfo?.status, roomId, setMessages]);

  return { roomInfo, gameInfo, isLoading, setGameInfo };
};
