export type ChatMessage = {
  id: string;
  sender: string;
  message: string;
  timestamp: string;
  type: "system" | "whisper" | "normal";
};

export interface WebSocketMessage {
  message_type: "whisper" | "system" | "normal";
  player_id: string;
  player_name: string;
  content: string;
  timestamp: string;
  room_id: string;
}

export type Player = {
  id: string;
  name: string;
  is_dead: boolean;
  isReady: boolean;
  role: "Villager" | "Werewolf" | "Seer" | "Guard";
};
