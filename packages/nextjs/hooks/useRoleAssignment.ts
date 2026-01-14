import { useCallback, useState } from "react";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { RoleAssignmentInput, RoleAssignmentOutput } from "~~/utils/crypto/type";

export const useRoleAssignment = () => {
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

  const submitRoleAssignment = useCallback(
    async (roomId: string, roleAssignmentData: RoleAssignmentInput, alivePlayerCount: number) => {
      setIsLoading(true);
      setError(null);
      try {
        if (mpcPublicKeys.length !== 3) {
          throw new Error("MPC node public keys are not properly configured");
        }

        console.log(roleAssignmentData);

        // Encrypt role assignment data (using MPC node public key)
        const encryptedRoleAssignment: RoleAssignmentOutput =
          await MPCEncryption.encryptRoleAssignment(roleAssignmentData);

        console.log("Sending role assignment request.");

        const requestBody = {
          proof_type: "RoleAssignment",
          data: {
            user_id: String(roleAssignmentData.privateInput.id),
            prover_count: alivePlayerCount,
            encrypted_data: encryptedRoleAssignment,
            public_key: roleAssignmentData.publicKey, // プレイヤーの公開鍵を追加
          },
        };

        console.log(requestBody);

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
    submitRoleAssignment,
    isLoading,
    error,
    proofId,
    proofStatus,
  };
};
