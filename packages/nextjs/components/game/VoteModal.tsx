import React from "react";
import { useState } from "react";
import type { Player } from "../../app/types";
import { useVoting } from "../../hooks/useVoting";

interface VoteModalProps {
  players: Player[];
  roomId: string;
  onSubmit: (targetId: string) => void;
  onClose: () => void;
}

const VoteModal: React.FC<VoteModalProps> = ({ players, roomId, onSubmit, onClose }) => {
  const [selectedPlayerId, setSelectedPlayerId] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { submitVote, error, proofStatus } = useVoting();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedPlayerId) return;

    setIsSubmitting(true);
    try {
      // データ型は要修正
      await submitVote(roomId, {
        targetId: selectedPlayerId,
        timestamp: Date.now(),
      });
      await onSubmit(selectedPlayerId);
      onClose();
    } catch (err) {
      console.error("投票に失敗しました:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-6 max-w-md w-full mx-4">
        <h2 className="text-xl font-bold mb-4 text-gray-900">投票</h2>
        <p className="mb-4 text-gray-600">処刑する対象を選択してください</p>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid gap-3">
            {players
              .filter(player => !player.is_dead) // 生存プレイヤーのみ表示
              .map(player => (
                <button
                  key={player.id}
                  type="button"
                  onClick={() => setSelectedPlayerId(player.id)}
                  className={`p-3 text-left rounded-lg transition-colors ${
                    selectedPlayerId === player.id
                      ? "bg-indigo-600 text-white"
                      : "bg-gray-100 hover:bg-gray-200 text-gray-900"
                  }`}
                >
                  {player.name}
                </button>
              ))}
          </div>

          {error && <div className="p-3 bg-red-100 border border-red-400 text-red-700 rounded">{error}</div>}

          <div className="flex gap-3 mt-6">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
            >
              キャンセル
            </button>
            <button
              type="submit"
              disabled={!selectedPlayerId || isSubmitting}
              className="flex-1 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors disabled:opacity-50"
            >
              {isSubmitting ? "投票中..." : "投票する"}
            </button>
          </div>
        </form>

        {proofStatus && (
          <div className="mt-4 p-3 bg-gray-100 rounded text-sm text-gray-600">
            <p>証明状態: {proofStatus}</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default VoteModal;
