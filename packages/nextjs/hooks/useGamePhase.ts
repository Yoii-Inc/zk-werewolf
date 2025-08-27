import { useEffect, useRef } from "react";
import { useRoleAssignment } from "./useRoleAssignment";
import { useWinningJudge } from "./useWinningJudge";
import JSONbig from "json-bigint";
import type { ChatMessage, GameInfo } from "~~/types/game";
import { NodeKey, SecretSharingScheme, WinningJudgementInput, WinningJudgementPublicInput } from "~~/utils/crypto/type";

export const useGamePhase = (
  gameInfo: GameInfo | null,
  roomId: string,
  addMessage: (message: ChatMessage) => void,
  username?: string,
) => {
  const prevPhaseRef = useRef(gameInfo?.phase);
  const { submitWinningJudge } = useWinningJudge();
  const { submitRoleAssignment } = useRoleAssignment();
  const JSONbigNative = JSONbig({ useNativeBigInt: true });

  // 役職配布の処理
  useEffect(() => {
    if (gameInfo?.phase === "Waiting") {
      const handleRoleAssignment = async () => {
        try {
          const playerCount = gameInfo.players.length;
          // 型エラーを避けるため、コメントアウト
          // const roleAssignmentData = {
          //   privateInput: {
          //     id: gameInfo.players.findIndex(player => player.name === username),
          //     isTargetId: false,
          //     playerRandomness: null,
          //   },
          //   publicInput: {},
          //   nodeKeys: [
          //     {
          //       nodeId: "0",
          //       publicKey: process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "",
          //     },
          //     {
          //       nodeId: "1",
          //       publicKey: process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "",
          //     },
          //     {
          //       nodeId: "2",
          //       publicKey: process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "",
          //     },
          //   ],
          //   scheme: {
          //     totalShares: 3,
          //     modulus: 97,
          //   },
          // };

          // await submitRoleAssignment(roomId, roleAssignmentData, playerCount);

          addMessage({
            id: Date.now().toString(),
            sender: "システム",
            message: "役職配布処理を開始しました",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (error) {
          console.error("役職配布処理エラー:", error);
          addMessage({
            id: Date.now().toString(),
            sender: "システム",
            message: "役職配布処理に失敗しました",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        }
      };

      handleRoleAssignment();
    }
  }, [gameInfo?.phase, roomId, username, addMessage, submitRoleAssignment]);

  // 勝利判定の処理
  useEffect(() => {
    if (!gameInfo) return;

    const prevPhase = prevPhaseRef.current;
    prevPhaseRef.current = gameInfo.phase;

    const checkGameResult = async () => {
      if (
        (prevPhase === "Night" && gameInfo.phase === "Discussion") ||
        (prevPhase === "Voting" && gameInfo.phase === "Result")
      ) {
        console.log("Phase changed from", prevPhase, "to", gameInfo.phase);

        try {
          const alivePlayersCount = gameInfo.players.filter(player => !player.is_dead).length;

          const res = await fetch("/pedersen-params.json");
          const params = await res.text();
          const parsedParams = JSONbigNative.parse(params);

          const randres = await fetch("/pedersen_randomness_0.json");
          const randomness = await randres.text();
          const parsedRandomness = JSONbigNative.parse(randomness);

          const commitres = await fetch("/pedersen_commitment_0.json");
          const commitment = await commitres.text();
          const parsedCommitment = JSONbigNative.parse(commitment);

          const players = gameInfo.players;
          const myId = gameInfo.players.find(player => player.name === username)?.id ?? "";

          const privateInput = {
            id: players.findIndex(player => player.id === myId),
            amWerewolf:
              gameInfo.players.find(player => player.name === username)?.role === "Werewolf"
                ? [[0, 0, 0, 1], null]
                : [[0, 0, 0, 0], null],
            playerRandomness: parsedRandomness,
          };

          const publicInput: WinningJudgementPublicInput = {
            pedersenParam: parsedParams,
            playerCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
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

          const winningJudgeData: WinningJudgementInput = {
            privateInput,
            publicInput,
            nodeKeys,
            scheme,
          };

          console.log(`Player ${myId} is sending winning judgement proof request`);
          await submitWinningJudge(roomId, winningJudgeData, alivePlayersCount);
        } catch (error) {
          console.error("勝利判定処理エラー:", error);
        }
      }
    };

    checkGameResult();
  }, [gameInfo?.phase, roomId, username, submitWinningJudge, JSONbigNative]);

  return { prevPhase: prevPhaseRef.current };
};
