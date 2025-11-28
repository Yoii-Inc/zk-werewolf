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
  const phaseTransitionProcessedRef = useRef<string | null>(null);
  const winningJudgementSentRef = useRef<string | null>(null);
  const divinationCompletedRef = useRef(false); // 占い完了フラグ
  const handleGameResultCheckRef = useRef<((transitionId: string) => void) | null>(null);

  // WebSocketからのフェーズ変更通知を処理
  useEffect(() => {
    const handlePhaseChangeNotification = async (event: Event) => {
      const customEvent = event as CustomEvent;
      const { fromPhase, toPhase, requiresDummyRequest } = customEvent.detail;

      console.log(`WebSocketフェーズ変更通知受信: ${fromPhase} → ${toPhase}`);

      if (!gameInfo || !username) return;

      const currentPlayer = gameInfo.players.find(player => player.name === username);
      if (!currentPlayer) return;

      // トランジションIDを生成
      const transitionId = `${fromPhase}_to_${toPhase}`;

      // hasActedをリセット
      updateHasActed(roomId, currentPlayer.id, false);
      console.log(`WebSocket通知によりhasActedをリセット: ${fromPhase} → ${toPhase}`);

      // 処理の優先順位を明確にした順次実行
      const processingSteps: (() => Promise<void>)[] = [];

      // Step 1: ダミーリクエスト送信
      if (requiresDummyRequest && currentPlayer.role !== "Seer" && !currentPlayer.is_dead) {
        processingSteps.push(async () => {
          console.log(`Step 1: 占い師以外のプレイヤー ${username} がダミーリクエストを送信します。`);

          try {
            await handleBackgroundNightAction(roomId, currentPlayer.id, gameInfo.players);

            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: "ダミーリクエストを送信しました",
              timestamp: new Date().toISOString(),
              type: "system",
            });

            console.log("Step 1: ダミーリクエスト送信完了");
          } catch (error) {
            console.error("Step 1: ダミーリクエスト送信エラー:", error);
            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: "ダミーリクエストの送信に失敗しました",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          }
        });
      }

      // Step 2: 勝利判定実行
      if ((fromPhase === "Night" && toPhase === "Discussion") || (fromPhase === "Voting" && toPhase === "Result")) {
        processingSteps.push(async () => {
          console.log(`Step 2: 勝利判定処理開始: ${fromPhase} → ${toPhase}`);

          if (handleGameResultCheckRef.current) {
            handleGameResultCheckRef.current(transitionId);
          }

          console.log("Step 2: 勝利判定処理完了");
        });
      }

      // 順次実行（ダミーリクエスト → 勝利判定の順序を保証）
      for (const step of processingSteps) {
        try {
          await step();
          // 各ステップ間に少し遅延を入れてサーバー側の処理順序を保証
          await new Promise(resolve => setTimeout(resolve, 300));
        } catch (error) {
          console.error("処理ステップでエラーが発生:", error);
        }
      }
    };

    window.addEventListener("phaseChangeNotification", handlePhaseChangeNotification);

    return () => {
      window.removeEventListener("phaseChangeNotification", handlePhaseChangeNotification);
    };
  }, [gameInfo, username, roomId, handleBackgroundNightAction, addMessage]);

  // 占い完了イベントを監視
  useEffect(() => {
    const handleDivinationCompleted = () => {
      console.log("占い処理完了イベントを受信しました");
      divinationCompletedRef.current = true;

      // 一定時間後にフラグをリセット
      const resetTimer = setTimeout(() => {
        divinationCompletedRef.current = false;
        console.log("占い完了フラグをリセットしました");
      }, 30000); // 30秒後にリセット

      return () => clearTimeout(resetTimer);
    };

    window.addEventListener("divinationCompleted", handleDivinationCompleted);

    return () => {
      window.removeEventListener("divinationCompleted", handleDivinationCompleted);
    };
  }, []);

  // 占いステータスを監視（従来の仕組みも残す）
  useEffect(() => {
    if (proofStatus === "completed") {
      console.log("占い結果の検証が完了しました（proofStatus経由）");
      divinationCompletedRef.current = true;

      // 一定時間後にフラグをリセット
      const resetTimer = setTimeout(() => {
        divinationCompletedRef.current = false;
      }, 30000); // 30秒後にリセット

      // クリーンアップ関数
      return () => clearTimeout(resetTimer);
    }
  }, [proofStatus]);

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

        // 占い師の場合は、占い結果が完了するまで待つ（最大30秒）
        return new Promise<void>(resolve => {
          const startTime = Date.now();
          const maxWaitTime = 30000; // 30秒

          const checkDivination = () => {
            const elapsedTime = Date.now() - startTime;

            if (divinationCompletedRef.current) {
              console.log("占い結果の処理が完了したため、勝敗判定を実行します");
              resolve();
            } else if (elapsedTime >= maxWaitTime) {
              console.log("占い結果の待機がタイムアウトしました。勝敗判定を続行します");
              resolve();
            } else {
              console.log(`占い結果の処理待機中... (${Math.round(elapsedTime / 1000)}秒経過)`);
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

        const res = await fetch("/pedersen_params2.json");
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
            ? JSONbigNative.parse(
                '["9015221291577245683", "8239323489949974514", "1646089257421115374", "958099254763297437"]',
              )
            : JSONbigNative.parse('["0", "0", "0", "0"]');

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

  // handleGameResultCheckをuseRefに設定
  useEffect(() => {
    handleGameResultCheckRef.current = handleGameResultCheck;
  }, [handleGameResultCheck]);

  // 役職配布の処理
  useEffect(() => {
    if (gameInfo?.phase === "Night" && gameInfo.players.some(p => p.role === null)) {
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

          // サーバー側エラーメッセージをチェック
          const errorMessage = error instanceof Error ? error.message : String(error);
          if (errorMessage.includes("Role assignment has already been completed")) {
            console.log("役職配布はすでに完了済みです");
            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: "役職配布はすでに完了しています",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          } else {
            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: "役職配布処理に失敗しました",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          }
        }
      };

      handleRoleAssignment();
    }
    //   }, [gameInfo?.phase, roomId, username, addMessage, submitRoleAssignment]);
  }, [gameInfo?.phase, roomId, username, gameInfo?.players]);

  // フェーズ変更の検出（基本的な更新のみ）
  useEffect(() => {
    if (!gameInfo) return;

    const prevPhase = prevPhaseRef.current;
    prevPhaseRef.current = gameInfo.phase;

    // フェーズが変わった時のログ出力のみ
    if (prevPhase && prevPhase !== gameInfo.phase) {
      console.log(`フェーズ変更を検知: ${prevPhase} → ${gameInfo.phase}`);
    }
  }, [gameInfo?.phase]);

  return { prevPhase: prevPhaseRef.current };
};
