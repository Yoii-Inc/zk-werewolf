import { useEffect, useRef, useState } from "react";
import type { ChatMessage, WebSocketMessage } from "~~/types/game";

interface ComputationResult {
  computationType: string;
  resultData: any;
  targetPlayerId?: string;
  batchId: string;
  timestamp: string;
}

export const useGameWebSocket = (
  roomId: string,
  addServerMessage: (message: ChatMessage) => void, // サーバー側メッセージを追加する関数
  username?: string,
) => {
  const websocketRef = useRef<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected");
  const hasConnectedRef = useRef(false);

  const connectWebSocket = () => {
    setWebsocketStatus("connecting");
    const ws = new WebSocket(`${process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/api"}/room/${roomId}/ws`);

    ws.onopen = () => {
      console.log("WebSocket connection established");
      setWebsocketStatus("connected");
    };

    ws.onmessage = event => {
      console.log("Message received:", event.data);
      const data = JSON.parse(event.data);

      // フェーズ変更通知の場合
      if (data.message_type === "phase_change") {
        console.log(`Phase change notification received: ${data.from_phase} → ${data.to_phase}`);

        // カスタムイベントを発行してuseGamePhaseフックに通知
        window.dispatchEvent(
          new CustomEvent("phaseChangeNotification", {
            detail: {
              fromPhase: data.from_phase,
              toPhase: data.to_phase,
              requiresDummyRequest: data.requires_dummy_request,
            },
          }),
        );
        return;
      }

      // For computation result notification
      if (data.message_type === "computation_result") {
        console.log(`Computation result notification received: ${data.computation_type}`);

        // カスタムイベントを発行
        window.dispatchEvent(
          new CustomEvent("computationResultNotification", {
            detail: {
              computationType: data.computation_type,
              resultData: data.result_data,
              targetPlayerId: data.target_player_id,
              batchId: data.batch_id,
              timestamp: data.timestamp,
            },
          }),
        );
        return;
      }

      // ゲームリセット通知の場合
      if (data.message_type === "game_reset") {
        console.log("Game reset notification received");

        // カスタムイベントを発行
        window.dispatchEvent(
          new CustomEvent("gameResetNotification", {
            detail: {
              roomId: data.room_id,
              timestamp: data.timestamp,
            },
          }),
        );
        return;
      }

      // 通常のチャットメッセージの場合
      const fullMessage: WebSocketMessage = data;

      addServerMessage({
        id: "Server",
        sender: fullMessage.player_name,
        message: fullMessage.content,
        timestamp: new Date().toISOString(),
        type: "normal",
        source: "server", // WebSocketで受信したメッセージはサーバー側
      });
    };

    ws.onclose = event => {
      console.log("WebSocket connection closed", event);
      setWebsocketStatus("disconnected");
      websocketRef.current = null;
    };

    ws.onerror = error => {
      console.error("WebSocket error occurred:", error);
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
        player_name: username || "Player",
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
