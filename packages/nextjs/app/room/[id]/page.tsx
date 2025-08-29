"use client";

import React, { useEffect, useRef, useState } from "react";
import NightActionModal from "../../../components/game/NightActionModal";
import VoteModal from "../../../components/game/VoteModal";
import type { ChatMessage, Player, WebSocketMessage } from "../../types";
import { NightAction, NightActionRequest } from "../types";
import { Clock, Moon, Send, StickyNote, Sun, UserCheck, UserX, Users } from "lucide-react";
import { useAuth } from "~~/app/contexts/AuthContext";
import { TweetNaclKeyManager } from "~~/utils/crypto/tweetNaclKeyManager";

interface RoomInfo {
  room_id: string;
  name: string;
  status: "Open" | "InProgress" | "Closed"; // statusを追加
  max_players: number;
  currentPlayers: number;
  remainingTime: number;
  players: Player[];
}

interface GameInfo {
  room_id: string;
  phase: "Waiting" | "Night" | "Discussion" | "Voting" | "Result" | "Finished";
  players: Player[];
  playerRole: "占い師" | "人狼" | "村人"; // 自分の役職
  hasActed: boolean; // その夜にアクションを実行済みかどうか
  result: "InProgress" | "VillagerWin" | "WerewolfWin";
}

interface GameResultModalProps {
  result: "VillagerWin" | "WerewolfWin" | "InProgress";
  onClose: () => void;
}

const GameResultModal = ({ result, onClose }: GameResultModalProps) => {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-8 max-w-lg w-full mx-4 text-center">
        <h2 className="text-3xl font-bold mb-4 text-indigo-900">
          {result === "VillagerWin" ? "村人陣営の勝利！" : "人狼陣営の勝利！"}
        </h2>
        {/* <p className="text-xl mb-6 text-gray-700">{result}</p> */}
        <button
          onClick={onClose}
          className="px-6 py-3 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          閉じる
        </button>
      </div>
    </div>
  );
};

const mockMessages: ChatMessage[] = [
  {
    id: "1",
    sender: "システム",
    message: "これはデフォルトメッセージです。",
    timestamp: new Date().toISOString(),
    type: "system",
  },
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
  const [gameInfo, setGameInfo] = useState<GameInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isStarting, setIsStarting] = useState(false);
  const [showNightAction, setShowNightAction] = useState(false);
  const [showVoteModal, setShowVoteModal] = useState(false);
  const { isAuthenticated, user, logout } = useAuth();
  const [showGameResult, setShowGameResult] = useState(false);
  const prevPhaseRef = useRef(gameInfo?.phase);

  const websocketRef = useRef<WebSocket | null>(null);
  const [websocketStatus, setWebsocketStatus] = useState<string>("disconnected"); // WebSocket接続状態
  const hasConnectedRef = useRef(false);

  const connectWebSocket = () => {
    setWebsocketStatus("connecting");
    const ws = new WebSocket(`${process.env.NEXT_PUBLIC_WS_URL}/api/room/${params.id}/ws`);

    ws.onopen = () => {
      console.log("WebSocket接続が確立されました");
      setWebsocketStatus("connected");
    };

    ws.onmessage = event => {
      console.log("メッセージを受信しました:", event.data);

      const fullMessage: WebSocketMessage = JSON.parse(event.data);
      // サーバーからの応答を処理する
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

  // 新しい部屋が作成されたときのチャットログリセット
  useEffect(() => {
    if (roomInfo?.status === "Open") {
      localStorage.removeItem(`chat_messages_${params.id}`);
      setMessages([
        {
          id: Date.now().toString(),
          sender: "システム",
          message: "新しい部屋が作成されました",
          timestamp: new Date().toISOString(),
          type: "system",
        },
      ]);
    }
  }, [roomInfo?.status, params.id]);

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
        const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/room/${params.id}`);
        if (!response.ok) {
          throw new Error("ルーム情報の取得に失敗しました");
        }
        const data = await response.json();
        // console.log(data);
        setRoomInfo(data);
      } catch (error) {
        console.error("ルーム情報の取得エラー:", error);
      } finally {
        setIsLoading(false);
      }
    };

    const fetchGameInfo = async () => {
      try {
        const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/state`);
        if (!response.ok) {
          throw new Error("ゲーム情報の取得に失敗しました");
        }
        const data = await response.json();
        console.log(data);
        setGameInfo(data);
        // if (gameInfo?.chat_log?.messages) {
        //   setMessages(data.chat_log.messages);
        // }

        // システムメッセージを追加
        // const message: ChatMessage = {
        //   id: Date.now().toString(),
        //   sender: "システム",
        //   message: "夜の行動を実行しました",
        //   timestamp: new Date().toISOString(),
        //   type: "system",
        // };

        const message_from_server = data.chat_log.messages;

        // 各メッセージをChatMessage型に変換
        const messages: ChatMessage[] = message_from_server.map(
          (msg: { id: any; player_name: any; content: any; timestamp: any; message_type: string }) => ({
            id: msg.id,
            sender: msg.player_name,
            message: msg.content,
            timestamp: msg.timestamp,
            type: msg.message_type === "System" ? "system" : "normal",
          }),
        );
        setMessages(prev => [...messages]);
      } catch (error) {
        console.error("ゲーム情報の取得エラー:", error);
      } finally {
        setIsLoading(false);
      }
    };

    console.log("isStarting:", isStarting);

    fetchRoomInfo();
    // 定期的にルーム情報を更新
    const interval = setInterval(fetchRoomInfo, 5000);

    let gameInterval: NodeJS.Timeout | null = null;
    if (roomInfo?.status === "InProgress") {
      fetchGameInfo();
      gameInterval = setInterval(fetchGameInfo, 5000);
    }

    return () => {
      clearInterval(interval);
      if (gameInterval) {
        clearInterval(gameInterval);
      }
    };
  }, [roomInfo?.status, params.id]);

  useEffect(() => {
    if (!gameInfo) return;

    const prevPhase = prevPhaseRef.current;
    prevPhaseRef.current = gameInfo.phase;

    const checkGameResult = async () => {
      if (
        (prevPhase === "Night" && gameInfo.phase === "Discussion") ||
        (prevPhase === "Voting" && gameInfo.phase === "Result")
      ) {
        try {
          const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/check-winner`);
          if (!response.ok) {
            throw new Error("ゲーム結果の取得に失敗しました");
          }
          const result = await response.json();
          if (result !== "ゲーム進行中") {
            setGameInfo(prev => ({ ...prev!, result: result }));
            setShowGameResult(true);
          }
        } catch (error) {
          console.error("ゲーム結果の取得エラー:", error);
        }
      }
    };

    checkGameResult();
  }, [gameInfo?.phase, params.id]);

  const sendMessage = () => {
    console.log(websocketRef.current);
    if (websocketRef.current && websocketRef.current.readyState === WebSocket.OPEN && newMessage.trim() !== "") {
      // websocketRef.current.send(newMessage);
      const message: WebSocketMessage = {
        message_type: "normal",
        player_id: Date.now().toString(),
        player_name: "プレイヤー",
        content: newMessage,
        timestamp: new Date().toISOString(),
        room_id: params.id,
      };
      websocketRef.current.send(JSON.stringify(message));
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
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/start`, {
        method: "POST",
      });
      if (!response.ok) {
        throw new Error("ゲームの開始に失敗しました");
      }
      // ゲーム開始時にチャットログをリセット
      localStorage.removeItem(`chat_messages_${params.id}`);
      setMessages([
        {
          id: Date.now().toString(),
          sender: "システム",
          message: "ゲームが開始されました",
          timestamp: new Date().toISOString(),
          type: "system",
        },
      ]);
    } catch (error) {
      console.error("ゲーム開始エラー:", error);
    } finally {
      setIsStarting(false);
    }
  };

  const handleNightAction = async (targetPlayerId: string) => {
    try {
      // プレイヤーの役職に基づいてアクションタイプを決定
      if (!gameInfo) {
        throw new Error("ゲーム情報が取得できません");
      }
      const role = gameInfo.players.find(player => player.name === user?.username)?.role;

      const action: NightAction = (() => {
        switch (role) {
          case "Werewolf":
            return { Attack: { target_id: targetPlayerId } };
          case "Seer":
            return { Divine: { target_id: targetPlayerId } };
          case "Guard":
            return { Guard: { target_id: targetPlayerId } };
          default:
            throw new Error("夜の行動を実行できない役職です");
        }
      })();

      const request: NightActionRequest = {
        player_id: user?.id ?? "",
        action: action,
      };

      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/night-action`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(request),
        });

      if (!response.ok) {
        throw new Error("夜の行動の送信に失敗しました");
      }

      console.log(response);

      setShowNightAction(false);

      // システムメッセージを追加
      const message: ChatMessage = {
        id: Date.now().toString(),
        sender: "システム",
        message: "夜の行動を実行しました",
        timestamp: new Date().toISOString(),
        type: "system",
      };
      setMessages(prev => [...prev, message]);
    } catch (error) {
      console.error("夜の行動エラー:", error);
    }
  };

  const handleChangeRole = async (playerId: string, newRole: string) => {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/debug/change-role`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          player_id: playerId,
          new_role: newRole,
        }),
      });

      if (!response.ok) {
        throw new Error("役職の変更に失敗しました");
      }

      // システムメッセージを追加
      const message: ChatMessage = {
        id: Date.now().toString(),
        sender: "システム",
        message: `${gameInfo?.players.find(p => p.id === playerId)?.name || "Unknown"}の役職が${newRole}に変更されました`,
        timestamp: new Date().toISOString(),
        type: "system",
      };
      setMessages(prev => [...prev, message]);
    } catch (error) {
      console.error("役職変更エラー:", error);
    }
  };

  const handleVote = async (targetId: string) => {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/actions/vote`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          voter_id: user?.id,
          target_id: targetId,
        }),
      });

      if (!response.ok) {
        throw new Error("投票の送信に失敗しました");
      }

      // システムメッセージを追加
      const message: ChatMessage = {
        id: Date.now().toString(),
        sender: "システム",
        message: "投票を実行しました",
        timestamp: new Date().toISOString(),
        type: "system" as const,
      };
      setMessages(prev => [...prev, message]);
    } catch (error) {
      console.error("投票エラー:", error);
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
                {gameInfo && (
                  <>
                    {gameInfo.phase === "Night" ? (
                      <span className="flex items-center gap-2 text-indigo-600 bg-indigo-50 px-3 py-1 rounded-full">
                        <Moon size={16} />
                        夜フェーズ
                      </span>
                    ) : (
                      <span className="flex items-center gap-2 text-amber-600 bg-amber-50 px-3 py-1 rounded-full">
                        <Sun size={16} />
                        {gameInfo.phase}フェーズ
                      </span>
                    )}

                    {gameInfo.result !== "InProgress" && (
                      <span className="flex items-center gap-2 text-green-600 bg-green-50 px-3 py-1 rounded-full text-sm">
                        ゲーム結果: {gameInfo.result === "VillagerWin" ? "村人陣営の勝利" : "人狼陣営の勝利"}
                      </span>
                    )}

                    <span className="flex items-center gap-2 text-purple-600 bg-purple-50 px-3 py-1 rounded-full text-sm">
                      あなたの役職：{" "}
                      {(() => {
                        const role = gameInfo.players.find(player => player.name === user?.username)?.role;
                        switch (role) {
                          case "Seer":
                            return "占い師";
                          case "Werewolf":
                            return "人狼";
                          case "Villager":
                            return "村人";
                          default:
                            return "不明";
                        }
                      })()}
                    </span>

                    {gameInfo.result === "InProgress" && (
                      <>
                        {/* デバッグ用のフェーズ進行ボタン */}
                        <button
                          onClick={async () => {
                            try {
                              const response = await fetch(
                                `${process.env.NEXT_PUBLIC_API_URL}/api/game/${params.id}/phase/next`,
                                {
                                  method: "POST",
                                },
                              );
                              if (!response.ok) {
                                throw new Error("フェーズの進行に失敗しました");
                              }
                              // システムメッセージを追加
                              const message: ChatMessage = {
                                id: Date.now().toString(),
                                sender: "システム",
                                message: "フェーズが進行しました",
                                timestamp: new Date().toISOString(),
                                type: "system" as const,
                              };
                              setMessages(prev => [...prev, message]);
                            } catch (error) {
                              console.error("フェーズ進行エラー:", error);
                            }
                          }}
                          className="px-3 py-1 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-full text-sm border border-gray-300 transition-colors"
                        >
                          次のフェーズへ(デバッグ用)
                        </button>
                      </>
                    )}
                  </>
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
                    className={`px-4 py-2 rounded-lg text-white font-medium transition-colors ${isStarting || roomInfo.players.length < 2
                      ? "bg-gray-400 cursor-not-allowed"
                      : "bg-green-600 hover:bg-green-700"
                      }`}
                  >
                    {isStarting ? "開始中..." : "ゲーム開始"}
                  </button>
                )}
                {gameInfo?.phase === "Night" &&
                  !gameInfo.hasActed &&
                  gameInfo.players.find(player => player.name === user?.username)?.role !== "Villager" && (
                    <button
                      onClick={() => setShowNightAction(true)}
                      className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors shadow-sm"
                    >
                      夜の行動を実行
                    </button>
                  )}
                {gameInfo?.phase === "Voting" && (
                  <button
                    onClick={() => setShowVoteModal(true)}
                    className="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 transition-colors shadow-sm"
                  >
                    投票する
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
                {(gameInfo ? gameInfo.players : roomInfo.players).map(player => (
                  <div
                    key={player.id}
                    className={`flex flex-col p-2 rounded-lg ${player.is_dead === true ? "bg-gray-100 text-gray-500" : "bg-white text-indigo-900"
                      }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        {player.is_dead === false ? (
                          <UserCheck size={18} className="text-green-500" />
                        ) : (
                          <UserX size={18} className="text-red-500" />
                        )}
                        <span className={player.is_dead === true ? "line-through" : ""}>{player.name}</span>
                      </div>
                      {!player.isReady && (
                        <span className="text-xs bg-yellow-100 text-yellow-700 px-2 py-1 rounded">準備中</span>
                      )}
                    </div>

                    {/* デバッグ用の役職変更ボタン */}
                    {roomInfo.status === "InProgress" && (
                      <div className="mt-2 flex gap-1 text-xs">
                        <button
                          onClick={() => handleChangeRole(player.id, "村人")}
                          className={`px-2 py-1 rounded transition-colors ${player.role === "Villager"
                            ? "bg-gray-300 text-gray-800 font-medium border-gray-500 border-2"
                            : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                            }`}
                        >
                          村人
                        </button>
                        <button
                          onClick={() => handleChangeRole(player.id, "人狼")}
                          className={`px-2 py-1 rounded transition-colors ${player.role === "Werewolf"
                            ? "bg-red-300 text-red-800 font-medium border-red-500 border-2"
                            : "bg-red-100 text-red-600 hover:bg-red-200"
                            }`}
                        >
                          人狼
                        </button>
                        <button
                          onClick={() => handleChangeRole(player.id, "占い師")}
                          className={`px-2 py-1 rounded transition-colors ${player.role === "Seer"
                            ? "bg-blue-300 text-blue-800 font-medium border-blue-500 border-2"
                            : "bg-blue-100 text-blue-600 hover:bg-blue-200"
                            }`}
                        >
                          占い師
                        </button>
                      </div>
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

                {/* 鍵生成デバッグ用 */}
                <div className="p-4 border-b border-indigo-100">
                  <h2 className="text-lg font-semibold text-indigo-900">鍵生成(デバッグ用)</h2>
                  <button
                    onClick={async () => {
                      try {
                        if (!gameInfo || !user) {
                          console.error("ゲーム情報またはユーザー情報が利用できません");
                          return;
                        }
                        const playerId = gameInfo.players.find(player => player.name === user?.username)?.id;
                        if (!playerId) {
                          console.error("ユーザーIDが利用できません");
                          return;
                        }
                        console.log("Generating keys for player:", playerId);

                        const keyManager = new TweetNaclKeyManager();
                        const publicKey = await keyManager.generateAndSaveKeyPair(playerId);
                        console.log("Keys generated and saved, public key:", publicKey);

                        const loadSuccess = await keyManager.loadKeyPairFromStorage(playerId);
                        if (loadSuccess) {
                          console.log("Keys loaded successfully");
                          console.log("Public key verified:", keyManager.getPublicKey());
                          // システムメッセージを追加
                          const message: ChatMessage = {
                            id: Date.now().toString(),
                            sender: "システム",
                            message: "鍵ペアが生成されました",
                            timestamp: new Date().toISOString(),
                            type: "system",
                          };
                          setMessages(prev => [...prev, message]);
                        }
                      } catch (error) {
                        console.error("Error managing keys:", error);
                        // エラーメッセージを追加
                        const message: ChatMessage = {
                          id: Date.now().toString(),
                          sender: "システム",
                          message: "鍵ペアの生成に失敗しました",
                          timestamp: new Date().toISOString(),
                          type: "system",
                        };
                        setMessages(prev => [...prev, message]);
                      }
                    }}
                    disabled={!user?.id}
                    className="bg-purple-500 hover:bg-purple-700 text-white font-bold py-2 px-4 rounded mt-2 disabled:bg-gray-400 disabled:cursor-not-allowed"
                  >
                    鍵生成と保存
                  </button>
                </div>
              </div>
            </div>

            {/* Chat Area */}
            <div className="flex-1 flex flex-col">
              <div className="flex-1 overflow-y-auto p-4">
                {messages.map(msg => (
                  <div
                    key={msg.id}
                    className={`mb-4 rounded-lg p-3 ${msg.type === "system"
                      ? "bg-indigo-50 text-indigo-700 text-left"
                      : msg.type === "whisper"
                        ? "bg-purple-50 text-purple-700 italic"
                        : "bg-white"
                      }`}
                  >
                    <span className="text-s text-gray-500">
                      {new Date(msg.timestamp).toLocaleTimeString("ja-JP", {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      })}
                    </span>{" "}
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
        <div className="p-4 border-t border-indigo-100">
          <h2 className="text-lg font-semibold text-indigo-900">デバッグ情報</h2>
          <pre className="text-sm text-gray-700 bg-gray-100 p-2 rounded overflow-x-auto">
            gameInfo = {JSON.stringify(gameInfo, null, 2)}
          </pre>
        </div>
      </div>
      {showNightAction && gameInfo?.phase === "Night" && (
        <NightActionModal
          players={gameInfo.players}
          role={gameInfo.players.find(player => player.name === user?.username)?.role ?? "Villager"}
          onSubmit={handleNightAction}
          onClose={() => setShowNightAction(false)}
        />
      )}
      {showVoteModal && gameInfo?.phase === "Voting" && (
        <VoteModal
          myId={gameInfo.players.find(player => player.name === user?.username)?.id ?? ""}
          roomId={gameInfo.room_id}
          players={gameInfo.players}
          onSubmit={handleVote}
          onClose={() => setShowVoteModal(false)}
        />
      )}
      {gameInfo?.result && showGameResult && (
        <GameResultModal result={gameInfo.result} onClose={() => setShowGameResult(false)} />
      )}
    </div>
  );
}
