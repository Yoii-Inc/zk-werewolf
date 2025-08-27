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
      const res = await fetch("/pedersen-params.json");
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

      // ダミーのプライベート入力を作成
      const privateInput = {
        id: players.findIndex(player => player.id === myId),
        isTargetId: players.map(() => [[0, 0, 0, 0], null]),
        isWerewolf: [[0, 0, 0, 0], null],
        playerRandomness: parsedRandomness,
      };

      const publicInput: DivinationPublicInput = {
        pedersenParam: parsedParams,
        playerCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
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
            prover_count: alivePlayerCount,
            encrypted_data: encryptedDivination,
          },
        }),
      });

      if (!response.ok) {
        throw new Error("夜の行動の送信に失敗しました");
      }
    } catch (error) {
      console.error("バックグラウンド夜行動エラー:", error);
    }
  }, []);

  return { handleBackgroundNightAction };
};
