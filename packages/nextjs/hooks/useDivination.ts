import { useCallback, useState } from "react";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { DivinationInput } from "~~/utils/crypto/type";

export const useDivination = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [proofId, setProofId] = useState<string | null>(null);
  const [proofStatus, setProofStatus] = useState<"pending" | "completed" | "failed" | null>(null);

  // MPCノードの公開鍵を環境変数から取得
  const mpcPublicKeys = [
    process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY,
    process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY,
    process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY,
  ].filter((key): key is string => key != null);

  const submitDivination = useCallback(
    async (roomId: string, divinationData: DivinationInput, alivePlayerCount: number) => {
      setIsLoading(true);
      setError(null);
      try {
        if (mpcPublicKeys.length !== 3) {
          throw new Error("MPC node public keys are not properly configured");
        }

        // 占いデータの暗号化（MPCノードの公開鍵を使用）
        const encryptedDivination = await MPCEncryption.encryptDivination(divinationData);

        // 占い証明のリクエスト送信
        const newProofId = await fetch(`http://localhost:8080/api/game/${roomId}/proof`, {
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

        if (!newProofId.ok) {
          const errorData = await newProofId.json();
          console.error("Error message:", errorData);
          throw new Error("占いデータの送信に失敗しました");
        }

        console.log("proof request is accepted. batch_id is ", await newProofId.json());

        setProofStatus("pending");

        return newProofId;
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error occurred");
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [],
  );

  return {
    submitDivination,
    isLoading,
    error,
    proofId,
    proofStatus,
  };
};
