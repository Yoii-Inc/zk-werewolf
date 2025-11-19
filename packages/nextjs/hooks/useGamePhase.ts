import { useCallback, useEffect, useRef } from "react";
import { useBackgroundNightAction } from "./useBackgroundNightAction";
import { useDivination } from "./useDivination";
import { useRoleAssignment } from "./useRoleAssignment";
import { useWinningJudge } from "./useWinningJudge";
import JSONbig from "json-bigint";
import type { ChatMessage, GameInfo } from "~~/types/game";
import {
  NodeKey,
  RoleAssignmentInput,
  RoleAssignmentPrivateInput,
  RoleAssignmentPublicInput,
  SecretSharingScheme,
  WinningJudgementInput,
  WinningJudgementPublicInput,
} from "~~/utils/crypto/type";
import { updateHasActed } from "~~/utils/privateGameInfoUtils";

const JSONbigNative = JSONbig({ useNativeBigInt: true });

export const useGamePhase = (
  gameInfo: GameInfo | null,
  roomId: string,
  addMessage: (message: ChatMessage) => void,
  username?: string,
) => {
  const prevPhaseRef = useRef(gameInfo?.phase);
  const { submitWinningJudge } = useWinningJudge();
  const { submitRoleAssignment } = useRoleAssignment();
  const { handleBackgroundNightAction } = useBackgroundNightAction();
  const { proofStatus } = useDivination();
  const roleAssignmentRequestedRef = useRef(false);
  const phaseTransitionProcessedRef = useRef<string | null>(null);
  const winningJudgementSentRef = useRef<string | null>(null);
  const divinationCompletedRef = useRef(false); // 占い完了フラグ

  // 占いステータスを監視
  useEffect(() => {
    if (proofStatus === "completed") {
      console.log("占い結果の検証が完了しました");
      divinationCompletedRef.current = true;

      // 一定時間後にフラグをリセット
      const resetTimer = setTimeout(() => {
        divinationCompletedRef.current = false;
      }, 30000); // 30秒後にリセット

      // クリーンアップ関数
      return () => clearTimeout(resetTimer);
    }
  }, [proofStatus]);

  // ゲーム開始時にroleAssignmentRequestedRefをリセットする
  useEffect(() => {
    if (gameInfo?.phase === "Waiting") {
      roleAssignmentRequestedRef.current = false;
      console.log("ゲーム開始: roleAssignmentRequestedRefをリセットしました");
    }
  }, [gameInfo?.phase]);

  // 勝敗判定処理を行う関数
  const handleGameResultCheck = useCallback(
    async (phaseTransitionId: string) => {
      if (!gameInfo || winningJudgementSentRef.current === phaseTransitionId) return;

      // 占い師の結果を待つ処理
      const waitForDivination = () => {
        // 占い師以外のプレイヤーはすぐに勝敗判定を行う
        const currentPlayer = gameInfo.players.find(player => player.name === username);
        if (!currentPlayer || currentPlayer.role !== "Seer") {
          return Promise.resolve();
        }

        // 占い師の場合は、占い結果が完了するまで待つ
        return new Promise<void>(resolve => {
          const checkDivination = () => {
            if (divinationCompletedRef.current) {
              console.log("占い結果の処理が完了したため、勝敗判定を実行します");
              resolve();
            } else {
              console.log("占い結果の処理待機中...");
              setTimeout(checkDivination, 1000); // 1秒ごとにチェック
            }
          };

          checkDivination();
        });
      };

      try {
        // このフェーズ変更での勝敗判定をすでに実行済みとマーク
        winningJudgementSentRef.current = phaseTransitionId;
        console.log(`勝敗判定処理を開始します。トランジションID: ${phaseTransitionId}`);

        // 占い結果を待つ
        await waitForDivination();
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

        const amWerewolfValues =
          gameInfo.players.find(player => player.name === username)?.role === "Werewolf"
            ? JSONbigNative.parse("[9015221291577245683, 8239323489949974514, 1646089257421115374, 958099254763297437]")
            : [0n, 0n, 0n, 0n];

        const privateInput = {
          id: players.findIndex(player => player.id === myId),
          amWerewolf: [amWerewolfValues, null],
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

        // Only proceed if the player is alive
        const isPlayerAlive = gameInfo.players.find(player => player.name === username)?.is_dead === false;
        if (!isPlayerAlive) {
          console.log(`Player ${myId} is dead - skipping winning judgement`);
          return;
        }

        console.log(`Player ${myId} is sending winning judgement proof request`);
        await submitWinningJudge(roomId, winningJudgeData, alivePlayersCount);
        console.log(`Player ${myId} の勝敗判定リクエスト送信完了`);
      } catch (error) {
        console.error("勝利判定処理エラー:", error);
        // エラー時もフラグをリセット（一定時間後）
        const resetTimer = setTimeout(() => {
          if (winningJudgementSentRef.current === phaseTransitionId) {
            winningJudgementSentRef.current = null;
          }
        }, 10000);

        // クリーンアップ時にタイマーをクリア
        return () => clearTimeout(resetTimer);
      }
    },
    [gameInfo, roomId, username, submitWinningJudge],
  );

  // 役職配布の処理
  useEffect(() => {
    console.log("Role assignment requested:", roleAssignmentRequestedRef.current);
    if (gameInfo?.phase === "Night" && !roleAssignmentRequestedRef.current) {
      // 役職配布処理のフラグを立てる
      roleAssignmentRequestedRef.current = true;
      const handleRoleAssignment = async () => {
        try {
          const playerCount = gameInfo.players.length;
          // 型エラーを避けるため、コメントアウト

          const res = await fetch("/pedersen-params.json");
          const params = await res.text();
          const parsedParams = JSONbigNative.parse(params);

          const commitres = await fetch("/pedersen_commitment_0.json");
          const commitment = await commitres.text();
          const parsedCommitment = JSONbigNative.parse(commitment);

          //   const privateInput: RoleAssignmentPrivateInput = {
          //     id: gameInfo.players.findIndex(player => player.name === username),
          //     shuffleMatrices: null,
          //     randomness: null,
          //     playerRandomness: parsedParams,
          //   };

          const publicInput: RoleAssignmentPublicInput = {
            numPlayers: 3,
            maxGroupSize: 3,
            pedersenParam: parsedParams,
            tauMatrix: null,
            roleCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
            playerCommitment: [parsedCommitment, parsedCommitment, parsedCommitment],
            groupingParameter: {
              Villager: [2, false],
              FortuneTeller: [1, false],
              Werewolf: [1, false],
            },
          };

          const rinputres = await fetch("/test_role_assignment_input2.json");
          const rinput = await rinputres.text();
          const parsedRinput: RoleAssignmentInput = JSONbigNative.parse(rinput);

          const roleAssignmentData: RoleAssignmentInput = {
            privateInput: parsedRinput.privateInput,
            publicInput: parsedRinput.publicInput,
            // publicInput,
            nodeKeys: [
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
            ],
            scheme: {
              totalShares: 3,
              modulus: 97,
            },
          };

          roleAssignmentData.privateInput.id = gameInfo.players.findIndex(player => player.name === username);

          console.log(
            `Player ${username} (ID: ${roleAssignmentData.privateInput.id}) initiating role assignment for ${playerCount} players`,
          );

          await submitRoleAssignment(roomId, roleAssignmentData, playerCount);

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
    //   }, [gameInfo?.phase, roomId, username, addMessage, submitRoleAssignment]);
  }, [gameInfo?.phase, roomId, username]);

  // フェーズ変更の検出と処理
  useEffect(() => {
    if (!gameInfo) return;

    const prevPhase = prevPhaseRef.current;
    prevPhaseRef.current = gameInfo.phase;

    // トランジションIDを生成（フェーズ変更の一意性を確保するため）
    const transitionId = prevPhase && `${prevPhase}_to_${gameInfo.phase}`;

    // transitionIdがundefinedの場合は処理しない
    if (!transitionId) return;

    // 既に処理済みのトランジションならスキップ
    if (phaseTransitionProcessedRef.current === transitionId) {
      return;
    }

    // フェーズが変わった時の処理
    if (prevPhase && prevPhase !== gameInfo.phase) {
      // このトランジションを処理済みとしてマーク
      phaseTransitionProcessedRef.current = transitionId;

      const currentPlayer = gameInfo.players.find(player => player.name === username);
      const currentPlayerId = currentPlayer?.id;

      // hasActedをリセット
      if (currentPlayerId) {
        updateHasActed(roomId, currentPlayerId, false);
        console.log(`Phase changed from ${prevPhase} to ${gameInfo.phase}, hasActed reset to false`);
      }

      // Night → Discussionへの変更時の処理
      if (prevPhase === "Night" && gameInfo.phase === "Discussion") {
        console.log(`フェーズ変更を検知: ${prevPhase} → ${gameInfo.phase}`);

        // 占い師以外の生きているプレイヤーの場合、ダミーリクエストを送信
        if (currentPlayer && currentPlayer.role !== "Seer" && !currentPlayer.is_dead) {
          // ダミーリクエストの送信
          const sendBackgroundNightAction = async () => {
            try {
              console.log(`占い師以外のプレイヤー ${username} がダミーリクエストを送信します。`);
              await handleBackgroundNightAction(roomId, currentPlayer.id, gameInfo.players);
            } catch (error) {
              console.error("ダミーリクエスト送信エラー:", error);
            }
          };

          sendBackgroundNightAction();
        }

        // 勝敗判定処理を実行
        handleGameResultCheck(transitionId);
      } else if (prevPhase === "Voting" && gameInfo.phase === "Result") {
        // 勝敗判定処理を実行
        handleGameResultCheck(transitionId);
      }

      // 次のフェーズ変更のために一定時間後にリセット
      const resetTimer = setTimeout(() => {
        if (phaseTransitionProcessedRef.current === transitionId) {
          phaseTransitionProcessedRef.current = null;
          winningJudgementSentRef.current = null;
          console.log(`フェーズ変更フラグをリセットしました: ${transitionId}`);
        }
      }, 10000);

      // コンポーネントのアンマウント時などにタイマーをクリア
      return () => clearTimeout(resetTimer);
    }
  }, [gameInfo, roomId, username, handleBackgroundNightAction, handleGameResultCheck]);

  return { prevPhase: prevPhaseRef.current };
};
