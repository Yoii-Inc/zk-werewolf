import { Role } from "~~/app/types";
import {
  ElGamalParam,
  ElGamalPublicKey,
  ElGamalSecretKey,
  Field,
  PedersenCommitment,
  PedersenParam,
} from "~~/utils/crypto/type";

export interface RoomInfo {
  room_id: string;
  name: string;
  status: "Open" | "InProgress" | "Closed";
  max_players: number;
  currentPlayers: number;
  remainingTime: number;
  players: Player[];
}

// 暗号パラメータの型定義（サーバー側のCryptoParametersに対応）
export interface CryptoParameters {
  pedersen_param: PedersenParam; // Pedersenコミットメントパラメータ
  player_commitment: PedersenCommitment[]; // プレイヤーのコミットメント配列
  fortune_teller_public_key: ElGamalPublicKey; // 占い師の公開鍵
  elgamal_param: ElGamalParam; // ElGamal暗号化パラメータ
  player_randomness: Field[]; // プレイヤーのランダムネス配列(本来は含めるべきでない)
  secret_key: ElGamalSecretKey; // 秘密鍵（注意: 本来は含めるべきでない）
}

export interface GameInfo {
  room_id: string;
  phase: "Waiting" | "Night" | "Discussion" | "Voting" | "Result" | "Finished";
  players: Player[];
  playerRole: Role;
  hasActed: boolean;
  result: "InProgress" | "VillagerWin" | "WerewolfWin";
  crypto_parameters?: CryptoParameters;
  chat_log?: {
    messages: Array<{
      id: any;
      player_name: any;
      content: any;
      timestamp: any;
      message_type: string;
    }>;
  };
  grouping_parameter?: GroupingParameter;
}

export interface PrivateGameInfo {
  playerId: string;
  playerRole: Role;
  hasActed: boolean; // アクションを実行済みか
}

export interface Player {
  id: string;
  name: string;
  role: Role;
  is_dead: boolean;
  isReady: boolean;
}

export interface ChatMessage {
  id: string;
  sender: string;
  message: string;
  timestamp: string;
  type: "system" | "normal" | "whisper";
  source?: "server" | "client"; // メッセージの送信元（オプショナル）
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

export interface GroupingParameter {
  Villager: [number, boolean];
  FortuneTeller: [number, boolean];
  Werewolf: [number, boolean];
}
