"use client";

import React, { useState } from "react";
import { GameResultModal } from "../../../components/game/GameResultModal";
import NightActionModal from "../../../components/game/NightActionModal";
import VoteModal from "../../../components/game/VoteModal";
import { Clock, Moon, Send, StickyNote, Sun, UserCheck, UserX, Users } from "lucide-react";
import { useAuth } from "~~/app/contexts/AuthContext";
import { useGameActions } from "~~/hooks/useGameActions";
import { useGameChat } from "~~/hooks/useGameChat";
import { useGameInfo } from "~~/hooks/useGameInfo";
import { useGamePhase } from "~~/hooks/useGamePhase";
import { useGameWebSocket } from "~~/hooks/useGameWebSocket";
import type { ChatMessage } from "~~/types/game";
import { TweetNaclKeyManager } from "~~/utils/crypto/tweetNaclKeyManager";

export default function RoomPage({ params }: { params: { id: string } }) {
  const { isAuthenticated, user, logout } = useAuth();

  // State for UI components
  const [newMessage, setNewMessage] = useState("");
  const [notes, setNotes] = useState("");
  const [showNightAction, setShowNightAction] = useState(false);
  const [showVoteModal, setShowVoteModal] = useState(false);
  const [showGameResult, setShowGameResult] = useState(false);

  // Custom hooks
  const { messages, setMessages, addMessage, resetMessages } = useGameChat(params.id, null);
  const { roomInfo, gameInfo, isLoading, privateGameInfo } = useGameInfo(params.id, user?.id, setMessages);
  const { websocketRef, websocketStatus, connectWebSocket, disconnectWebSocket, sendMessage } = useGameWebSocket(
    params.id,
    setMessages,
  );
  const {
    isStarting,
    startGame,
    handleNightAction,
    handleVote,
    handleChangeRole,
    nextPhase,
    resetGame,
    resetBatchRequest,
  } = useGameActions(params.id, addMessage, gameInfo, user?.id);

  // Phase monitoring
  useGamePhase(gameInfo, params.id, addMessage, user?.username);

  // Update chat hook with room info
  const chatHook = useGameChat(params.id, roomInfo);

  const handleSendMessage = () => {
    if (newMessage.trim() !== "") {
      const success = sendMessage(newMessage);
      if (success) {
        setNewMessage("");
      } else {
        console.error("メッセージの送信に失敗しました");
      }
    }
  };

  const handleStartGame = async () => {
    console.log("Starting game...");
    const success = await startGame();
    if (success) {
      console.log("Game started successfully"); // デバッグログ
      resetMessages();
    } else {
      console.error("Failed to start game"); // デバッグログ
    }
  };

  const handleKeyGeneration = async () => {
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
        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: "鍵ペアが生成されました",
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    } catch (error) {
      console.error("Error managing keys:", error);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "鍵ペアの生成に失敗しました",
        timestamp: new Date().toISOString(),
        type: "system",
      });
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
                            const success = await nextPhase();
                            if (!success) {
                              console.error("フェーズの進行に失敗しました");
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
                    onClick={handleStartGame}
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
                    className={`flex flex-col p-2 rounded-lg ${
                      player.is_dead === true ? "bg-gray-100 text-gray-500" : "bg-white text-indigo-900"
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
                          className={`px-2 py-1 rounded transition-colors ${
                            player.role === "Villager"
                              ? "bg-gray-300 text-gray-800 font-medium border-gray-500 border-2"
                              : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                          }`}
                        >
                          村人
                        </button>
                        <button
                          onClick={() => handleChangeRole(player.id, "人狼")}
                          className={`px-2 py-1 rounded transition-colors ${
                            player.role === "Werewolf"
                              ? "bg-red-300 text-red-800 font-medium border-red-500 border-2"
                              : "bg-red-100 text-red-600 hover:bg-red-200"
                          }`}
                        >
                          人狼
                        </button>
                        <button
                          onClick={() => handleChangeRole(player.id, "占い師")}
                          className={`px-2 py-1 rounded transition-colors ${
                            player.role === "Seer"
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

                {/* ゲームリセット(デバッグ用) */}
                <div className="p-4 border-b border-indigo-100">
                  <h2 className="text-lg font-semibold text-indigo-900">ゲームリセット(デバッグ用)</h2>
                  <button
                    onClick={async () => {
                      const success = await resetGame();
                      if (success) {
                        addMessage({
                          id: Date.now().toString(),
                          sender: "システム",
                          message: "ゲームがリセットされました",
                          timestamp: new Date().toISOString(),
                          type: "system",
                        });
                      }
                    }}
                    className="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded mt-2"
                  >
                    ゲームをリセット
                  </button>
                </div>

                {/* バッチリクエストリセット(デバッグ用) */}
                <div className="p-4 border-b border-indigo-100">
                  <h2 className="text-lg font-semibold text-indigo-900">バッチリクエストリセット(デバッグ用)</h2>
                  <button
                    onClick={async () => {
                      const success = await resetBatchRequest();
                      if (!success) {
                        addMessage({
                          id: Date.now().toString(),
                          sender: "システム",
                          message: "バッチリクエストのリセットに失敗しました",
                          timestamp: new Date().toISOString(),
                          type: "system",
                        });
                      }
                    }}
                    className="bg-orange-500 hover:bg-orange-700 text-white font-bold py-2 px-4 rounded mt-2"
                  >
                    バッチリクエストをリセット
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
                    className={`mb-4 rounded-lg p-3 ${
                      msg.type === "system"
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
                    onClick={handleSendMessage}
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
            username = {user?.username}
          </pre>
          <pre className="text-sm text-gray-700 bg-gray-100 p-2 rounded overflow-x-auto">
            privateGameInfo = {JSON.stringify(privateGameInfo, null, 2)}
          </pre>
          <pre className="text-sm text-gray-700 bg-gray-100 p-2 rounded overflow-x-auto">
            gameInfo = {JSON.stringify(gameInfo, null, 2)}
          </pre>
          <pre className="text-sm text-gray-700 bg-gray-100 p-2 rounded overflow-x-auto">
            roomInfo = {JSON.stringify(roomInfo, null, 2)}
          </pre>
        </div>
      </div>
      {showNightAction && gameInfo?.phase === "Night" && (
        <NightActionModal
          players={gameInfo.players}
          role={gameInfo.players.find(player => player.name === user?.username)?.role ?? "Villager"}
          onSubmit={(targetPlayerId: string) => {
            const userRole = gameInfo.players.find(player => player.name === user?.username)?.role;
            handleNightAction(targetPlayerId, userRole);
            setShowNightAction(false);
          }}
          onClose={() => setShowNightAction(false)}
        />
      )}
      {showVoteModal && gameInfo?.phase === "Voting" && (
        <VoteModal
          myId={gameInfo.players.find(player => player.name === user?.username)?.id ?? ""}
          roomId={gameInfo.room_id}
          players={gameInfo.players}
          onSubmit={(targetId: string) => {
            handleVote(targetId);
            setShowVoteModal(false);
          }}
          onClose={() => setShowVoteModal(false)}
        />
      )}
      {gameInfo?.result && showGameResult && (
        <GameResultModal result={gameInfo.result} onClose={() => setShowGameResult(false)} />
      )}
    </div>
  );
}
