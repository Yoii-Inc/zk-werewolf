export type ChatMessage = {
  id: string;
  sender: string;
  message: string;
  timestamp: string;
  type: "system" | "whisper" | "normal";
};

export type Player = {
  id: string;
  name: string;
  status: "alive" | "dead";
  isReady: boolean;
};
