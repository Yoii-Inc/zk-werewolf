import type { GameResultModalProps } from "~~/types/game";

export const GameResultModal = ({ result, onClose }: GameResultModalProps) => {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-8 max-w-lg w-full mx-4 text-center">
        <h2 className="text-3xl font-bold mb-4 text-indigo-900">
          {result === "VillagerWin" ? "村人陣営の勝利！" : "人狼陣営の勝利！"}
        </h2>
        <button
          onClick={onClose}
          className="px-6 py-3 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          閉じる
        </button>
      </div>
    </div>
  );
};
