"use client";

import React, { useEffect, useState } from "react";
import type { ChatMessage, Player } from "../../types";
import { Clock, Moon, Send, StickyNote, Sun, UserCheck, UserX, Users } from "lucide-react";

const mockMessages: ChatMessage[] = [
  {
    id: "1",
    sender: "システム",
    message: "ゲームが開始されました。",
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
  const [messages, setMessages] = useState<ChatMessage[]>(mockMessages);
  const [newMessage, setNewMessage] = useState("");
  const [notes, setNotes] = useState("");
  const [isNight] = useState(true);

  const [websocket, setWebsocket] = useState<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected"); // WebSocket接続状態

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
      setWebsocket(null);
    };

    ws.onerror = error => {
      console.error("WebSocketエラーが発生しました:", error);
      setWebsocketStatus("error");
      setWebsocket(null);
    };

    setWebsocket(ws);
  };

  const disconnectWebSocket = () => {
    if (websocket && websocket.readyState !== WebSocket.CLOSED) {
      websocket.close();
    }
  };

  useEffect(() => {
    // クリーンアップ関数
    return () => {
      disconnectWebSocket();
    };
  }, []);

  const sendMessage = () => {
    if (websocket && newMessage) {
      websocket.send(newMessage);
      setNewMessage(""); // 送信後にinputをクリア
    } else {
      if (!websocket) {
        console.error("WebSocket接続が確立されていません。");
      }
      if (!newMessage) {
        console.error("メッセージが空です。");
      }
    }
  };

  return (
    <div className="h-screen flex bg-gradient-to-br from-indigo-50 to-purple-50">
      {/* Main Game Area */}
      <div className="flex-1 flex flex-col">
        {/* Game Info */}
        <div className="bg-white/80 backdrop-blur-sm border-b border-indigo-100 p-4 shadow-sm">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <h1 className="text-2xl font-bold text-indigo-900">初心者歓迎！</h1>
              {isNight ? (
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
                <span>5/8人</span>
              </div>
              <div className="flex items-center gap-2 text-indigo-700">
                <Clock size={18} />
                <span>残り時間: 5:00</span>
              </div>
            </div>
          </div>
        </div>

        <div className="flex-1 flex">
          {/* Players List */}
          <div className="w-64 bg-white/80 backdrop-blur-sm border-l border-indigo-100">
            <div className="p-4 border-b border-indigo-100">
              <h2 className="text-lg font-semibold text-indigo-900">参加者一覧</h2>
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
            <div className="p-4 space-y-3">
              {mockPlayers.map(player => (
                <div
                  key={player.id}
                  className={`flex items-center justify-between p-2 rounded-lg ${
                    player.status === "dead" ? "bg-gray-100 text-gray-500" : "bg-white text-indigo-900"
                  }`}
                >
                  <div className="flex items中心 gap-2">
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
