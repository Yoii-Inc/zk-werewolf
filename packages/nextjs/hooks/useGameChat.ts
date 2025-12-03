import { useEffect, useState } from "react";
import type { ChatMessage, RoomInfo } from "~~/types/game";

const mockMessages: ChatMessage[] = [
  {
    id: "1",
    sender: "System",
    message: "This is the default message.",
    timestamp: new Date().toISOString(),
    type: "system",
  },
];

export const useGameChat = (roomId: string, roomInfo: RoomInfo | null) => {
  const [messages, setMessages] = useState<ChatMessage[]>(() => {
    if (typeof window !== "undefined") {
      const savedMessages = localStorage.getItem(`chat_messages_${roomId}`);
      return savedMessages ? JSON.parse(savedMessages) : mockMessages;
    }
    return mockMessages;
  });

  // メッセージが更新されたときにローカルストレージに保存
  useEffect(() => {
    if (messages.length > 0) {
      localStorage.setItem(`chat_messages_${roomId}`, JSON.stringify(messages));
    }
  }, [messages, roomId]);

  // 新しい部屋が作成されたときのチャットログリセット
  useEffect(() => {
    if (roomInfo?.status === "Open") {
      localStorage.removeItem(`chat_messages_${roomId}`);
      setMessages([
        {
          id: Date.now().toString(),
          sender: "System",
          message: "New room created",
          timestamp: new Date().toISOString(),
          type: "system",
        },
      ]);
    }
  }, [roomInfo?.status, roomId]);

  const addMessage = (message: ChatMessage) => {
    setMessages(prev => [...prev, message]);
  };

  const resetMessages = () => {
    localStorage.removeItem(`chat_messages_${roomId}`);
    setMessages([
      {
        id: Date.now().toString(),
        sender: "System",
        message: "Game has started",
        timestamp: new Date().toISOString(),
        type: "system",
      },
    ]);
  };

  return { messages, setMessages, addMessage, resetMessages };
};
