// Room Status
export type RoomStatus = "Open" | "InProgress" | "Closed";

// Game Phase
export type GamePhase = "Waiting" | "Night" | "Discussion" | "Voting" | "Result" | "Finished";

export interface Room {
  id: string;
  name: string;
  players: number;
  maxPlayers: number;
  status: RoomStatus;
  createdAt: string;
}

export interface Game {
  roomId: string;
  phase: GamePhase;
  roles: Record<string, string>; // { 'playerId': 'role' }
  votes: Record<string, string[]>; // { 'targetId': ['voterId1', 'voterId2'] }
  result?: "VillagerWin" | "WerewolfWin";
}

export type NightAction =
  | { Attack: { target_id: string } }
  | { Divine: { target_id: string } }
  | { Guard: { target_id: string } };

export interface NightActionRequest {
  player_id: string;
  action: NightAction;
}
