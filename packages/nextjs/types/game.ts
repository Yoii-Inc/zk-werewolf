export interface RoomInfo {
  room_id: string;
  name: string;
  status: "Open" | "InProgress" | "Closed";
  max_players: number;
  currentPlayers: number;
  remainingTime: number;
  players: Player[];
}

export interface GameInfo {
  room_id: string;
  phase: "Waiting" | "Night" | "Discussion" | "Voting" | "Result" | "Finished";
  players: Player[];
  playerRole: "占い師" | "人狼" | "村人";
  hasActed: boolean;
  result: "InProgress" | "VillagerWin" | "WerewolfWin";
  chat_log?: {
    messages: Array<{
      id: any;
      player_name: any;
      content: any;
      timestamp: any;
      message_type: string;
    }>;
  };
}

export interface PrivateGameInfo {
  playerId: string;
  playerRole: "占い師" | "人狼" | "村人";
  hasActed: boolean; // アクションを実行済みか
}

export interface Player {
  id: string;
  name: string;
  role: "Seer" | "Werewolf" | "Villager";
  is_dead: boolean;
  isReady: boolean;
}

export interface ChatMessage {
  id: string;
  sender: string;
  message: string;
  timestamp: string;
  type: "system" | "normal" | "whisper";
}

export interface WebSocketMessage {
  message_type: string;
  player_id: string;
  player_name: string;
  content: string;
  timestamp: string;
  room_id: string;
}

export interface GameResultModalProps {
  result: "VillagerWin" | "WerewolfWin" | "InProgress";
  onClose: () => void;
}
