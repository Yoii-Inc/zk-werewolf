import { useCallback, useState } from "react";
import { KeyManager } from "../utils/crypto/keyManager";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { AnonymousVotingInput } from "~~/utils/crypto/type";

export const useVoting = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [proofId, setProofId] = useState<string | null>(null);
  const [proofStatus, setProofStatus] = useState<"pending" | "completed" | "failed" | null>(null);

  const keyManager = new KeyManager();

  // MPCノードの公開鍵を環境変数から取得
  const mpcPublicKeys = [
    process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY,
    process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY,
    process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY,
  ].filter((key): key is string => key != null);

  const submitVote = useCallback(async (roomId: string, voteData: AnonymousVotingInput, alivePlayerCount: number) => {
    setIsLoading(true);
    setError(null);
    try {
      if (mpcPublicKeys.length !== 3) {
        throw new Error("MPC node public keys are not properly configured");
      }

      // 投票データの暗号化（MPCノードの公開鍵を使用）
      const encryptedVote = await MPCEncryption.encryptAnonymousVoting(voteData);

      // 投票証明のリクエスト送信
      const newProofId = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/proof`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            proof_type: "AnonymousVoting",
            data: {
              user_id: String(voteData.privateInput.id),
              prover_count: alivePlayerCount,
              encrypted_data: encryptedVote,
            },
          }),
        },
      );

      if (!newProofId.ok) {
        const errorData = await newProofId.json();
        console.error("Error message:", errorData);
        throw new Error("Failed to send vote");
      }

      console.log("proof request is accepted. batch_id is ", await newProofId.json());

      //   const response = await setProofId(newProofId);
      setProofStatus("pending");

      // 証明の状態を監視
      //   const checkStatus = async () => {
      //     try {
      //       const status = await voteApi.checkProofStatus(newProofId);
      //       setProofStatus(status.status);
      //       if (status.status !== "pending") {
      //         clearInterval(intervalId);
      //       }
      //       if (status.error) {
      //         setError(status.error);
      //       }
      //     } catch (err) {
      //       console.error("Failed to check proof status:", err);
      //     }
      //   };

      //   const intervalId = setInterval(checkStatus, 5000);

      return newProofId;
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error occurred");
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  return {
    submitVote,
    isLoading,
    error,
    proofId,
    proofStatus,
  };
};
