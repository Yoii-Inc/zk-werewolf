import { useEffect, useRef, useState } from "react";
import type { ChatMessage, WebSocketMessage } from "~~/types/game";

export const useGameWebSocket = (roomId: string, setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>) => {
  const websocketRef = useRef<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected");
  const hasConnectedRef = useRef(false);

  const connectWebSocket = () => {
    setWebsocketStatus("connecting");
    const ws = new WebSocket(`ws://localhost:8080/api/room/${roomId}/ws`);

    ws.onopen = () => {
      console.log("WebSocket接続が確立されました");
      setWebsocketStatus("connected");
    };

    ws.onmessage = event => {
      console.log("メッセージを受信しました:", event.data);
      const fullMessage: WebSocketMessage = JSON.parse(event.data);

      setMessages(prevMessages => [
        ...prevMessages,
        {
          id: "Server",
          sender: fullMessage.player_name,
          message: fullMessage.content,
          timestamp: new Date().toISOString(),
          type: "normal",
        },
      ]);
    };

    ws.onclose = event => {
      console.log("WebSocket接続が閉じられました", event);
      setWebsocketStatus("disconnected");
      websocketRef.current = null;
    };

    ws.onerror = error => {
      console.error("WebSocketエラーが発生しました:", error);
      setWebsocketStatus("error");
      websocketRef.current = null;
    };

    websocketRef.current = ws;
  };

  const disconnectWebSocket = () => {
    if (websocketRef.current && websocketRef.current.readyState !== WebSocket.CLOSED) {
      websocketRef.current.close();
    }
  };

  const sendMessage = (message: string) => {
    if (websocketRef.current && websocketRef.current.readyState === WebSocket.OPEN && message.trim() !== "") {
      const websocketMessage: WebSocketMessage = {
        message_type: "normal",
        player_id: Date.now().toString(),
        player_name: "プレイヤー",
        content: message,
        timestamp: new Date().toISOString(),
        room_id: roomId,
      };
      websocketRef.current.send(JSON.stringify(websocketMessage));
      return true;
    }
    return false;
  };

  useEffect(() => {
    if (!hasConnectedRef.current) {
      hasConnectedRef.current = true;
      connectWebSocket();
    }
    return () => {
      // disconnectWebSocket();
    };
  }, [roomId]);

  return {
    websocketRef,
    websocketStatus,
    connectWebSocket,
    disconnectWebSocket,
    sendMessage,
  };
};
