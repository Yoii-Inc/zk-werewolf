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
      const data = JSON.parse(event.data);

      // フェーズ変更通知の場合
      if (data.message_type === "phase_change") {
        console.log(`フェーズ変更通知を受信: ${data.from_phase} → ${data.to_phase}`);

        // Night → Discussion の場合、ダミーリクエストが必要な通知を発行
        if (data.requires_dummy_request) {
          // カスタムイベントを発行してuseGamePhaseフックに通知
          window.dispatchEvent(
            new CustomEvent("phaseChangeNotification", {
              detail: {
                fromPhase: data.from_phase,
                toPhase: data.to_phase,
                requiresDummyRequest: true,
              },
            }),
          );
        }
        return;
      }

      // 通常のチャットメッセージの場合
      const fullMessage: WebSocketMessage = data;

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
