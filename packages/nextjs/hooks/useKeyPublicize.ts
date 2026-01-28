import { useCallback, useState } from "react";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { KeyPublicizeInput, KeyPublicizeOutput } from "~~/utils/crypto/type";

export const useKeyPublicize = () => {
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

  const submitKeyPublicize = useCallback(
    async (roomId: string, keyPublicizeData: KeyPublicizeInput, alivePlayerCount: number) => {
      setIsLoading(true);
      setError(null);
      try {
        if (mpcPublicKeys.length !== 3) {
          throw new Error("MPC node public keys are not properly configured");
        }

        console.log("Submitting key publicize request:", keyPublicizeData);

        // Encrypt key publicize data (using MPC node public key)
        const encryptedKeyPublicize: KeyPublicizeOutput = await MPCEncryption.encryptKeyPublicize(keyPublicizeData);

        console.log("Sending key publicize request");

        const requestBody = {
          proof_type: "KeyPublicize",
          data: {
            user_id: String(keyPublicizeData.privateInput.id),
            prover_count: alivePlayerCount,
            encrypted_data: encryptedKeyPublicize,
          },
        };

        console.log("Request body:", requestBody);

        const newProofId = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/proof`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
            },
            body: JSON.stringify(requestBody),
          },
        );

        if (!newProofId.ok) {
          const errorData = await newProofId.json();
          console.error("Error message:", errorData);
          throw new Error("Failed to send role assignment data");
        }

        console.log("proof request is accepted. batch_id is ", await newProofId.json());

        // setProofId(newProofId);
        setProofStatus("pending");

        return newProofId;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : "Unknown error occurred";
        setError(errorMessage);
        setProofStatus("failed");
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [mpcPublicKeys],
  );

  const checkProofStatus = useCallback(async (roomId: string, proofId: string) => {
    try {
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/rooms/${roomId}/proof/${proofId}/status`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
          },
          credentials: "include",
        },
      );

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      setProofStatus(data.status);

      return data.status;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : "Unknown error occurred";
      setError(errorMessage);
      throw err;
    }
  }, []);

  return {
    submitKeyPublicize,
    checkProofStatus,
    isLoading,
    error,
    proofId,
    proofStatus,
  };
};
