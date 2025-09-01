"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useAuth } from "./contexts/AuthContext";
import { Clock, Moon, PlayCircle, Plus, Users } from "lucide-react";
import { toast } from "react-hot-toast";

// interface Village {
//     room_id: number;
//     name: string;
//     lobby_id: string;
//     players?: {id: number}[] | null; // playersの型も必要に応じて修正
//     max_players: number; // max_playersプロパティを追加
//     // その他のプロパティ...
//   }

export interface Room {
  id: string;
  name: string;
  players: number;
  maxPlayers: number;
  status: "waiting" | "playing" | "finished";
  createdAt: string;
}

const mockRooms: Room[] = [
  {
    id: "1",
    name: "初心者歓迎！",
    players: 5,
    maxPlayers: 8,
    status: "waiting",
    createdAt: new Date().toISOString(),
  },
  {
    id: "2",
    name: "経験者のみ",
    players: 8,
    maxPlayers: 8,
    status: "playing",
    createdAt: new Date().toISOString(),
  },
];

interface Village {
  room_id: string;
  name: string;
  lobby_id: string;
  players?: Player[] | null;
  max_players: number;
  status: "Open" | "InProgress" | "Closed"; // RoomStatus の Enum

  roles: string[]; // 役職一覧
  voting_status: "not_started" | "in_progress" | "completed"; // VotingStatus の Enum
  votes: Record<number, Vote>; // key: target_player_id, value: Vote
}

interface Player {
  id: number;
  name: string;
  role?: string; // 役職が未確定の時もあるので optional
  is_alive: boolean; // 生存状態
}

interface Vote {
  voter_id: number;
  target_id: number;
}

const Home = () => {
  const [villages, setVillages] = useState<Village[]>([]);
  const [newVillageName, setNewVillageName] = useState("");
  const [error, setError] = useState("");
  const [rooms] = useState<Room[]>(mockRooms);
  const [isCreating, setIsCreating] = useState(false);
  const [notification, setNotification] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const { isAuthenticated, user } = useAuth();

  useEffect(() => {
    const fetchVillages = async () => {
      try {
        const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/room/rooms`);
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setVillages(data);
      } catch (error) {
        console.error("Error fetching villages:", error);
        setError("Failed to fetch villages.");
      }
    };

    fetchVillages();
  }, []);

  const createRoom = async () => {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/room/create`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ name: newVillageName }),
      });

      if (!response.ok) {
        throw new Error("Failed to create room");
      }

      setNotification({
        message: "Room created successfully",
        type: "success",
      });

      setIsCreating(false);
    } catch (error) {
      console.error("Error creating room:", error);
      setNotification({
        message: "Failed to create room",
        type: "error",
      });
    }
  };

  const joinRoom = async (roomId: string) => {
    if (!isAuthenticated || !user) {
      toast.error("ルームに参加するにはログインが必要です", {
        duration: 4000,
        position: "top-center",
      });
      return;
    }

    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/room/${roomId}/join/${user.id}`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${localStorage.getItem("token")}`,
        },
      });

      if (!response.ok) {
        throw new Error("ルームへの参加に失敗しました");
      }

      setNotification({
        message: `${user.username}として参加しました`,
        type: "success",
      });

      // 成功したら/room/{roomId}に遷移
      window.location.href = `/room/${roomId}`;
    } catch (error) {
      console.error("Error joining room:", error);
      setNotification({
        message: "ルームへの参加に失敗しました",
        type: "error",
      });
    }
  };

  return (
    <div className="container mx-auto p-4">
      {notification && (
        <div
          className={`fixed z-50 top-4 right-4 p-4 rounded-lg shadow-lg ${
            notification.type === "success" ? "bg-green-500" : "bg-red-500"
          } text-white`}
        >
          {notification.message}
        </div>
      )}

      <div className="flex justify-between items-center mb-8">
        <div className="flex items-center gap-4">
          <Moon size={32} className="text-indigo-600" />
          <h1 className="text-3xl font-bold text-indigo-900">人狼ゲーム</h1>
        </div>
        <div className="flex items-center gap-4">
          <input
            type="text"
            value={newVillageName}
            onChange={e => setNewVillageName(e.target.value)}
            placeholder="Enter room name"
            className="px-4 py-2 border rounded-lg"
          />
          <button
            onClick={createRoom}
            className="bg-indigo-600 text-white px-6 py-3 rounded-lg flex items-center gap-2 hover:bg-indigo-700 transition-colors shadow-sm"
          >
            <Plus size={20} />
            ルーム作成
          </button>
        </div>
      </div>

      <div className="grid gap-4">
        {Object.entries(villages).map(([key, room]) => (
          <div
            key={room.room_id}
            className="bg-white/80 backdrop-blur-sm rounded-xl shadow-sm hover:shadow-md transition-all border border-indigo-50 p-6"
          >
            <div className="flex justify-between items-center">
              <h2 className="text-xl font-semibold text-indigo-900">
                {room.name ? room.name : `ルーム${room.room_id}`}
              </h2>
              <span
                className={`px-4 py-1.5 rounded-full text-sm font-medium ${
                  room.status === "Open" ? "bg-green-50 text-green-700" : "bg-amber-50 text-amber-700"
                }`}
              >
                {room.status === "Open" ? "待機中" : "ゲーム中"}
              </span>
            </div>

            <div className="mt-4 flex items-center gap-6 text-indigo-700">
              <div className="flex items-center gap-2">
                <Users size={18} />
                <span>
                  {room.players?.length ?? 0}/{room.max_players}人
                </span>
              </div>
              <div className="flex items-center gap-2">
                <Clock size={18} />
                <span>作成: {/*room.createdAt> :*/ "不明"}</span>
              </div>
            </div>

            <button
              onClick={() => joinRoom(room.room_id)}
              disabled={room.status !== "Open"}
              className={`mt-6 w-full py-3 rounded-lg flex items-center justify-center gap-2 transition-colors ${
                room.status === "Open"
                  ? "bg-indigo-600 text-white hover:bg-indigo-700 shadow-sm"
                  : "bg-gray-100 text-gray-400 cursor-not-allowed"
              }`}
            >
              <PlayCircle size={20} />
              {room.status === "Open" ? "参加する" : "ゲーム中"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Home;
