import { useCallback } from "react";
import { Player } from "../app/types";
import JSONbig from "json-bigint";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { DivinationInput, DivinationPublicInput, NodeKey, SecretSharingScheme } from "~~/utils/crypto/type";

export const useBackgroundNightAction = () => {
  const JSONbigNative = JSONbig({ useNativeBigInt: true });

  const handleBackgroundNightAction = useCallback(async (roomId: string, myId: string, players: Player[]) => {
    try {
      // パラメータの取得
      const res = await fetch("/pedersen_params2.json");
      const params = await res.text();
      const parsedParams = JSONbigNative.parse(params);

      const randres = await fetch("/pedersen_randomness_0.json");
      const randomness = await randres.text();
      const parsedRandomness = JSONbigNative.parse(randomness);

      const commitres = await fetch("/pedersen_commitment_0.json");
      const commitment = await commitres.text();
      const parsedCommitment = JSONbigNative.parse(commitment);

      const elgamalparamres = await fetch("/elgamal_params.json");
      const elgamalparam = await elgamalparamres.text();
      const parsedElgamalParam = JSONbigNative.parse(elgamalparam);

      const elgamalpubkeyres = await fetch("/elgamal_public_key.json");
      const elgamalpubkey = await elgamalpubkeyres.text();
      const parsedElgamalPubkey = JSONbigNative.parse(elgamalpubkey);

      // ダミーのプライベート入力を作成
      // Determine whether the current player (by myId) is a Werewolf using the players array
      const amWerewolfValues =
        players.find(player => player.id === myId)?.role === "Werewolf"
          ? JSONbigNative.parse(
              '["9015221291577245683", "8239323489949974514", "1646089257421115374", "958099254763297437"]',
            )
          : JSONbigNative.parse('["0", "0", "0", "0"]');

      const privateInput = {
        id: players.findIndex(player => player.id === myId),
        isTarget: players.map(() => [JSONbigNative.parse('["0","0","0","0"]'), null]),
        isWerewolf: [amWerewolfValues, null],
        randomness: parsedRandomness,
      };

      const publicInput: DivinationPublicInput = {
        pedersenParam: parsedParams,
        // playerCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
        elgamalParam: parsedElgamalParam,
        pubKey: parsedElgamalPubkey,
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

      const divinationData: DivinationInput = {
        privateInput,
        publicInput,
        nodeKeys,
        scheme,
      };

      // 暗号化
      const encryptedDivination = await MPCEncryption.encryptDivination(divinationData);

      const alivePlayerCount = players.filter(player => !player.is_dead).length;

      // バックエンドにリクエスト送信
      const response = await fetch(`http://localhost:8080/api/game/${roomId}/proof`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          proof_type: "Divination",
          data: {
            user_id: String(divinationData.privateInput.id),
            prover_count: alivePlayerCount,
            encrypted_data: encryptedDivination,
          },
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to send night action");
      }
    } catch (error) {
      console.error("Background night action error:", error);
    }
  }, []);

  return { handleBackgroundNightAction };
};
