import { useCallback, useEffect, useRef } from "react";
import { useBackgroundNightAction } from "./useBackgroundNightAction";
import { useDivination } from "./useDivination";
import { useGameInputGenerator } from "./useGameInputGenerator";
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
  const { inputGenerator, isReady } = useGameInputGenerator(roomId, username || "", gameInfo);
  const phaseTransitionProcessedRef = useRef<string | null>(null);
  const winningJudgementSentRef = useRef<string | null>(null);
  const divinationCompletedRef = useRef(false); // 占い完了フラグ
  const handleGameResultCheckRef = useRef<((transitionId: string) => void) | null>(null);

  // WebSocketからのフェーズ変更通知を処理
  useEffect(() => {
    const handlePhaseChangeNotification = async (event: Event) => {
      const customEvent = event as CustomEvent;
      const { fromPhase, toPhase, requiresDummyRequest } = customEvent.detail;

      //   console.log(`WebSocketフェーズ変更通知受信: ${fromPhase} → ${toPhase}`);

      if (!gameInfo || !username) return;

      const currentPlayer = gameInfo.players.find(player => player.name === username);
      if (!currentPlayer) return;

      // トランジションIDを生成
      const transitionId = `${fromPhase}_to_${toPhase}`;

      // hasActedをリセット
      updateHasActed(roomId, currentPlayer.id, false);
      console.log(`Reset hasActed by WebSocket notification: ${fromPhase} → ${toPhase}`);

      // 処理の優先順位を明確にした順次実行
      const processingSteps: (() => Promise<void>)[] = [];

      // Step 1: ダミーリクエスト送信
      if (
        requiresDummyRequest &&
        fromPhase === "Night" &&
        toPhase === "DivinationProcessing" &&
        currentPlayer.role !== "Seer" &&
        !currentPlayer.is_dead
      ) {
        processingSteps.push(async () => {
          console.log(`Step 1: Non-Seer player ${username} sending dummy request.`);

          try {
            if (!inputGenerator) {
              console.error("inputGenerator is null. Cannot send dummy request.");
              addMessage({
                id: Date.now().toString(),
                sender: "System",
                message: "Failed to send dummy request: inputGenerator is null",
                timestamp: new Date().toISOString(),
                type: "system",
              });
              return;
            }

            await handleBackgroundNightAction(roomId, currentPlayer.id, gameInfo.players, inputGenerator);

            addMessage({
              id: Date.now().toString(),
              sender: "System",
              message: "Dummy request sent",
              timestamp: new Date().toISOString(),
              type: "system",
            });

            console.log("Step 1: Dummy request completed");
          } catch (error) {
            console.error("Step 1: Dummy request error:", error);
            addMessage({
              id: Date.now().toString(),
              sender: "System",
              message: "Failed to send dummy request",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          }
        });
      }

      // Step 2: 勝利判定実行（DivinationProcessing → Discussion または Voting → Result）
      if (
        (fromPhase === "DivinationProcessing" && toPhase === "Discussion") ||
        (fromPhase === "Voting" && toPhase === "Result")
      ) {
        processingSteps.push(async () => {
          console.log(`Step 2: Starting winning judgement process: ${fromPhase} → ${toPhase}`);

          if (handleGameResultCheckRef.current) {
            handleGameResultCheckRef.current(transitionId);
          }

          console.log("Step 2: Winning judgement process completed");
        });
      }

      // 順次実行（ダミーリクエスト → 勝利判定の順序を保証）
      for (const step of processingSteps) {
        try {
          await step();
          // 各ステップ間に少し遅延を入れてサーバー側の処理順序を保証
          await new Promise(resolve => setTimeout(resolve, 300));
        } catch (error) {
          console.error("Error occurred in processing step:", error);
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
      console.log("Divination completion event received");
      divinationCompletedRef.current = true;

      // 一定時間後にフラグをリセット
      const resetTimer = setTimeout(() => {
        divinationCompletedRef.current = false;
        console.log("Divination completion flag reset");
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
      console.log("Divination result verification completed (via proofStatus)");
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
        // Non-Seer players proceed immediately with winning judgement
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
              console.log("Divination processing completed, executing winning judgement");
              resolve();
            } else if (elapsedTime >= maxWaitTime) {
              console.log("Divination wait timeout. Continuing with winning judgement");
              resolve();
            } else {
              console.log(`Waiting for divination processing... (${Math.round(elapsedTime / 1000)} seconds elapsed)`);
              setTimeout(checkDivination, 1000); // 1秒ごとにチェック
            }
          };

          checkDivination();
        });
      };

      try {
        // このフェーズ変更での勝敗判定をすでに実行済みとマーク
        winningJudgementSentRef.current = phaseTransitionId;
        console.log(`Starting winning judgement process. Transition ID: ${phaseTransitionId}`);

        // Wait for divination results
        await waitForDivination();
        const alivePlayersCount = gameInfo.players.filter(player => !player.is_dead).length;

        if (!inputGenerator || !isReady) {
          throw new Error("Input generator not ready");
        }

        // const players = gameInfo.players;
        const myId = gameInfo.players.find(player => player.name === username)?.id ?? "";

        const { input: winningJudgeData } = await inputGenerator.getWinningJudgementInput();

        // Only proceed if the player is alive
        const isPlayerAlive = gameInfo.players.find(player => player.name === username)?.is_dead === false;
        if (!isPlayerAlive) {
          console.log(`Player ${myId} is dead - skipping winning judgement`);
          return;
        }

        console.log(`Player ${myId} is sending winning judgement proof request`);
        await submitWinningJudge(roomId, winningJudgeData, alivePlayersCount);
        console.log(`Player ${myId} winning judgement request completed`);
      } catch (error) {
        console.error("Winning judgement process error:", error);
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
    [gameInfo, roomId, username, submitWinningJudge, inputGenerator, isReady],
  );

  // handleGameResultCheckをuseRefに設定
  useEffect(() => {
    handleGameResultCheckRef.current = handleGameResultCheck;
  }, [handleGameResultCheck]);

  // 役職配布の処理
  useEffect(() => {
    if (gameInfo?.phase === "Night" && gameInfo.players.some(p => p.role === null) && inputGenerator && isReady) {
      const handleRoleAssignment = async () => {
        try {
          const playerCount = gameInfo.players.length;

          const { input: roleAssignmentData } = await inputGenerator.getRoleAssignmentInput();

          console.log(
            `Player ${username} (ID: ${roleAssignmentData.privateInput.id}) initiating role assignment for ${playerCount} players`,
          );

          await submitRoleAssignment(roomId, roleAssignmentData, playerCount);

          addMessage({
            id: Date.now().toString(),
            sender: "System",
            message: "Role assignment process started",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (error) {
          console.error("Role assignment process error:", error);

          // サーバー側エラーメッセージをチェック
          const errorMessage = error instanceof Error ? error.message : String(error);
          if (errorMessage.includes("Role assignment has already been completed")) {
            console.log("Role assignment already completed");
            addMessage({
              id: Date.now().toString(),
              sender: "System",
              message: "Role assignment already completed",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          } else {
            addMessage({
              id: Date.now().toString(),
              sender: "System",
              message: "Role assignment process failed",
              timestamp: new Date().toISOString(),
              type: "system",
            });
          }
        }
      };

      handleRoleAssignment();
    }
    //   }, [gameInfo?.phase, roomId, username, addMessage, submitRoleAssignment]);
  }, [gameInfo?.phase, roomId, username, gameInfo?.players, inputGenerator, isReady]);

  // フェーズ変更の検出（基本的な更新のみ）
  useEffect(() => {
    if (!gameInfo) return;

    const prevPhase = prevPhaseRef.current;
    prevPhaseRef.current = gameInfo.phase;

    // フェーズが変わった時のログ出力のみ
    if (prevPhase && prevPhase !== gameInfo.phase) {
      console.log(`Phase change detected: ${prevPhase} → ${gameInfo.phase}`);
    }
  }, [gameInfo?.phase]);

  return { prevPhase: prevPhaseRef.current };
};
