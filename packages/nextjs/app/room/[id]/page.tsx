"use client";

import React, { useEffect, useRef, useState } from "react";
import type { ChatMessage, Player } from "../../types";
import { Clock, Moon, Send, StickyNote, Sun, UserCheck, UserX, Users } from "lucide-react";

interface RoomInfo {
  room_id: string;
  name: string;
  status: "Open" | "Inprogress" | "Closed"; // statusを追加
  phase: "day" | "night";
  max_players: number;
  currentPlayers: number;
  remainingTime: number;
  players: Player[];
}

const mockMessages: ChatMessage[] = [
  {
    id: "1",
    sender: "システム",
    message: "これはデフォルトメッセージです。",
    timestamp: new Date().toISOString(),
    type: "system",
  },
];

const mockPlayers: Player[] = [
  { id: "1", name: "プレイヤー1", status: "alive", isReady: true },
  { id: "2", name: "プレイヤー2", status: "alive", isReady: true },
  { id: "3", name: "プレイヤー3", status: "dead", isReady: true },
  { id: "4", name: "プレイヤー4", status: "alive", isReady: false },
  { id: "5", name: "プレイヤー5", status: "alive", isReady: true },
];

export default function RoomPage({ params }: { params: { id: string } }) {
  const [messages, setMessages] = useState<ChatMessage[]>(() => {
    // ページ読み込み時にローカルストレージからメッセージを復元
    if (typeof window !== "undefined") {
      const savedMessages = localStorage.getItem(`chat_messages_${params.id}`);
      return savedMessages ? JSON.parse(savedMessages) : mockMessages;
    }
    return mockMessages;
  });

  // メッセージが更新されたときにローカルストレージに保存
  useEffect(() => {
    if (messages.length > 0) {
      localStorage.setItem(`chat_messages_${params.id}`, JSON.stringify(messages));
    }
  }, [messages, params.id]);

  const [newMessage, setNewMessage] = useState("");
  const [notes, setNotes] = useState("");
  const [roomInfo, setRoomInfo] = useState<RoomInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isStarting, setIsStarting] = useState(false);

  const websocketRef = useRef<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected"); // WebSocket接続状態
  const hasConnectedRef = useRef(false);

  const connectWebSocket = () => {
    setWebsocketStatus("connecting");
    const ws = new WebSocket("ws://localhost:8080/api/room/ws");

    ws.onopen = () => {
      console.log("WebSocket接続が確立されました");
      setWebsocketStatus("connected");
    };

    ws.onmessage = event => {
      console.log("メッセージを受信しました:", event.data);
      // サーバーからの応答を処理する
      setMessages(prevMessages => [
        ...prevMessages,
        { id: "Server", sender: "Server", message: event.data, timestamp: new Date().toISOString(), type: "normal" },
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

  useEffect(() => {
    if (!hasConnectedRef.current) {
      hasConnectedRef.current = true;
      connectWebSocket();
    }
    // クリーンアップ関数
    return () => {
      // disconnectWebSocket();
    };
  }, [params.id]);

  useEffect(() => {
    const fetchRoomInfo = async () => {
      try {
        const response = await fetch(`http://localhost:8080/api/room/${params.id}`);
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

    fetchRoomInfo();
    // 定期的にルーム情報を更新
    const interval = setInterval(fetchRoomInfo, 5000);

    return () => {
      clearInterval(interval);
    };
  }, [params.id]);

  const sendMessage = () => {
    console.log(websocketRef.current);
    if (websocketRef.current && websocketRef.current.readyState === WebSocket.OPEN && newMessage.trim() !== "") {
      websocketRef.current.send(newMessage);
      setNewMessage(""); // 送信後にinputをクリア
    } else {
      if (!websocketRef.current || websocketRef.current.readyState !== WebSocket.OPEN) {
        console.error("WebSocket接続が確立されていません。");
      }
      if (newMessage.trim() === "") {
        console.error("メッセージが空です。");
      }
    }
  };

  const startGame = async () => {
    if (!roomInfo) return;
    setIsStarting(true);
    try {
      const response = await fetch(`http://localhost:8080/api/game/${params.id}/start`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("ゲームの開始に失敗しました");
      }
      // ゲーム開始成功時の処理
      const message: ChatMessage = {
        id: Date.now().toString(),
        sender: "システム",
        message: "ゲームが開始されました",
        timestamp: new Date().toISOString(),
        type: "system",
      };
      setMessages(prev => [...prev, message]);
    } catch (error) {
      console.error("ゲーム開始エラー:", error);
    } finally {
      setIsStarting(false);
    }
  };

  return (
    <div className="h-screen flex bg-gradient-to-br from-indigo-50 to-purple-50">
      {isLoading ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-xl text-indigo-600">ルーム情報を読み込み中...</div>
        </div>
      ) : roomInfo ? (
        <div className="flex-1 flex flex-col">
          {/* Game Info */}
          <div className="bg-white/80 backdrop-blur-sm border-b border-indigo-100 p-4 shadow-sm">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <h1 className="text-2xl font-bold text-indigo-900">{roomInfo.name}</h1>
                {roomInfo.status === "Open" && (
                  <span className="flex items-center gap-2 text-green-600 bg-green-50 px-3 py-1 rounded-full text-sm border">
                    部屋の状態：オープン(参加者待ち)
                  </span>
                )}
                {roomInfo.phase === "night" ? (
                  <span className="flex items-center gap-2 text-indigo-600 bg-indigo-50 px-3 py-1 rounded-full">
                    <Moon size={16} />
                    夜フェーズ
                  </span>
                ) : (
                  <span className="flex items-center gap-2 text-amber-600 bg-amber-50 px-3 py-1 rounded-full">
                    <Sun size={16} />
                    昼フェーズ
                  </span>
                )}
              </div>
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2 text-indigo-700">
                  <Users size={18} />
                  <span>
                    {roomInfo.players.length}/{roomInfo.max_players}人
                  </span>
                </div>
                <div className="flex items-center gap-2 text-indigo-700">
                  <Clock size={18} />
                  <span>
                    残り時間: {Math.floor(roomInfo.remainingTime / 60)}:
                    {String(roomInfo.remainingTime % 60).padStart(2, "0")}
                  </span>
                </div>
                {roomInfo.status === "Open" && (
                  <button
                    onClick={startGame}
                    disabled={isStarting || roomInfo.players.length < 2}
                    className={`px-4 py-2 rounded-lg text-white font-medium transition-colors ${
                      isStarting || roomInfo.players.length < 2
                        ? "bg-gray-400 cursor-not-allowed"
                        : "bg-green-600 hover:bg-green-700"
                    }`}
                  >
                    {isStarting ? "開始中..." : "ゲーム開始"}
                  </button>
                )}
              </div>
            </div>
          </div>

          <div className="flex-1 flex">
            {/* Players List */}
            <div className="w-64 bg-white/80 backdrop-blur-sm border-l border-indigo-100">
              <div className="p-4 border-b border-indigo-100">
                <h2 className="text-lg font-semibold text-indigo-900">参加者一覧</h2>
              </div>
              <div className="p-4 space-y-3">
                {roomInfo.players.map(player => (
                  <div
                    key={player.id}
                    className={`flex items-center justify-between p-2 rounded-lg ${
                      player.status === "dead" ? "bg-gray-100 text-gray-500" : "bg-white text-indigo-900"
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      {player.status === "alive" ? (
                        <UserCheck size={18} className="text-green-500" />
                      ) : (
                        <UserX size={18} className="text-red-500" />
                      )}
                      <span className={player.status === "dead" ? "line-through" : ""}>{player.name}</span>
                    </div>
                    {!player.isReady && (
                      <span className="text-xs bg-yellow-100 text-yellow-700 px-2 py-1 rounded">準備中</span>
                    )}
                  </div>
                ))}
              </div>
              <div>
                {/* webscoket関連(デバッグ用)とかく */}
                <div className="p-4 border-b border-indigo-100">
                  <h2 className="text-lg font-semibold text-indigo-900">Websocket関連(デバッグ用)</h2>
                  <div className="text-indigo-700">WebSocket URL: ws://localhost:8080/api/room/ws</div>
                  <div className="text-indigo-700">WebSocket ReadyState: {websocketRef.current?.readyState}</div>
                  <div className="text-indigo-700">WebSocket Status: {websocketStatus}</div>
                </div>

                <button
                  onClick={connectWebSocket}
                  disabled={websocketStatus === "connected" || websocketStatus === "connecting"}
                  className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                >
                  {websocketStatus === "connecting" ? "接続中..." : "WebSocket接続"}
                </button>
                <button
                  onClick={disconnectWebSocket}
                  disabled={websocketStatus === "disconnected"}
                  className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                >
                  WebSocket切断
                </button>
              </div>
            </div>

            {/* Chat Area */}
            <div className="flex-1 flex flex-col">
              <div className="flex-1 overflow-y-auto p-4">
                {messages.map(msg => (
                  <div
                    key={msg.id}
                    className={`mb-4 rounded-lg p-3 ${
                      msg.type === "system"
                        ? "bg-indigo-50 text-indigo-700 text-center"
                        : msg.type === "whisper"
                          ? "bg-purple-50 text-purple-700 italic"
                          : "bg-white"
                    }`}
                  >
                    <span className="font-semibold">{msg.sender}: </span>
                    <span>{msg.message}</span>
                  </div>
                ))}
              </div>

              {/* Message Input */}
              <div className="p-4">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newMessage}
                    onChange={e => setNewMessage(e.target.value)}
                    placeholder="メッセージを入力..."
                    className="flex-1 border border-indigo-200 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 bg-white/80 backdrop-blur-sm"
                  />
                  <button
                    onClick={sendMessage}
                    className="bg-indigo-600 text-white px-6 py-2 rounded-lg flex items-center gap-2 hover:bg-indigo-700 transition-colors shadow-sm"
                  >
                    <Send size={20} />
                    送信
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-xl text-red-600">ルーム情報の取得に失敗しました。</div>
        </div>
      )}

      {/* Notes Panel */}
      <div className="w-80 bg-white/80 backdrop-blur-sm border-l border-indigo-100 flex flex-col">
        <div className="p-4 border-b border-indigo-100 flex items-center gap-2">
          <StickyNote size={20} className="text-indigo-600" />
          <h2 className="text-lg font-semibold text-indigo-900">メモ</h2>
        </div>
        <textarea
          value={notes}
          onChange={e => setNotes(e.target.value)}
          placeholder="メモを入力..."
          className="flex-1 p-4 resize-none focus:outline-none bg-transparent"
        />
      </div>
    </div>
  );
}
