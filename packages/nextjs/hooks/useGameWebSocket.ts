import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ChatMessage, WebSocketMessage } from "~~/types/game";

type WebSocketStatus = "disconnected" | "connecting" | "connected" | "reconnecting" | "error";

interface UseGameWebSocketOptions {
  onReconnect?: () => void | Promise<void>;
  onGapDetected?: (detail: { expectedEventId: number; receivedEventId: number }) => void | Promise<void>;
  playerId?: string;
}

interface RoomEventEnvelope {
  event_id: number;
  room_id: string;
  timestamp: string;
  payload: unknown;
}

const getLastEventIdStorageKey = (roomId: string) => `ws_last_event_id_${roomId}`;

const loadLastEventId = (roomId: string): number => {
  if (typeof window === "undefined" || !roomId) return 0;
  try {
    const raw = sessionStorage.getItem(getLastEventIdStorageKey(roomId));
    if (!raw) return 0;
    const parsed = Number(raw);
    return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : 0;
  } catch {
    return 0;
  }
};

const persistLastEventId = (roomId: string, eventId: number) => {
  if (typeof window === "undefined" || !roomId) return;
  try {
    sessionStorage.setItem(getLastEventIdStorageKey(roomId), String(eventId));
  } catch {
    // ignore storage errors
  }
};

const isRoomEventEnvelope = (value: unknown): value is RoomEventEnvelope => {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Partial<RoomEventEnvelope>;
  return (
    typeof candidate.event_id === "number" &&
    typeof candidate.room_id === "string" &&
    typeof candidate.timestamp === "string" &&
    "payload" in candidate
  );
};

export const useGameWebSocket = (
  roomId: string,
  addServerMessage: (message: ChatMessage) => void,
  username?: string,
  options?: UseGameWebSocketOptions,
) => {
  const { onReconnect, onGapDetected, playerId } = options ?? {};
  const websocketRef = useRef<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<WebSocketStatus>("disconnected");
  const [reconnectAttempt, setReconnectAttempt] = useState(0);

  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptRef = useRef(0);
  const manualDisconnectRef = useRef(false);
  const hasConnectedOnceRef = useRef(false);
  const onReconnectRef = useRef<UseGameWebSocketOptions["onReconnect"]>(onReconnect);
  const onGapDetectedRef = useRef<UseGameWebSocketOptions["onGapDetected"]>(onGapDetected);
  const connectWebSocketRef = useRef<() => void>(() => undefined);
  const lastEventIdRef = useRef(0);

  useEffect(() => {
    onReconnectRef.current = onReconnect;
  }, [onReconnect]);

  useEffect(() => {
    onGapDetectedRef.current = onGapDetected;
  }, [onGapDetected]);

  useEffect(() => {
    lastEventIdRef.current = loadLastEventId(roomId);
  }, [roomId]);

  const wsUrl = useMemo(
    () => `${process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/api"}/room/${roomId}/ws`,
    [roomId],
  );

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const scheduleReconnect = useCallback(() => {
    if (manualDisconnectRef.current || reconnectTimerRef.current) {
      return;
    }

    const nextAttempt = reconnectAttemptRef.current + 1;
    reconnectAttemptRef.current = nextAttempt;
    setReconnectAttempt(nextAttempt);
    setWebsocketStatus("reconnecting");

    const baseDelayMs = Math.min(30000, 1000 * 2 ** Math.min(nextAttempt - 1, 5));
    const jitterMs = Math.floor(Math.random() * 300);
    const delayMs = baseDelayMs + jitterMs;

    reconnectTimerRef.current = setTimeout(() => {
      reconnectTimerRef.current = null;
      connectWebSocketRef.current();
    }, delayMs);
  }, []);

  const handleSocketMessage = useCallback(
    (rawData: string) => {
      let parsed: unknown;
      try {
        parsed = JSON.parse(rawData);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error);
        return;
      }

      let messageData = parsed;
      let sourceEventId: number | undefined;
      let sourceEventTimestamp: string | undefined;

      if (isRoomEventEnvelope(parsed)) {
        const incomingEventId = parsed.event_id;
        let lastEventId = lastEventIdRef.current;

        // Server restart or room event store reset can roll event_id back.
        // In that case, treat incoming stream as a new sequence instead of dropping all events forever.
        if (incomingEventId < lastEventId) {
          console.warn(
            `Detected room event_id reset (last=${lastEventId}, incoming=${incomingEventId}). Resetting cursor.`,
          );
          lastEventIdRef.current = 0;
          persistLastEventId(roomId, 0);
          lastEventId = 0;
        } else if (incomingEventId === lastEventId) {
          return;
        }

        if (lastEventId > 0 && incomingEventId > lastEventId + 1) {
          const detail = {
            expectedEventId: lastEventId + 1,
            receivedEventId: incomingEventId,
          };

          window.dispatchEvent(new CustomEvent("wsEventGapDetected", { detail }));

          if (onGapDetectedRef.current) {
            void Promise.resolve(onGapDetectedRef.current(detail)).catch(error => {
              console.error("Failed to recover after WebSocket event gap:", error);
            });
          } else if (onReconnectRef.current) {
            void Promise.resolve(onReconnectRef.current()).catch(error => {
              console.error("Failed to recover after WebSocket event gap:", error);
            });
          }
        }

        lastEventIdRef.current = incomingEventId;
        persistLastEventId(roomId, incomingEventId);

        sourceEventId = incomingEventId;
        sourceEventTimestamp = parsed.timestamp;
        messageData = parsed.payload;
      }

      if (!messageData || typeof messageData !== "object") {
        return;
      }

      const data = messageData as Record<string, unknown>;
      const messageType = data.message_type;

      if (messageType === "phase_change") {
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

      if (messageType === "commitments_ready") {
        window.dispatchEvent(
          new CustomEvent("commitmentsReadyNotification", {
            detail: {
              roomId: data.room_id,
              commitmentsCount: data.commitments_count,
              totalPlayers: data.total_players,
              timestamp: data.timestamp,
            },
          }),
        );
        return;
      }

      if (messageType === "computation_result") {
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

      if (messageType === "proof_job_status") {
        window.dispatchEvent(
          new CustomEvent("proofJobStatusNotification", {
            detail: {
              roomId: data.room_id,
              batchId: data.batch_id,
              state: data.state,
              attemptCount: data.attempt_count,
              lastError: data.last_error,
              jobNodeStatus: data.job_node_status,
              updatedAt: data.updated_at,
            },
          }),
        );
        return;
      }

      if (messageType === "room_state_changed") {
        window.dispatchEvent(
          new CustomEvent("roomStateChangedNotification", {
            detail: {
              roomId: data.room_id,
              reason: data.reason,
              timestamp: data.timestamp,
            },
          }),
        );
        return;
      }

      if (messageType === "game_reset") {
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

      if (typeof data.player_name !== "string" || typeof data.content !== "string") {
        return;
      }

      const fullMessage = data as unknown as WebSocketMessage;
      addServerMessage({
        id:
          sourceEventId !== undefined
            ? `event-${sourceEventId}`
            : typeof fullMessage.player_id === "string"
              ? fullMessage.player_id
              : "Server",
        sender: fullMessage.player_name,
        message: fullMessage.content,
        timestamp:
          typeof fullMessage.timestamp === "string"
            ? fullMessage.timestamp
            : sourceEventTimestamp || new Date().toISOString(),
        type: "normal",
        source: "server",
      });
    },
    [addServerMessage, roomId],
  );

  const connectWebSocket = useCallback(() => {
    if (!roomId) return;

    const currentReadyState = websocketRef.current?.readyState;
    if (currentReadyState === WebSocket.OPEN || currentReadyState === WebSocket.CONNECTING) {
      return;
    }

    clearReconnectTimer();
    manualDisconnectRef.current = false;
    setWebsocketStatus(reconnectAttemptRef.current > 0 ? "reconnecting" : "connecting");

    const lastEventId = lastEventIdRef.current;
    const params = new URLSearchParams();
    if (lastEventId > 0) {
      params.set("last_event_id", String(lastEventId));
    }
    if (playerId) {
      params.set("player_id", playerId);
    }
    const query = params.toString();
    const resumeWsUrl = query.length > 0 ? `${wsUrl}?${query}` : wsUrl;

    const ws = new WebSocket(resumeWsUrl);
    websocketRef.current = ws;

    ws.onopen = () => {
      if (websocketRef.current !== ws) return;

      const wasReconnect = hasConnectedOnceRef.current;
      hasConnectedOnceRef.current = true;
      reconnectAttemptRef.current = 0;
      setReconnectAttempt(0);
      setWebsocketStatus("connected");

      if (wasReconnect && onReconnectRef.current) {
        void Promise.resolve(onReconnectRef.current()).catch(error => {
          console.error("Failed to resync after reconnect:", error);
        });
      }
    };

    ws.onmessage = event => {
      if (typeof event.data !== "string") return;
      handleSocketMessage(event.data);
    };

    ws.onclose = () => {
      if (websocketRef.current !== ws) return;
      websocketRef.current = null;

      if (manualDisconnectRef.current) {
        setWebsocketStatus("disconnected");
        return;
      }

      scheduleReconnect();
    };

    ws.onerror = error => {
      if (websocketRef.current !== ws) return;
      console.error("WebSocket error occurred:", error);
      setWebsocketStatus("error");
      ws.close();
    };
  }, [clearReconnectTimer, handleSocketMessage, playerId, roomId, scheduleReconnect, wsUrl]);
  connectWebSocketRef.current = connectWebSocket;

  const disconnectWebSocket = useCallback(() => {
    manualDisconnectRef.current = true;
    clearReconnectTimer();
    reconnectAttemptRef.current = 0;
    setReconnectAttempt(0);
    setWebsocketStatus("disconnected");

    if (websocketRef.current && websocketRef.current.readyState !== WebSocket.CLOSED) {
      websocketRef.current.close();
    }
    websocketRef.current = null;
  }, [clearReconnectTimer]);

  const sendMessage = useCallback(
    (message: string) => {
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
    },
    [roomId, username],
  );

  useEffect(() => {
    reconnectAttemptRef.current = 0;
    setReconnectAttempt(0);
    hasConnectedOnceRef.current = false;
    manualDisconnectRef.current = false;
    lastEventIdRef.current = loadLastEventId(roomId);
    connectWebSocket();

    return () => {
      disconnectWebSocket();
    };
  }, [connectWebSocket, disconnectWebSocket, roomId]);

  useEffect(() => {
    const tryReconnectNow = () => {
      if (manualDisconnectRef.current) return;
      const state = websocketRef.current?.readyState;
      if (state === WebSocket.OPEN || state === WebSocket.CONNECTING) return;
      clearReconnectTimer();
      connectWebSocketRef.current();
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        tryReconnectNow();
      }
    };

    window.addEventListener("online", tryReconnectNow);
    window.addEventListener("focus", tryReconnectNow);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.removeEventListener("online", tryReconnectNow);
      window.removeEventListener("focus", tryReconnectNow);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [clearReconnectTimer]);

  return {
    websocketRef,
    websocketStatus,
    reconnectAttempt,
    connectWebSocket,
    disconnectWebSocket,
    sendMessage,
  };
};
