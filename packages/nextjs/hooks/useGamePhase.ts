import { useCallback, useEffect, useRef } from "react";
import { useBackgroundNightAction } from "./useBackgroundNightAction";
import { useDivination } from "./useDivination";
import { useGameInputGenerator } from "./useGameInputGenerator";
import { useKeyPublicize } from "./useKeyPublicize";
import { useRoleAssignment } from "./useRoleAssignment";
import { useWinningJudge } from "./useWinningJudge";
import JSONbig from "json-bigint";
import { toast } from "react-hot-toast";
import * as GameInputGenerator from "~~/services/gameInputGenerator";
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
  const { submitKeyPublicize } = useKeyPublicize();
  const { handleBackgroundNightAction } = useBackgroundNightAction();
  const { proofStatus } = useDivination();
  const { isReady, generateRoleAssignmentInput, generateWinningJudgementInput } = useGameInputGenerator(
    roomId,
    username || "",
    gameInfo,
  );
  const phaseTransitionProcessedRef = useRef<string | null>(null);
  const winningJudgementSentRef = useRef<string | null>(null);
  const divinationCompletedRef = useRef(false); // 占い完了フラグ
  const commitmentsReadyRef = useRef(false); // コミットメント準備完了フラグ
  const handleGameResultCheckRef = useRef<((transitionId: string, latestGameInfo: GameInfo) => void) | null>(null);

  // WebSocketからのフェーズ変更通知を処理
  useEffect(() => {
    const handlePhaseChangeNotification = async (event: Event) => {
      const customEvent = event as CustomEvent;
      const { fromPhase, toPhase, requiresDummyRequest } = customEvent.detail;

      console.log(`WebSocket phase change notification: ${fromPhase} → ${toPhase}`);

      const phaseLabelMap: Record<string, string> = {
        Night: "🌙 Night Phase",
        Discussion: "☀️ Discussion Phase",
        Voting: "🗳️ Voting Phase",
        Result: "📢 Result Phase",
        DivinationProcessing: "🔮 Divination Processing",
        Finished: "🏁 Game Finished",
      };
      toast(`${phaseLabelMap[toPhase] ?? toPhase} started`, {
        duration: 3000,
        position: "top-center",
      });

      // WebSocketイベント発生時に最新のgameInfoを取得
      // (props経由のgameInfoはポーリングタイミング次第でnullや古い可能性がある)
      const fetchLatestGameInfo = async (): Promise<GameInfo | null> => {
        try {
          const response = await fetch(
            `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/state`,
          );
          if (!response.ok) {
            console.error("Failed to fetch game info");
            return null;
          }
          const data = await response.json();
          return data;
        } catch (error) {
          console.error("Error fetching game info:", error);
          return null;
        }
      };

      if (!username) {
        console.warn("Username not available, skipping phase change processing");
        return;
      }

      // 最新のgameInfoを取得
      const latestGameInfo = await fetchLatestGameInfo();
      if (!latestGameInfo) {
        console.error("Failed to get latest game info, skipping phase change processing");
        return;
      }

      // GameCryptoの初期化を確認・実行
      const ensureGameCryptoReady = async (): Promise<boolean> => {
        try {
          // 既に初期化済みの場合はスキップ
          if (GameInputGenerator.isInitialized(roomId, username)) {
            console.log("Game crypto already initialized");
            return true;
          }

          console.log("Initializing game crypto...");
          await GameInputGenerator.initializeGameCrypto(roomId, username, latestGameInfo);
          console.log("Game crypto initialization completed");
          return true;
        } catch (error) {
          console.error("Failed to initialize game crypto:", error);
          return false;
        }
      };

      const isCryptoReady = await ensureGameCryptoReady();
      if (!isCryptoReady) {
        console.error("Game crypto not ready, skipping phase change processing");
        return;
      }

      const currentPlayer = latestGameInfo.players.find(player => player.name === username);
      if (!currentPlayer) return;

      // トランジションIDを生成
      const transitionId = `${fromPhase}_to_${toPhase}`;

      // hasActedをリセット
      updateHasActed(roomId, currentPlayer.id, false);
      console.log(`Reset hasActed by WebSocket notification: ${fromPhase} → ${toPhase}`);

      // 処理の優先順位を明確にした順次実行
      const processingSteps: (() => Promise<void>)[] = [];

      // Step 0: 役職配布リクエスト送信（コミットメント完了通知を待つ）
      if (fromPhase === "Waiting" && toPhase === "Night") {
        console.log("Step 0: Waiting for all commitments to be ready before role assignment...");

        const handleRoleAssignment = async () => {
          try {
            // コミットメント完了フラグをチェック（最大30秒待機）
            const maxWaitTime = 30000; // 30秒
            const checkInterval = 500; // 0.5秒ごとにチェック
            let waited = 0;

            while (!commitmentsReadyRef.current && waited < maxWaitTime) {
              console.log(`Waiting for commitments... (${waited}ms / ${maxWaitTime}ms)`);
              await new Promise(resolve => setTimeout(resolve, checkInterval));
              waited += checkInterval;
            }

            if (!commitmentsReadyRef.current) {
              console.warn("Timeout waiting for commitments, proceeding anyway...");
              addMessage({
                id: Date.now().toString(),
                sender: "System",
                message: "Warning: Proceeding without all commitments confirmed",
                timestamp: new Date().toISOString(),
                type: "system",
              });
            } else {
              console.log("All commitments ready, proceeding with role assignment");
            }

            const playerCount = latestGameInfo.players.length;

            // latestGameInfoを使って直接サービスから入力を生成
            const roleAssignmentData = await GameInputGenerator.generateRoleAssignmentInput(
              roomId,
              username,
              latestGameInfo,
            );

            console.log(
              `Player ${username} (ID: ${roleAssignmentData.privateInput.id}) initiating role assignment for ${playerCount} players`,
            );

            await submitRoleAssignment(roomId, roleAssignmentData, playerCount);
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

      // Step 1: ダミーリクエスト送信
      if (
        requiresDummyRequest &&
        fromPhase === "Night" &&
        toPhase === "DivinationProcessing" &&
        !currentPlayer.is_dead
      ) {
        processingSteps.push(async () => {
          console.log(`Step 1: Player ${username} sending dummy request.`);

          try {
            await handleBackgroundNightAction(
              roomId,
              currentPlayer.id,
              latestGameInfo.players,
              username,
              latestGameInfo,
            );

            console.log("Step 1: Dummy request completed");
          } catch (error) {
            console.error("Step 1: Dummy request error:", error);
            // TODO: サーバー側でエラーメッセージを送るようになったら削除
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
            // 最新のgameInfoを再取得して渡す（投票結果反映後の状態を確実に取得）
            const currentGameInfo = await fetchLatestGameInfo();
            if (currentGameInfo) {
              handleGameResultCheckRef.current(transitionId, currentGameInfo);
            } else {
              console.error("Failed to fetch latest game info for winning judgement");
            }
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

  // コミットメント完了イベントを監視
  useEffect(() => {
    const handleCommitmentsReady = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { roomId: notifiedRoomId, commitmentsCount, totalPlayers } = customEvent.detail;

      console.log(
        `Commitments ready notification received for room ${notifiedRoomId}: ${commitmentsCount}/${totalPlayers}`,
      );

      if (notifiedRoomId === roomId) {
        commitmentsReadyRef.current = true;
        console.log("Commitments ready flag set to true");

        addMessage({
          id: Date.now().toString(),
          sender: "System",
          message: `All player commitments received (${commitmentsCount}/${totalPlayers})`,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    };

    window.addEventListener("commitmentsReadyNotification", handleCommitmentsReady);

    return () => {
      window.removeEventListener("commitmentsReadyNotification", handleCommitmentsReady);
    };
  }, [roomId, addMessage]);

  // ゲームリセット通知を監視してフラグをリセット
  useEffect(() => {
    const handleGameReset = (event: Event) => {
      const customEvent = event as CustomEvent;
      const { roomId: resetRoomId } = customEvent.detail;

      console.log("🔄 [useGamePhase] Game reset notification received for room:", resetRoomId);
      console.log("🔄 [useGamePhase] Current roomId:", roomId);

      if (resetRoomId === roomId) {
        // コミットメント準備完了フラグをリセット
        commitmentsReadyRef.current = false;
        console.log("✅ [useGamePhase] Commitments ready flag reset to false");

        // KeyPublicize実行済みフラグをリセット
        keyPublicizeExecutedRef.current = false;
        console.log("✅ [useGamePhase] KeyPublicize executed flag reset to false");
      } else {
        console.log("⚠️ [useGamePhase] Room ID mismatch, skipping reset");
      }
    };

    console.log("🎯 [useGamePhase] Adding gameResetNotification listener for room:", roomId);
    window.addEventListener("gameResetNotification", handleGameReset);

    return () => {
      console.log("🗑️ [useGamePhase] Removing gameResetNotification listener for room:", roomId);
      window.removeEventListener("gameResetNotification", handleGameReset);
    };
  }, [roomId]);

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
    async (phaseTransitionId: string, currentGameInfo: GameInfo) => {
      if (!username) return;

      try {
        // このフェーズ変更での勝敗判定をすでに実行済みとマーク
        winningJudgementSentRef.current = phaseTransitionId;
        console.log(`Starting winning judgement process. Transition ID: ${phaseTransitionId}`);

        const myId = currentGameInfo.players.find(player => player.name === username)?.id ?? "";

        // 最新のgameInfoを使って生存確認（投票結果が反映された状態）
        const isPlayerAlive = currentGameInfo.players.find(player => player.name === username)?.is_dead === false;
        if (!isPlayerAlive) {
          console.log(`Player ${myId} is dead - skipping winning judgement`);
          return;
        }

        const alivePlayersCount = currentGameInfo.players.filter(player => !player.is_dead).length;

        if (!isReady) {
          throw new Error("Game crypto not ready");
        }

        // 最新のgameInfoを使って勝利判定データを生成
        const winningJudgeData = await GameInputGenerator.generateWinningJudgementInput(
          roomId,
          username,
          currentGameInfo,
        );

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
    [roomId, username, submitWinningJudge, isReady],
  );

  // handleGameResultCheckをuseRefに設定
  useEffect(() => {
    handleGameResultCheckRef.current = handleGameResultCheck;
  }, [handleGameResultCheck]);

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

  // KeyPublicize実行済みフラグ（重複実行防止）
  const keyPublicizeExecutedRef = useRef(false);

  // 役職配布完了後にKeyPublicizeを実行
  useEffect(() => {
    const handleRoleAssignmentCompleted = async () => {
      // 既に実行済みの場合はスキップ
      if (keyPublicizeExecutedRef.current) {
        console.log("KeyPublicize: Already executed, skipping...");
        return;
      }

      if (!username || !roomId) {
        console.log("KeyPublicize: Missing required data (username or roomId)");
        return;
      }

      console.log("Role assignment completed event received, starting KeyPublicize...");

      // 実行済みフラグを立てる
      keyPublicizeExecutedRef.current = true;

      try {
        // 最新のゲーム状態を取得
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/state`,
        );
        if (!response.ok) {
          throw new Error("Failed to fetch latest game state");
        }
        const latestGameInfo = await response.json();

        // KeyPublicize入力を生成
        const keyPublicizeData = await GameInputGenerator.generateKeyPublicizeInput(roomId, username, latestGameInfo);

        const alivePlayersCount = latestGameInfo.players.filter((player: any) => !player.is_dead).length;

        console.log("Submitting KeyPublicize request...");
        await submitKeyPublicize(roomId, keyPublicizeData, alivePlayersCount);
        console.log("KeyPublicize request submitted successfully");
      } catch (error) {
        console.error("KeyPublicize error:", error);
        addMessage({
          id: Date.now().toString(),
          sender: "System",
          message: `Failed to submit KeyPublicize: ${error instanceof Error ? error.message : String(error)}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    };

    window.addEventListener("roleAssignmentCompleted", handleRoleAssignmentCompleted);

    return () => {
      window.removeEventListener("roleAssignmentCompleted", handleRoleAssignmentCompleted);
    };
  }, [username, roomId, submitKeyPublicize, addMessage]);

  return { prevPhase: prevPhaseRef.current };
};
