"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useAuth } from "./contexts/AuthContext";
import { Clock, Moon, PlayCircle, Plus, Users, X } from "lucide-react";
import { toast } from "react-hot-toast";

interface Village {
  room_id: string;
  name: string;
  lobby_id: string;
  players?: Player[] | null;
  max_players: number;
  status: "Open" | "InProgress" | "Closed";
  roles: string[];
  voting_status: "not_started" | "in_progress" | "completed";
  votes: Record<number, Vote>;
}

interface Player {
  id: number;
  name: string;
  role?: string;
  is_alive: boolean;
}

interface Vote {
  voter_id: number;
  target_id: number;
}

type RoleConfig = {
  Seer: number;
  Werewolf: number;
  Villager: number;
};

type TimeConfig = {
  day_phase: number;
  night_phase: number;
  voting_phase: number;
};

const Home = () => {
  const [villages, setVillages] = useState<Village[]>([]);
  const [newVillageName, setNewVillageName] = useState("");
  const [error, setError] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [maxPlayers, setMaxPlayers] = useState(8);
  const [seerCount, setSeerCount] = useState(1);
  const [werewolfCount, setWerewolfCount] = useState(2);
  const [dayPhase, setDayPhase] = useState(300);
  const [nightPhase, setNightPhase] = useState(120);
  const [votingPhase, setVotingPhase] = useState(90);
  const { isAuthenticated, user } = useAuth();

  const fetchVillages = useCallback(async () => {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/room/rooms`);
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      setVillages(data);
    } catch (fetchError) {
      console.error("Error fetching villages:", fetchError);
      setError("Failed to fetch villages.");
    }
  }, []);

  useEffect(() => {
    fetchVillages();
  }, [fetchVillages]);

  useEffect(() => {
    const defaultWerewolf = Math.min(5, Math.max(1, Math.ceil(maxPlayers * 0.25)));
    setWerewolfCount(defaultWerewolf);
  }, [maxPlayers]);

  const villagerCount = maxPlayers - seerCount - werewolfCount;
  const roleConfig: RoleConfig = useMemo(
    () => ({
      Seer: seerCount,
      Werewolf: werewolfCount,
      Villager: Math.max(0, villagerCount),
    }),
    [seerCount, werewolfCount, villagerCount],
  );
  const timeConfig: TimeConfig = useMemo(
    () => ({
      day_phase: dayPhase,
      night_phase: nightPhase,
      voting_phase: votingPhase,
    }),
    [dayPhase, nightPhase, votingPhase],
  );

  const hasRoleConfigError = villagerCount < 0;
  const isWerewolfDangerous = werewolfCount * 2 >= maxPlayers;
  const canCreateRoom = newVillageName.trim().length > 0 && !hasRoleConfigError;

  const resetCreateRoomForm = () => {
    setNewVillageName("");
    setMaxPlayers(8);
    setSeerCount(1);
    setWerewolfCount(2);
    setDayPhase(300);
    setNightPhase(120);
    setVotingPhase(90);
  };

  const createRoom = async () => {
    if (!canCreateRoom) {
      toast.error("Please check room settings.");
      return;
    }

    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/room/create`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          name: newVillageName,
          max_players: maxPlayers,
          role_config: roleConfig,
          time_config: timeConfig,
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to create room");
      }

      toast.success("Room created successfully");
      setIsCreating(false);
      resetCreateRoomForm();
      await fetchVillages();
    } catch (createError) {
      console.error("Error creating room:", createError);
      toast.error("Failed to create room");
    }
  };

  const joinRoom = async (roomId: string) => {
    if (!isAuthenticated || !user) {
      toast.error("You need to be logged in to join a room", {
        duration: 4000,
        position: "top-center",
      });
      return;
    }

    try {
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/room/${roomId}/join/${user.id}`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${localStorage.getItem("token")}`,
          },
        },
      );

      if (!response.ok) {
        throw new Error("Failed to join the room");
      }

      toast.success(`joined the room as ${user.username}`);
      window.location.href = `/room/${roomId}`;
    } catch (joinError) {
      console.error("Error joining room:", joinError);
      toast.error("Failed to join the room");
    }
  };

  return (
    <div className="container mx-auto p-4">
      <div className="flex justify-between items-center mb-8">
        <div className="flex items-center gap-4">
          <Moon size={32} className="text-indigo-600" />
          <h1 className="text-3xl font-bold text-indigo-900">Werewolf Game</h1>
        </div>
        <button
          onClick={() => setIsCreating(true)}
          className="bg-indigo-600 text-white px-6 py-3 rounded-lg flex items-center gap-2 hover:bg-indigo-700 transition-colors shadow-sm"
        >
          <Plus size={20} />
          Create Room
        </button>
      </div>

      {error && <div className="mb-4 text-red-600">{error}</div>}

      <div className="grid gap-4">
        {Object.entries(villages).map(([key, room]) => (
          <div
            key={`${room.room_id}-${key}`}
            className="bg-white/80 backdrop-blur-sm rounded-xl shadow-sm hover:shadow-md transition-all border border-indigo-50 p-6"
          >
            <div className="flex justify-between items-center">
              <h2 className="text-xl font-semibold text-indigo-900">
                {room.name ? room.name : `Room ${room.room_id}`}
              </h2>
              <span
                className={`px-4 py-1.5 rounded-full text-sm font-medium ${
                  room.status === "Open" ? "bg-green-50 text-green-700" : "bg-amber-50 text-amber-700"
                }`}
              >
                {room.status === "Open" ? "Waiting" : "In Game"}
              </span>
            </div>

            <div className="mt-4 flex items-center gap-6 text-indigo-700">
              <div className="flex items-center gap-2">
                <Users size={18} />
                <span>
                  {room.players?.length ?? 0}/{room.max_players} players
                </span>
              </div>
              <div className="flex items-center gap-2">
                <Clock size={18} />
                <span>Created: Unknown</span>
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
              {room.status === "Open" ? "Join" : "In Game"}
            </button>
          </div>
        ))}
      </div>

      {isCreating && (
        <div className="fixed inset-0 z-50 bg-black/45 flex items-center justify-center px-4">
          <div className="w-full max-w-3xl bg-white rounded-2xl shadow-xl border border-indigo-100 overflow-hidden">
            <div className="px-6 py-4 border-b border-indigo-100 flex items-center justify-between">
              <h2 className="text-xl font-semibold text-indigo-900">Create Room</h2>
              <button
                onClick={() => setIsCreating(false)}
                className="p-2 rounded-lg hover:bg-gray-100 text-gray-600 transition-colors"
              >
                <X size={18} />
              </button>
            </div>

            <div className="grid md:grid-cols-2 gap-6 p-6">
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-indigo-900 mb-2">Room Name</label>
                  <input
                    type="text"
                    value={newVillageName}
                    onChange={e => setNewVillageName(e.target.value)}
                    placeholder="Enter room name"
                    className="w-full px-4 py-2 border border-indigo-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-indigo-900 mb-2">Players: {maxPlayers}</label>
                  <input
                    type="range"
                    min={4}
                    max={20}
                    value={maxPlayers}
                    onChange={e => setMaxPlayers(Number(e.target.value))}
                    className="w-full"
                  />
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-sm font-medium text-indigo-900 mb-2">Seer</label>
                    <input
                      type="number"
                      min={0}
                      max={2}
                      value={seerCount}
                      onChange={e => setSeerCount(Math.max(0, Math.min(2, Number(e.target.value))))}
                      className="w-full px-3 py-2 border border-indigo-200 rounded-lg"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-indigo-900 mb-2">Werewolf</label>
                    <input
                      type="number"
                      min={1}
                      max={5}
                      value={werewolfCount}
                      onChange={e => setWerewolfCount(Math.max(1, Math.min(5, Number(e.target.value))))}
                      className="w-full px-3 py-2 border border-indigo-200 rounded-lg"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-3 gap-3">
                  <div>
                    <label className="block text-xs font-medium text-indigo-900 mb-2">Day (sec)</label>
                    <input
                      type="number"
                      min={60}
                      max={600}
                      value={dayPhase}
                      onChange={e => setDayPhase(Math.max(60, Math.min(600, Number(e.target.value))))}
                      className="w-full px-3 py-2 border border-indigo-200 rounded-lg"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium text-indigo-900 mb-2">Night (sec)</label>
                    <input
                      type="number"
                      min={30}
                      max={300}
                      value={nightPhase}
                      onChange={e => setNightPhase(Math.max(30, Math.min(300, Number(e.target.value))))}
                      className="w-full px-3 py-2 border border-indigo-200 rounded-lg"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium text-indigo-900 mb-2">Voting (sec)</label>
                    <input
                      type="number"
                      min={30}
                      max={180}
                      value={votingPhase}
                      onChange={e => setVotingPhase(Math.max(30, Math.min(180, Number(e.target.value))))}
                      className="w-full px-3 py-2 border border-indigo-200 rounded-lg"
                    />
                  </div>
                </div>
              </div>

              <div className="bg-indigo-50 rounded-xl p-4 border border-indigo-100">
                <h3 className="text-sm font-semibold text-indigo-900 mb-3">Configuration Preview</h3>

                <div className="text-sm text-indigo-900 space-y-2">
                  <div className="flex justify-between">
                    <span>Max Players</span>
                    <span className="font-semibold">{maxPlayers}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Seer</span>
                    <span className="font-semibold">{roleConfig.Seer}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Werewolf</span>
                    <span className="font-semibold">{roleConfig.Werewolf}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Villager</span>
                    <span className="font-semibold">{roleConfig.Villager}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>Day/Night/Voting</span>
                    <span className="font-semibold">
                      {dayPhase}s / {nightPhase}s / {votingPhase}s
                    </span>
                  </div>
                </div>

                {hasRoleConfigError && (
                  <div className="mt-4 text-sm text-red-700 bg-red-100 border border-red-200 rounded-lg p-3">
                    Invalid role config: total roles exceeds max players.
                  </div>
                )}
                {isWerewolfDangerous && !hasRoleConfigError && (
                  <div className="mt-4 text-sm text-amber-700 bg-amber-100 border border-amber-200 rounded-lg p-3">
                    Warning: werewolf count is high for this room size.
                  </div>
                )}
              </div>
            </div>

            <div className="px-6 py-4 border-t border-indigo-100 flex items-center justify-end gap-3">
              <button
                onClick={() => setIsCreating(false)}
                className="px-4 py-2 rounded-lg border border-gray-300 text-gray-700 hover:bg-gray-50 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={createRoom}
                disabled={!canCreateRoom}
                className={`px-5 py-2 rounded-lg text-white transition-colors ${
                  canCreateRoom ? "bg-indigo-600 hover:bg-indigo-700" : "bg-gray-400 cursor-not-allowed"
                }`}
              >
                Create Room
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default Home;
