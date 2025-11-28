import { useCallback, useEffect, useState } from "react";
import JSONbig from "json-bigint";
import type { ChatMessage } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { getPrivateGameInfo, updatePrivateGameInfo } from "~~/utils/privateGameInfoUtils";

const JSONbigNative = JSONbig({ useNativeBigInt: true });

interface ComputationResult {
  computationType: string;
  resultData: any;
  targetPlayerId?: string;
  batchId: string;
  timestamp: string;
}

interface DivinationResult {
  ciphertext: any;
  status: string;
}

interface RoleAssignmentResult {
  role_assignments: Array<{
    player_id: string;
    player_name: string;
    role: string;
  }>;
  status: string;
}

interface WinningJudgeResult {
  game_result: "VillagerWin" | "WerewolfWin" | "InProgress";
  game_state_value: string;
  status: string;
}

interface AnonymousVotingResult {
  executed_player_id: string;
  executed_player_name: string;
  status: string;
}

export const useComputationResults = (
  roomId: string,
  playerId: string,
  addMessage: (message: ChatMessage) => void,
  gameInfo?: any,
) => {
  const [divinationResult, setDivinationResult] = useState<DivinationResult | null>(null);
  const [roleAssignmentResult, setRoleAssignmentResult] = useState<RoleAssignmentResult | null>(null);
  const [winningJudgeResult, setWinningJudgeResult] = useState<WinningJudgeResult | null>(null);
  const [votingResult, setVotingResult] = useState<AnonymousVotingResult | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  // WebSocketからの計算結果通知を処理
  useEffect(() => {
    const handleComputationResult = async (event: Event) => {
      const customEvent = event as CustomEvent;
      const result: ComputationResult = customEvent.detail;

      console.log(`計算結果受信: ${result.computationType}`, result);

      // 対象プレイヤーのチェック（指定がある場合）
      if (result.targetPlayerId && result.targetPlayerId !== playerId) {
        return; // 自分宛てでない場合はスキップ
      }

      setIsProcessing(true);

      try {
        switch (result.computationType) {
          case "divination":
            // 占い結果の処理
            setDivinationResult(result.resultData);

            // プレイヤーの役職を確認
            const privateGameInfo = getPrivateGameInfo(roomId, playerId);

            if (privateGameInfo?.playerRole === "Seer") {
              console.log("占い師として占い結果を復号化します");

              try {
                // ElGamal秘密鍵をJSONファイルから読み取り
                const secretKeyResponse = await fetch("/elgamal_secret_key.json");
                if (!secretKeyResponse.ok) {
                  throw new Error("ElGamal秘密鍵の読み込みに失敗しました");
                }

                const secretKeyText = await secretKeyResponse.text();
                const secretKey = JSONbigNative.parse(secretKeyText);

                // ElGamalパラメータをJSONファイルから読み取り
                const paramsResponse = await fetch("/test_elgamal_params.json");
                if (!paramsResponse.ok) {
                  throw new Error("ElGamalパラメータの読み込みに失敗しました");
                }

                const paramsText = await paramsResponse.text();
                const elgamalParams = JSONbigNative.parse(paramsText);

                console.log("占い結果の復号化を開始:", {
                  ciphertext: result.resultData.ciphertext,
                  secretKey: secretKey,
                  elgamalParams: elgamalParams,
                });

                // WASM復号化処理を実行
                const decryptInput = {
                  elgamalParams: elgamalParams,
                  secretKey: secretKey,
                  ciphertext: result.resultData.ciphertext,
                };

                const decryptedResult = await MPCEncryption.decryptElGamal(decryptInput);
                console.log("復号化結果:", decryptedResult);

                const notWerewolf = {
                  x: [["0", "0", "0", "0"], null],
                  y: [
                    ["9015221291577245683", "8239323489949974514", "1646089257421115374", "958099254763297437"],
                    null,
                  ],
                  _params: null,
                };

                const werewolf = {
                  x: [
                    ["469834705808616970", "3489346716202062344", "3775031930862818012", "1284874629665735135"],
                    null,
                  ],
                  y: [
                    ["3606830077131325521", "9477679840825260018", "8867541030756743570", "1156619796726615314"],
                    null,
                  ],
                  _params: null,
                };

                const decryptedStr = JSON.stringify(decryptedResult);
                const notWerewolfStr = JSON.stringify(notWerewolf);
                const werewolfStr = JSON.stringify(werewolf);

                let isWerewolf: boolean;
                if (decryptedStr === notWerewolfStr) {
                  isWerewolf = false;
                } else if (decryptedStr === werewolfStr) {
                  isWerewolf = true;
                } else {
                  console.warn("占い結果が期待値と一致しません", decryptedResult);
                  addMessage({
                    id: Date.now().toString(),
                    sender: "システム",
                    message: "占い結果が正しくありません。",
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                  return;
                }

                console.log("占い結果(復号化):", decryptedResult);
                console.log("判定:", isWerewolf ? "人狼です" : "人狼ではありません");
                if (result.targetPlayerId) {
                  console.log("対象プレイヤーID:", result.targetPlayerId);
                }

                addMessage({
                  id: Date.now().toString(),
                  sender: "システム",
                  message: `占い結果: ${isWerewolf ? "人狼です" : "人狼ではありません"}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });

                // 占い処理完了をグローバルイベントで通知
                window.dispatchEvent(new CustomEvent("divinationCompleted"));
                console.log("占い処理完了イベントを発行しました");

                addMessage({
                  id: Date.now().toString(),
                  sender: "システム",
                  message: `占い結果を復号化しました: ${JSON.stringify(decryptedResult)}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });
              } catch (error) {
                console.error("占い結果の復号化エラー:", error);
                addMessage({
                  id: Date.now().toString(),
                  sender: "システム",
                  message: `占い結果の復号化に失敗しました: ${error}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });
              }
            } else {
              addMessage({
                id: Date.now().toString(),
                sender: "システム",
                message: "占い結果が準備されました。",
                timestamp: new Date().toISOString(),
                type: "system",
              });
            }
            break;
          case "role_assignment":
            setRoleAssignmentResult(result.resultData);

            // ダミーコード: gameInfoから役職を取得してprivateGameInfoを更新
            console.log("役職配布結果を受信、gameInfoから役職を取得します");

            if (gameInfo && gameInfo.players) {
              const currentPlayer = gameInfo.players.find((player: any) => player.id === playerId);

              if (currentPlayer && currentPlayer.role) {
                console.log("gameInfoから取得した役職:", currentPlayer.role);

                // privateGameInfoを更新
                const updatedInfo = updatePrivateGameInfo(roomId, playerId, {
                  playerRole: currentPlayer.role,
                });

                if (updatedInfo) {
                  console.log("privateGameInfo更新 (gameInfoベース):", updatedInfo);

                  addMessage({
                    id: Date.now().toString(),
                    sender: "システム",
                    message: `あなたの役職は「${currentPlayer.role}」です。`,
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                } else {
                  console.warn("privateGameInfoの更新に失敗しました。初期化されていない可能性があります。");

                  addMessage({
                    id: Date.now().toString(),
                    sender: "システム",
                    message: "役職情報の更新に失敗しました。ゲームを再開してください。",
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                }
              } else {
                console.warn("gameInfoから役職情報を取得できませんでした");
              }
            } else {
              console.warn("gameInfoが利用できません");
            }

            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: "役職配布が完了しました。",
              timestamp: new Date().toISOString(),
              type: "system",
            });
            break;
          case "winning_judge":
            setWinningJudgeResult(result.resultData);
            if (result.resultData.game_result !== "InProgress") {
              const resultMessage =
                result.resultData.game_result === "VillagerWin" ? "村人陣営の勝利です！" : "人狼陣営の勝利です！";
              addMessage({
                id: Date.now().toString(),
                sender: "システム",
                message: resultMessage,
                timestamp: new Date().toISOString(),
                type: "system",
              });
            }
            break;
          case "anonymous_voting":
            setVotingResult(result.resultData);
            addMessage({
              id: Date.now().toString(),
              sender: "システム",
              message: `${result.resultData.executed_player_name}が処刑されました。`,
              timestamp: new Date().toISOString(),
              type: "system",
            });
            break;
          default:
            console.warn("Unknown computation type:", result.computationType);
        }
      } catch (error) {
        console.error("計算結果の処理エラー:", error);
        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: `計算結果の処理に失敗しました: ${result.computationType}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      } finally {
        setIsProcessing(false);
      }
    };

    window.addEventListener("computationResultNotification", handleComputationResult);

    return () => {
      window.removeEventListener("computationResultNotification", handleComputationResult);
    };
  }, [playerId, addMessage, roomId, gameInfo]);

  // 占い結果の処理
  const handleDivinationResult = useCallback(
    async (data: DivinationResult) => {
      setDivinationResult(data);

      // プレイヤーの役職を確認
      const privateGameInfo = getPrivateGameInfo(roomId, playerId);

      if (privateGameInfo?.playerRole === "Seer") {
        console.log("占い師として占い結果を復号化します");

        try {
          // ElGamal秘密鍵をJSONファイルから読み取り
          const secretKeyResponse = await fetch("/elgamal_secret_key.json");
          if (!secretKeyResponse.ok) {
            throw new Error("ElGamal秘密鍵の読み込みに失敗しました");
          }

          const secretKeyText = await secretKeyResponse.text();
          const secretKey = JSONbigNative.parse(secretKeyText);

          console.log("占い結果の復号化を開始:", {
            ciphertext: data.ciphertext,
            secretKey: secretKey,
          });

          // TODO: 実際の復号化処理を実装
          // 現在は復号化ロジックの代わりに暗号文と秘密鍵をログ出力
          console.log("暗号文:", data.ciphertext);
          console.log("秘密鍵:", secretKey);

          addMessage({
            id: Date.now().toString(),
            sender: "システム",
            message: "占い結果を復号化しました。（詳細はコンソールログを確認してください）",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (error) {
          console.error("占い結果の復号化エラー:", error);
          addMessage({
            id: Date.now().toString(),
            sender: "システム",
            message: `占い結果の復号化に失敗しました: ${error}`,
            timestamp: new Date().toISOString(),
            type: "system",
          });
        }
      } else {
        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: "占い結果が準備されました。",
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    },
    [roomId, playerId, addMessage],
  );

  // 役職配布結果の処理
  const handleRoleAssignmentResult = useCallback(
    (data: RoleAssignmentResult) => {
      setRoleAssignmentResult(data);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: "役職配布が完了しました。",
        timestamp: new Date().toISOString(),
        type: "system",
      });
    },
    [addMessage],
  );

  // 勝利判定結果の処理
  const handleWinningJudgeResult = useCallback(
    (data: WinningJudgeResult) => {
      setWinningJudgeResult(data);

      if (data.game_result !== "InProgress") {
        const resultMessage = data.game_result === "VillagerWin" ? "村人陣営の勝利です！" : "人狼陣営の勝利です！";

        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: resultMessage,
          timestamp: new Date().toISOString(),
          type: "system",
        });
      }
    },
    [addMessage],
  );

  // 投票結果の処理
  const handleVotingResult = useCallback(
    (data: AnonymousVotingResult) => {
      setVotingResult(data);
      addMessage({
        id: Date.now().toString(),
        sender: "システム",
        message: `${data.executed_player_name}が処刑されました。`,
        timestamp: new Date().toISOString(),
        type: "system",
      });
    },
    [addMessage],
  );

  // 占い結果の復号化処理
  const decryptDivinationResult = useCallback(
    async (privateKey: string) => {
      if (!divinationResult) {
        throw new Error("占い結果がありません");
      }

      try {
        const response = await fetch(`/api/game/${roomId}/divination/decrypt`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            player_id: playerId,
            private_key: privateKey,
          }),
        });

        if (!response.ok) {
          throw new Error(`復号化に失敗しました: ${response.statusText}`);
        }

        const decryptedResult = await response.json();

        addMessage({
          id: Date.now().toString(),
          sender: "システム",
          message: `占い結果: ${decryptedResult.is_werewolf ? "人狼です" : "人狼ではありません"}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });

        return decryptedResult;
      } catch (error) {
        console.error("復号化エラー:", error);
        throw error;
      }
    },
    [divinationResult, roomId, playerId, addMessage],
  );

  return {
    divinationResult,
    roleAssignmentResult,
    winningJudgeResult,
    votingResult,
    isProcessing,
    decryptDivinationResult,
  };
};
