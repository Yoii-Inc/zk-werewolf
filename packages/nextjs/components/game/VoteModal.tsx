import React from "react";
import type { Player } from "../../app/types";

interface VoteModalProps {
  players: Player[];
  onSubmit: (targetId: string) => void;
  onClose: () => void;
}

const VoteModal: React.FC<VoteModalProps> = ({ players, onSubmit, onClose }) => {
  const livingPlayers = players.filter(player => player.is_dead === false);

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center">
      <div className="bg-white rounded-lg p-6 w-96">
        <h2 className="text-xl font-bold text-gray-900 mb-4">投票</h2>
        <p className="text-gray-600 mb-4">処刑する人を選択してください。</p>

        <div className="space-y-2">
          {livingPlayers.map(player => (
            <button
              key={player.id}
              onClick={() => {
                onSubmit(player.id);
                onClose();
              }}
              className="w-full p-3 text-left hover:bg-gray-100 rounded-lg transition-colors flex items-center gap-2"
            >
              <span className="text-gray-900">{player.name}</span>
            </button>
          ))}
        </div>

        <div className="mt-6 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
          >
            キャンセル
          </button>
        </div>
      </div>
    </div>
  );
};

export default VoteModal;
