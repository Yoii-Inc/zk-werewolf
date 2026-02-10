import React, { useState } from "react";
import type { Player, Role } from "../../app/types";
import { useGameInputGenerator } from "../../hooks/useGameInputGenerator";
import { useBackgroundNightAction } from "~~/hooks/useBackgroundNightAction";
import { useDivination } from "~~/hooks/useDivination";
import type { GameInfo } from "~~/types/game";

interface NightActionModalProps {
  players: Player[];
  role: Role;
  gameInfo: GameInfo;
  username: string;
  onSubmit: (targetPlayerId: string) => void;
  onClose: () => void;
  roomId: string;
  myId: string;
}

const NightActionModal: React.FC<NightActionModalProps> = ({
  players,
  role,
  gameInfo,
  username,
  onSubmit,
  onClose,
  roomId,
  myId,
}) => {
  const [selectedPlayer, setSelectedPlayer] = useState<string>("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { submitDivination, error, proofStatus } = useDivination();
  const { handleBackgroundNightAction } = useBackgroundNightAction();
  const { isReady, generateDivinationInput } = useGameInputGenerator(roomId, username, gameInfo);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedPlayer) return;

    setIsSubmitting(true);

    try {
      // 占い師の場合は占い処理を行う
      if (role === "Seer") {
        if (!isReady) {
          throw new Error("Game crypto is not ready");
        }

        // 占い対象をlocalStorageに保存（結果受信時に使用）
        localStorage.setItem(`divination_target_${roomId}`, selectedPlayer);
        const targetPlayerName = players.find(p => p.id === selectedPlayer)?.name || "Unknown";
        localStorage.setItem(`divination_target_name_${roomId}`, targetPlayerName);
        console.log("Divination target saved to localStorage:", selectedPlayer, targetPlayerName);

        // 占いデータを生成
        const divinationData = await generateDivinationInput(selectedPlayer, false);

        console.log("占いデータ:", divinationData);

        if (!divinationData) {
          throw new Error("Failed to generate divination data");
        }

        const alivePlayerCount = players.filter(player => !player.is_dead).length;

        console.log("Executing divination.");
        await submitDivination(roomId, divinationData, alivePlayerCount);
      }
      // 人狼の場合は襲撃処理を行う
      else if (role === "Werewolf") {
        console.log("Executing werewolf attack:", selectedPlayer);

        // サーバーに襲撃リクエストを送信
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/actions/night-action`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify({
              player_id: myId,
              action: {
                Attack: {
                  target_id: selectedPlayer,
                },
              },
            }),
          },
        );

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`Failed to submit attack: ${errorText}`);
        }

        console.log("Werewolf attack submitted successfully");
      }

      // 親コンポーネントのonSubmit関数を呼び出す
      await onSubmit(selectedPlayer);
      onClose();
    } catch (err) {
      console.error(`${role === "Seer" ? "Divination" : "Night action"} failed:`, err);
    } finally {
      setIsSubmitting(false);
    }
  }; // Filter selectable players based on role
  const selectablePlayers = players.filter(p => {
    if (p.is_dead === true) return false;
    if (p.id === myId) return false; // 自分自身は選択できない
    // Note: Werewolf同士の識別情報はサーバーが保持していないため、
    // 将来的にMPC計算結果として共有する必要がある
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
            disabled={!selectedPlayer || isSubmitting}
            className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
          >
            {isSubmitting ? "Processing..." : "Confirm"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default NightActionModal;
