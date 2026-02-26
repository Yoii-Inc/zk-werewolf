import { useCallback, useEffect, useState } from "react";
import type { ChatMessage } from "~~/types/game";

// localStorageのキーを生成
const getClientMessagesKey = (roomId: string) => `client_messages_${roomId}`;

// クライアント側メッセージをlocalStorageから取得
const loadClientMessages = (roomId: string): ChatMessage[] => {
  if (typeof window === "undefined") return [];
  try {
    const stored = localStorage.getItem(getClientMessagesKey(roomId));
    return stored ? JSON.parse(stored) : [];
  } catch (error) {
    console.error("Failed to load client messages:", error);
    return [];
  }
};

// クライアント側メッセージをlocalStorageに保存
const saveClientMessages = (roomId: string, messages: ChatMessage[]) => {
  if (typeof window === "undefined") return;
  try {
    localStorage.setItem(getClientMessagesKey(roomId), JSON.stringify(messages));
  } catch (error) {
    console.error("Failed to save client messages:", error);
  }
};

const getMessageKey = (message: ChatMessage) =>
  `${message.source ?? "unknown"}|${message.id ?? ""}|${message.sender}|${message.message}|${message.timestamp}`;

const parseTimestamp = (timestamp: string) => {
  const value = new Date(timestamp).getTime();
  return Number.isNaN(value) ? 0 : value;
};

export const useGameChat = (roomId: string) => {
  // サーバー側メッセージ（setMessagesで外部から設定）
  const [serverMessages, setServerMessages] = useState<ChatMessage[]>([]);

  // クライアント側メッセージ（addMessageで追加、localStorageに保存）
  const [clientMessages, setClientMessages] = useState<ChatMessage[]>(() => loadClientMessages(roomId));

  // マージされた全メッセージ（タイムスタンプでソート）
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  // サーバー側とクライアント側のメッセージをマージ
  useEffect(() => {
    const dedupedMap = new Map<string, ChatMessage>();
    [...serverMessages, ...clientMessages].forEach(message => {
      const key = getMessageKey(message);
      if (!dedupedMap.has(key)) {
        dedupedMap.set(key, message);
      }
    });

    const merged = Array.from(dedupedMap.values()).sort((a, b) => {
      const timeDiff = parseTimestamp(a.timestamp) - parseTimestamp(b.timestamp);
      if (timeDiff !== 0) return timeDiff;
      return getMessageKey(a).localeCompare(getMessageKey(b));
    });

    setMessages(merged);
  }, [serverMessages, clientMessages]);

  // クライアント側メッセージが変更されたらlocalStorageに保存
  useEffect(() => {
    saveClientMessages(roomId, clientMessages);
  }, [roomId, clientMessages]);

  // クライアント側メッセージを追加（占い結果、エラーメッセージなど）
  const addMessage = useCallback((message: ChatMessage) => {
    const messageWithSource: ChatMessage = {
      ...message,
      source: message.source ?? "client", // sourceが未設定ならclientを設定
    };
    setClientMessages(prev => {
      const key = getMessageKey(messageWithSource);
      if (prev.some(existing => getMessageKey(existing) === key)) {
        return prev;
      }
      return [...prev, messageWithSource];
    });
  }, []);

  // サーバー側メッセージを追加（WebSocketで受信したメッセージなど）
  const addServerMessage = useCallback((message: ChatMessage) => {
    const messageWithSource: ChatMessage = {
      ...message,
      source: "server",
    };
    setServerMessages(prev => {
      const key = getMessageKey(messageWithSource);
      if (prev.some(existing => getMessageKey(existing) === key)) {
        return prev;
      }
      return [...prev, messageWithSource];
    });
  }, []);

  // サーバー側メッセージを設定（useGameInfoから呼ばれる）
  const setServerMessagesFromAPI = useCallback((apiMessages: ChatMessage[]) => {
    const messagesWithSource = apiMessages.map(msg => ({
      ...msg,
      source: "server" as const,
    }));
    setServerMessages(messagesWithSource);
  }, []);

  // メッセージをリセット（ゲームリセット時）
  const resetMessages = useCallback(() => {
    setServerMessages([]);
    setClientMessages([]);
    localStorage.removeItem(getClientMessagesKey(roomId));
  }, [roomId]);

  return {
    messages,
    setMessages: setServerMessagesFromAPI, // useGameInfoがsetMessagesを呼ぶ
    addMessage,
    addServerMessage, // WebSocketでのリアルタイム追加用
    resetMessages,
  };
};
