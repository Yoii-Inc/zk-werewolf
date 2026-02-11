"use client";

import React, { useEffect, useRef, useState } from "react";
import { GameResultModal } from "../../../components/game/GameResultModal";
import NightActionModal from "../../../components/game/NightActionModal";
import VoteModal from "../../../components/game/VoteModal";
import { Clock, Moon, Send, StickyNote, Sun, UserCheck, UserX, Users } from "lucide-react";
import { useAuth } from "~~/app/contexts/AuthContext";
import { useComputationResults } from "~~/hooks/useComputationResults";
import { useGameActions } from "~~/hooks/useGameActions";
import { useGameChat } from "~~/hooks/useGameChat";
import { useGameInfo } from "~~/hooks/useGameInfo";
import { useGamePhase } from "~~/hooks/useGamePhase";
import { useGameWebSocket } from "~~/hooks/useGameWebSocket";
import * as GameInputGenerator from "~~/services/gameInputGenerator";
import { TweetNaclKeyManager } from "~~/utils/crypto/tweetNaclKeyManager";

const isDebugMode = process.env.NEXT_PUBLIC_DEBUG_MODE === "true";

export default function RoomPage({ params }: { params: { id: string } }) {
  const { user } = useAuth();

  // State for UI components
  const [newMessage, setNewMessage] = useState("");
  const [notes, setNotes] = useState("");
  const [showNightAction, setShowNightAction] = useState(false);
  const [showVoteModal, setShowVoteModal] = useState(false);
  const [showGameResult, setShowGameResult] = useState(false);
  const [isTogglingReady, setIsTogglingReady] = useState(false);
  const [timerBaseline, setTimerBaseline] = useState(1);
  const chatEndRef = useRef<HTMLDivElement>(null);

  // Custom hooks
  const { messages, setMessages, addMessage, addServerMessage, resetMessages } = useGameChat(params.id, null);
  const { roomInfo, gameInfo, isLoading, privateGameInfo } = useGameInfo(params.id, user?.id, setMessages);
  const { websocketRef, websocketStatus, connectWebSocket, disconnectWebSocket, sendMessage } = useGameWebSocket(
    params.id,
    addServerMessage,
    user?.username,
  );
  const {
    isStarting,
    startGame,
    // handleNightAction,
    // handleVote,
    handleChangeRole,
    nextPhase,
    resetGame,
    resetBatchRequest,
  } = useGameActions(params.id, addMessage, gameInfo, user?.id);

  // Phase monitoring
  useGamePhase(gameInfo, params.id, addMessage, user?.username);

  // Computation results monitoring
  useComputationResults(params.id, user?.id || "", addMessage, gameInfo);

  const getPlayerReady = (player: { isReady?: boolean; is_ready?: boolean }) => {
    if (typeof player.isReady === "boolean") return player.isReady;
    if (typeof player.is_ready === "boolean") return player.is_ready;
    return false;
  };

  // ゲームリセット通知を監視
  useEffect(() => {
    const handleGameReset = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { roomId } = customEvent.detail;

      console.log("Game reset notification received, clearing messages and crypto state");
      resetMessages();

      // 暗号パラメータとランダムネスの初期化状態をリセット
      GameInputGenerator.resetGameCryptoState(roomId);

      // privateGameInfoのplayerRoleをnullにリセット
      if (user?.id) {
        const existingInfo = privateGameInfo;
        if (existingInfo) {
          const updatedInfo = {
            ...existingInfo,
            playerRole: null as any,
            hasActed: false,
          };
          sessionStorage.setItem(`game_${roomId}_player_${user.id}`, JSON.stringify(updatedInfo));
          console.log("PrivateGameInfo reset: playerRole set to null");
        }
      }
    };

    window.addEventListener("gameResetNotification", handleGameReset);

    return () => {
      window.removeEventListener("gameResetNotification", handleGameReset);
    };
  }, [resetMessages, user?.id, privateGameInfo]);

  // ゲーム終了を検知してモーダルを表示
  useEffect(() => {
    if (gameInfo?.result && gameInfo.result !== "InProgress") {
      setShowGameResult(true);
    }
  }, [gameInfo?.result]);

  const handleSendMessage = () => {
    if (newMessage.trim() !== "") {
      const success = sendMessage(newMessage);
      if (success) {
        setNewMessage("");
      } else {
        console.error("Failed to send message");
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

  const handleToggleReady = async () => {
    if (!roomInfo || !user?.id) return;
    setIsTogglingReady(true);

    try {
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/room/${roomInfo.room_id}/ready/${user.id}`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${localStorage.getItem("token") ?? ""}`,
          },
        },
      );

      if (!response.ok) {
        throw new Error("Failed to toggle ready state");
      }
    } catch (error) {
      console.error("Ready toggle error:", error);
      addMessage({
        id: Date.now().toString(),
        sender: "System",
        message: "Failed to update ready status.",
        timestamp: new Date().toISOString(),
        type: "system",
      });
    } finally {
      setIsTogglingReady(false);
    }
  };

  const handleKeyGeneration = async () => {
    try {
      if (!gameInfo || !user) {
        console.error("Game info or user info not available");
        return;
      }
      const playerId = gameInfo.players.find(player => player.name === user?.username)?.id;
      if (!playerId) {
        console.error("User ID not available");
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
          sender: "System",
          message: "Key pair generated successfully",
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    } catch (error) {
      console.error("Error managing keys:", error);
      addMessage({
        id: Date.now().toString(),
        sender: "System",
        message: "Failed to generate key pair",
        timestamp: new Date().toISOString(),
        type: "system",
      });
    }
  };

  useEffect(() => {
    if (roomInfo?.remainingTime && roomInfo.remainingTime > timerBaseline) {
      setTimerBaseline(roomInfo.remainingTime);
    }
  }, [roomInfo?.remainingTime, timerBaseline]);

  const playersForView = (gameInfo ? gameInfo.players : roomInfo?.players) ?? [];
  const currentPlayer = playersForView.find(player => player.id === user?.id || player.name === user?.username);
  const isCurrentPlayerReady = currentPlayer
    ? getPlayerReady(currentPlayer as { isReady?: boolean; is_ready?: boolean })
    : false;
  const allPlayersReady =
    playersForView.length > 0 &&
    playersForView.every(player => getPlayerReady(player as { isReady?: boolean; is_ready?: boolean }));
  const timerProgressPercent = roomInfo?.remainingTime
    ? Math.max(0, Math.min(100, (roomInfo.remainingTime / Math.max(timerBaseline, 1)) * 100))
    : 0;
  const isTimeWarning = (roomInfo?.remainingTime ?? 0) <= 30;

  return (
    <div className="h-screen flex bg-gradient-to-br from-indigo-50 to-purple-50">
      {isLoading ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-xl text-indigo-600">Loading room information...</div>
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
                    Room Status: Open (Waiting for players)
                  </span>
                )}
                {gameInfo && (
                  <>
                    {gameInfo.phase === "Night" ? (
                      <span className="flex items-center gap-2 text-indigo-600 bg-indigo-50 px-3 py-1 rounded-full">
                        <Moon size={16} />
                        Night Phase
                      </span>
                    ) : (
                      <span className="flex items-center gap-2 text-amber-600 bg-amber-50 px-3 py-1 rounded-full">
                        <Sun size={16} />
                        {gameInfo.phase} Phase
                      </span>
                    )}

                    {gameInfo.result !== "InProgress" && (
                      <span className="flex items-center gap-2 text-green-600 bg-green-50 px-3 py-1 rounded-full text-sm">
                        Game Result: {gameInfo.result === "VillagerWin" ? "Villagers Win" : "Werewolves Win"}
                      </span>
                    )}

                    <span className="flex items-center gap-2 text-purple-600 bg-purple-50 px-3 py-1 rounded-full text-sm">
                      Your Role: {privateGameInfo?.playerRole ?? "Unknown"}
                    </span>

                    {isDebugMode && gameInfo.result === "InProgress" && (
                      <>
                        {/* デバッグ用のフェーズ進行ボタン */}
                        <button
                          onClick={async () => {
                            const success = await nextPhase();
                            if (!success) {
                              console.error("Failed to advance phase");
                            }
                          }}
                          className="px-3 py-1 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-full text-sm border border-gray-300 transition-colors"
                        >
                          Next Phase (Debug)
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
                    {roomInfo.players.length}/{roomInfo.max_players} players
                  </span>
                </div>
                <div className="flex items-center gap-2 text-indigo-700">
                  <Clock size={18} />
                  <span>
                    Time Remaining: {Math.floor(roomInfo.remainingTime / 60)}:
                    {String(roomInfo.remainingTime % 60).padStart(2, "0")}
                  </span>
                </div>
                <div className="w-40">
                  <div className="h-2 rounded-full bg-indigo-100 overflow-hidden">
                    <div
                      className={`h-full transition-all duration-500 ${isTimeWarning ? "bg-red-500" : "bg-indigo-500"}`}
                      style={{ width: `${timerProgressPercent}%` }}
                    />
                  </div>
                </div>
                {roomInfo.status === "Open" && currentPlayer && (
                  <button
                    onClick={handleToggleReady}
                    disabled={isTogglingReady}
                    className={`px-4 py-2 rounded-lg font-medium transition-colors border ${
                      isCurrentPlayerReady
                        ? "bg-green-100 text-green-800 border-green-300 hover:bg-green-200"
                        : "bg-amber-100 text-amber-800 border-amber-300 hover:bg-amber-200"
                    } ${isTogglingReady ? "opacity-70 cursor-not-allowed" : ""}`}
                  >
                    {isTogglingReady ? "Updating..." : isCurrentPlayerReady ? "Ready: ON" : "Ready: OFF"}
                  </button>
                )}
                {roomInfo.status === "Open" && (
                  <button
                    onClick={handleStartGame}
                    disabled={isStarting || roomInfo.players.length < 2}
                    className={`px-4 py-2 rounded-lg text-white font-medium transition-colors ${
                      isStarting || roomInfo.players.length < 2
                        ? "bg-gray-400 cursor-not-allowed"
                        : allPlayersReady
                          ? "bg-emerald-600 hover:bg-emerald-700 ring-2 ring-emerald-300"
                          : "bg-green-600 hover:bg-green-700"
                    }`}
                  >
                    {isStarting ? "Starting..." : "Start Game"}
                  </button>
                )}
                {gameInfo?.phase === "Night" &&
                  !gameInfo.hasActed &&
                  privateGameInfo?.playerRole !== "Villager" &&
                  privateGameInfo?.playerRole !== null && (
                    <button
                      onClick={() => setShowNightAction(true)}
                      className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors shadow-sm"
                    >
                      Execute Night Action
                    </button>
                  )}
                {gameInfo?.phase === "Voting" && (
                  <button
                    onClick={() => setShowVoteModal(true)}
                    className="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 transition-colors shadow-sm"
                  >
                    Vote
                  </button>
                )}
              </div>
            </div>
          </div>

          <div className="flex-1 flex">
            {/* Players List */}
            <div className="w-64 bg-white/80 backdrop-blur-sm border-l border-indigo-100">
              <div className="p-4 border-b border-indigo-100">
                <h2 className="text-lg font-semibold text-indigo-900">Players List</h2>
              </div>
              <div className="p-4 space-y-3">
                {(gameInfo ? gameInfo.players : roomInfo.players).map(player => {
                  const isMe = player.name === user?.username;
                  return (
                    <div
                      key={player.id}
                      className={`flex flex-col p-2 rounded-lg ${
                        player.is_dead === true
                          ? "bg-gray-100 text-gray-500 opacity-60"
                          : isMe
                            ? "bg-indigo-50 text-indigo-900 ring-2 ring-indigo-300"
                            : "bg-white text-indigo-900"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          {player.is_dead === false ? (
                            <UserCheck size={18} className="text-green-500" />
                          ) : (
                            <UserX size={18} className="text-red-500" />
                          )}
                          <span className={player.is_dead === true ? "line-through" : ""}>
                            {player.name}
                            {isMe && <span className="text-xs text-indigo-500 ml-1">(You)</span>}
                          </span>
                        </div>
                        {!getPlayerReady(player as { isReady?: boolean; is_ready?: boolean }) && (
                          <span className="text-xs bg-yellow-100 text-yellow-700 px-2 py-1 rounded">
                            In preparation
                          </span>
                        )}
                      </div>

                      {/* デバッグ用の役職変更ボタン */}
                      {isDebugMode && roomInfo.status === "InProgress" && (
                        <div className="mt-2 flex gap-1 text-xs">
                          <button
                            onClick={() => handleChangeRole(player.id, "Villager")}
                            className={`px-2 py-1 rounded transition-colors ${
                              player.role === "Villager"
                                ? "bg-gray-300 text-gray-800 font-medium border-gray-500 border-2"
                                : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                            }`}
                          >
                            Villager
                          </button>
                          <button
                            onClick={() => handleChangeRole(player.id, "Werewolf")}
                            className={`px-2 py-1 rounded transition-colors ${
                              player.role === "Werewolf"
                                ? "bg-red-300 text-red-800 font-medium border-red-500 border-2"
                                : "bg-red-100 text-red-600 hover:bg-red-200"
                            }`}
                          >
                            Werewolf
                          </button>
                          <button
                            onClick={() => handleChangeRole(player.id, "Seer")}
                            className={`px-2 py-1 rounded transition-colors ${
                              player.role === "Seer"
                                ? "bg-blue-300 text-blue-800 font-medium border-blue-500 border-2"
                                : "bg-blue-100 text-blue-600 hover:bg-blue-200"
                            }`}
                          >
                            Seer
                          </button>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
              {isDebugMode && (
                <div>
                  {/* webscoket関連(デバッグ用)とかく */}
                  <div className="p-4 border-b border-indigo-100">
                    <h2 className="text-lg font-semibold text-indigo-900">WebSocket (Debug)</h2>
                    <div className="text-indigo-700">WebSocket URL: ws://localhost:8080/api/room/ws</div>
                    <div className="text-indigo-700">WebSocket ReadyState: {websocketRef.current?.readyState}</div>
                    <div className="text-indigo-700">WebSocket Status: {websocketStatus}</div>
                  </div>

                  <button
                    onClick={connectWebSocket}
                    disabled={websocketStatus === "connected" || websocketStatus === "connecting"}
                    className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                  >
                    {websocketStatus === "connecting" ? "Connecting..." : "Connect WebSocket"}
                  </button>
                  <button
                    onClick={disconnectWebSocket}
                    disabled={websocketStatus === "disconnected"}
                    className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                  >
                    Disconnect WebSocket
                  </button>

                  {/* 鍵生成デバッグ用 */}
                  <div className="p-4 border-b border-indigo-100">
                    <h2 className="text-lg font-semibold text-indigo-900">Key Generation (Debug)</h2>
                    <button
                      onClick={handleKeyGeneration}
                      disabled={!user?.id}
                      className="bg-purple-500 hover:bg-purple-700 text-white font-bold py-2 px-4 rounded mt-2 disabled:bg-gray-400 disabled:cursor-not-allowed"
                    >
                      Generate & Save Keys
                    </button>
                  </div>

                  {/* ゲームリセット(デバッグ用) */}
                  <div className="p-4 border-b border-indigo-100">
                    <h2 className="text-lg font-semibold text-indigo-900">Game Reset (Debug)</h2>
                    <button
                      onClick={async () => {
                        const success = await resetGame();
                        if (success) {
                          resetMessages();
                        }
                      }}
                      className="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded mt-2"
                    >
                      Reset Game
                    </button>
                  </div>

                  {/* バッチリクエストリセット(デバッグ用) */}
                  <div className="p-4 border-b border-indigo-100">
                    <h2 className="text-lg font-semibold text-indigo-900">Batch Request Reset (Debug)</h2>
                    <button
                      onClick={async () => {
                        const success = await resetBatchRequest();
                        if (!success) {
                          addMessage({
                            id: Date.now().toString(),
                            sender: "System",
                            message: "Failed to reset batch request",
                            timestamp: new Date().toISOString(),
                            type: "system",
                          });
                        }
                      }}
                      className="bg-orange-500 hover:bg-orange-700 text-white font-bold py-2 px-4 rounded mt-2"
                    >
                      Reset Batch Request
                    </button>
                  </div>
                </div>
              )}
            </div>

            {/* Chat Area */}
            <div className="flex-1 flex flex-col">
              <div className="h-[600px] overflow-y-auto p-4">
                {messages.map((msg, index) => (
                  <div
                    key={`${msg.id}-${index}`}
                    className={`mb-4 rounded-lg p-3 break-words ${
                      msg.type === "system"
                        ? msg.source === "client"
                          ? msg.message.includes("🐺")
                            ? "bg-red-100 text-red-700 text-left" // 人狼占い結果
                            : msg.message.includes("✅")
                              ? "bg-green-100 text-green-700 text-left" // 非人狼占い結果
                              : "bg-lime-100 text-lime-700 text-left" // クライアント側システムメッセージは黄緑
                          : "bg-indigo-100 text-indigo-700 text-left" // サーバー側システムメッセージは蒼
                        : msg.type === "whisper"
                          ? "bg-purple-50 text-purple-700 italic"
                          : "bg-white"
                    }`}
                  >
                    <span className="text-xs text-gray-400">
                      {new Date(msg.timestamp).toLocaleTimeString("ja-JP", {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      })}
                    </span>{" "}
                    <span className="font-semibold">{msg.sender}: </span>
                    <span className="whitespace-pre-wrap">{msg.message}</span>
                  </div>
                ))}
                <div ref={chatEndRef} />
              </div>

              {/* Message Input */}
              <div className="p-4">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newMessage}
                    onChange={e => setNewMessage(e.target.value)}
                    onKeyDown={e => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        handleSendMessage();
                      }
                    }}
                    placeholder="Enter message..."
                    className="flex-1 border border-indigo-200 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-indigo-500 bg-white/80 backdrop-blur-sm"
                  />
                  <button
                    onClick={handleSendMessage}
                    className="bg-indigo-600 text-white px-6 py-2 rounded-lg flex items-center gap-2 hover:bg-indigo-700 transition-colors shadow-sm"
                  >
                    <Send size={20} />
                    Send
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-xl text-red-600">Failed to retrieve room information.</div>
        </div>
      )}

      {/* Notes Panel */}
      <div className="w-80 bg-white/80 backdrop-blur-sm border-l border-indigo-100 flex flex-col">
        <div className="p-4 border-b border-indigo-100 flex items-center gap-2">
          <StickyNote size={20} className="text-indigo-600" />
          <h2 className="text-lg font-semibold text-indigo-900">Notes</h2>
        </div>
        <textarea
          value={notes}
          onChange={e => setNotes(e.target.value)}
          placeholder="Enter notes..."
          className="flex-1 p-4 resize-none focus:outline-none bg-transparent"
        />
        {isDebugMode && (
          <div className="p-4 border-t border-indigo-100">
            <h2 className="text-lg font-semibold text-indigo-900">Debug Info</h2>
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
        )}
      </div>
      {showNightAction && gameInfo?.phase === "Night" && (
        <NightActionModal
          players={gameInfo.players}
          role={privateGameInfo?.playerRole ?? "Villager"}
          gameInfo={gameInfo}
          username={user?.username ?? ""}
          onSubmit={(targetPlayerId: string) => {
            // handleNightAction(targetPlayerId, privateGameInfo?.playerRole);
            setShowNightAction(false);
          }}
          onClose={() => setShowNightAction(false)}
          roomId={gameInfo.room_id}
          myId={privateGameInfo?.playerId ?? ""}
        />
      )}
      {showVoteModal && gameInfo?.phase === "Voting" && (
        <VoteModal
          myId={gameInfo.players.find(player => player.name === user?.username)?.id ?? ""}
          roomId={gameInfo.room_id}
          players={gameInfo.players}
          gameInfo={gameInfo}
          username={user?.username ?? ""}
          onSubmit={(targetId: string) => {
            // handleVote(targetId);
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
