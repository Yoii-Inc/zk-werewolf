import { useCallback, useState } from "react";
import { KeyManager } from "../utils/crypto/keyManager";
import { AnonymousVotingInput, MPCEncryption } from "~~/utils/crypto/InputEncryption";

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

  const submitVote = useCallback(async (roomId: string, voteData: AnonymousVotingInput) => {
    setIsLoading(true);
    setError(null);
    try {
      if (mpcPublicKeys.length !== 3) {
        throw new Error("MPC node public keys are not properly configured");
      }

      // キーペアの生成
      await keyManager.generateKeyPair();
      const publicKey = keyManager.getPublicKey();
      if (!publicKey) throw new Error("Failed to generate key pair");

      voteData.publicKey = mpcPublicKeys;

      // 投票データの暗号化（MPCノードの公開鍵を使用）
      const encryptedVote = await MPCEncryption.encryptAnonymousVoting(voteData);

      // 署名の生成
      const message = JSON.stringify({ encryptedVote, publicKey });
      const signature = await keyManager.sign(message);

      // 投票の送信
      const newProofId = await fetch(`http://localhost:8080/api/game/${roomId}/actions/vote`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(encryptedVote),
      });

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
