import { useState } from "react";
import type { ChatMessage, RoomInfo } from "~~/types/game";

export const useGameChat = (roomId: string, roomInfo: RoomInfo | null) => {
  // サーバーから取得したメッセージのみを使用（localStorageは使用しない）
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  const addMessage = (message: ChatMessage) => {
    setMessages(prev => [...prev, message]);
  };

  const resetMessages = () => {
    // サーバーがログの唯一の真実の源なので、フロント側では空にするだけ
    setMessages([]);
  };

  return { messages, setMessages, addMessage, resetMessages };
};
