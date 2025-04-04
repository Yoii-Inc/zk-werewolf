import React, { useState } from "react";
import type { Player } from "../../app/types";

interface NightActionModalProps {
  players: Player[];
  role: "Seer" | "Werewolf" | "Villager" | "Guard";
  onSubmit: (targetPlayerId: string) => void;
  onClose: () => void;
}

const NightActionModal: React.FC<NightActionModalProps> = ({ players, role, onSubmit, onClose }) => {
  const [selectedPlayer, setSelectedPlayer] = useState<string>("");

  const handleSubmit = () => {
    if (selectedPlayer) {
      onSubmit(selectedPlayer);
    }
  };

  // Filter selectable players based on role
  const selectablePlayers = players.filter(p => {
    if (p.is_dead === true) return false;
    if (role === "Werewolf" && p.role === "Werewolf") return false;
    return true;
  });

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-6 w-96 shadow-xl">
        <h2 className="text-xl font-bold mb-4 text-indigo-900">
          {role === "Seer" ? "Select a target to divine" : role === "Werewolf" ? "Select a target to attack" : ""}
        </h2>
        <div className="space-y-2 mb-6">
          {selectablePlayers.map(player => (
            <div
              key={player.id}
              className="flex items-center gap-2 p-2 hover:bg-indigo-50 rounded-lg cursor-pointer"
              onClick={() => setSelectedPlayer(player.id)}
            >
              <input
                type="radio"
                id={player.id}
                name="target"
                value={player.id}
                checked={selectedPlayer === player.id}
                onChange={() => setSelectedPlayer(player.id)}
                className="form-radio text-indigo-600"
              />
              <label htmlFor={player.id} className="cursor-pointer flex-1">
                {player.name}
              </label>
            </div>
          ))}
        </div>
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="px-4 py-2 bg-gray-200 rounded-lg hover:bg-gray-300 transition-colors">
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={!selectedPlayer}
            className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
};

export default NightActionModal;
