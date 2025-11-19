import React, { useState } from "react";
import type { Player } from "../../app/types";
import JSONbig from "json-bigint";
import { useBackgroundNightAction } from "~~/hooks/useBackgroundNightAction";
import { useDivination } from "~~/hooks/useDivination";
import { DivinationInput, DivinationPublicInput, NodeKey, SecretSharingScheme } from "~~/utils/crypto/type";

interface NightActionModalProps {
  players: Player[];
  role: "Seer" | "Werewolf" | "Villager";
  onSubmit: (targetPlayerId: string) => void;
  onClose: () => void;
  roomId: string;
  myId: string;
}

const NightActionModal: React.FC<NightActionModalProps> = ({ players, role, onSubmit, onClose, roomId, myId }) => {
  const [selectedPlayer, setSelectedPlayer] = useState<string>("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { submitDivination, error, proofStatus } = useDivination();
  const { handleBackgroundNightAction } = useBackgroundNightAction();

  const JSONbigNative = JSONbig({ useNativeBigInt: true });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedPlayer) return;

    setIsSubmitting(true);

    try {
      // もし占い師の場合は、占い処理を行う
      if (role === "Seer") {
        const res = await fetch("/pedersen_params2.json");
        const params = await res.text();
        const parsedParams = JSONbigNative.parse(params);

        const randres = await fetch("/pedersen_randomness_0.json");
        const randomness = await randres.text();
        const parsedRandomness = JSONbigNative.parse(randomness);

        const commitres = await fetch("/pedersen_commitment_0.json");
        const commitment = await commitres.text();
        const parsedCommitment = JSONbigNative.parse(commitment);

        const elgamalparamres = await fetch("/test_elgamal_params.json");
        const elgamalparam = await elgamalparamres.text();
        const parsedElgamalParam = JSONbigNative.parse(elgamalparam);

        const elgamalpubkeyres = await fetch("/test_elgamal_pubkey.json");
        const elgamalpubkey = await elgamalpubkeyres.text();
        const parsedElgamalPubkey = JSONbigNative.parse(elgamalpubkey);

        const privateInput = {
          id: players.findIndex(player => player.id === myId),
          isWerewolf: [[0n, 0n, 0n, 0n], null],
          isTarget: players.map(player => [player.id === selectedPlayer ? [0n, 0n, 0n, 1n] : [0n, 0n, 0n, 0n], null]),
          randomness: parsedRandomness,
        };

        const publicInput: DivinationPublicInput = {
          pedersenParam: parsedParams,
          elgamalParam: parsedElgamalParam,
          pubKey: parsedElgamalPubkey,
          playerCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
          playerNum: players.length,
        };

        const nodeKeys: NodeKey[] = [
          {
            nodeId: "0",
            publicKey: process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "",
          },
          {
            nodeId: "1",
            publicKey: process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "",
          },
          {
            nodeId: "2",
            publicKey: process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "",
          },
        ];

        const scheme: SecretSharingScheme = {
          totalShares: 3,
          modulus: 97,
        };

        const votingData: DivinationInput = {
          privateInput,
          publicInput,
          nodeKeys,
          scheme,
        };

        const alivePlayerCount = players.filter(player => !player.is_dead).length;

        console.log("占いを実行します。");
        await submitDivination(roomId, votingData, alivePlayerCount);
      } else {
        // 占い師以外のプレイヤーの場合、ダミーリクエストを送信
        console.log("ダミーリクエストを送信します。");
        await handleBackgroundNightAction(roomId, myId, players);
      }

      // 親コンポーネントのonSubmit関数を呼び出す
      await onSubmit(selectedPlayer);
      onClose();
    } catch (err) {
      console.error(`${role === "Seer" ? "占い" : "夜の行動"}に失敗しました:`, err);
    } finally {
      setIsSubmitting(false);
    }
  }; // Filter selectable players based on role
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
            disabled={!selectedPlayer || isSubmitting}
            className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
          >
            {isSubmitting ? "処理中..." : "Confirm"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default NightActionModal;
