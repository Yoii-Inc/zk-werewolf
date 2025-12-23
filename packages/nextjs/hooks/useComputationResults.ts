import { useCallback, useEffect, useState } from "react";
import JSONbig from "json-bigint";
import { loadCryptoParams } from "~~/services/gameInputGenerator";
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

      console.log(`Computation result received: ${result.computationType}`, result);

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
              console.log("Decrypting divination result as Seer");

              try {
                // ElGamal秘密鍵をJSONファイルから読み取り
                const secretKeyResponse = await fetch("/elgamal_secret_key.json");
                if (!secretKeyResponse.ok) {
                  throw new Error("Failed to load ElGamal secret key");
                }

                const secretKeyText = await secretKeyResponse.text();
                const secretKey = JSONbigNative.parse(secretKeyText);

                // ElGamalパラメータを取得（キャッシュされたcryptoParamsから）
                const cryptoParams = await loadCryptoParams();
                const elgamalParams = cryptoParams.elgamalParam;

                console.log("Starting divination result decryption:", {
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
                console.log("Decryption result:", decryptedResult);

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
                  console.warn("Divination result does not match expected values", decryptedResult);
                  addMessage({
                    id: Date.now().toString(),
                    sender: "System",
                    message: "Divination result is not valid.",
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                  return;
                }

                console.log("Divination result (decrypted):", decryptedResult);
                console.log("Judgment:", isWerewolf ? "Werewolf" : "Not werewolf");
                if (result.targetPlayerId) {
                  console.log("Target player ID:", result.targetPlayerId);
                }

                addMessage({
                  id: Date.now().toString(),
                  sender: "System",
                  message: `Divination result: ${isWerewolf ? "Werewolf" : "Not werewolf"}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });

                // 占い処理完了をグローバルイベントで通知
                window.dispatchEvent(new CustomEvent("divinationCompleted"));
                console.log("Divination completion event dispatched");

                addMessage({
                  id: Date.now().toString(),
                  sender: "System",
                  message: `Divination result decrypted: ${JSON.stringify(decryptedResult)}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });
              } catch (error) {
                console.error("Divination result decryption error:", error);
                addMessage({
                  id: Date.now().toString(),
                  sender: "System",
                  message: `Divination result decryption failed: ${error}`,
                  timestamp: new Date().toISOString(),
                  type: "system",
                });
              }
            } else {
              addMessage({
                id: Date.now().toString(),
                sender: "System",
                message: "Divination result is ready.",
                timestamp: new Date().toISOString(),
                type: "system",
              });
            }
            break;
          case "role_assignment":
            setRoleAssignmentResult(result.resultData);

            // ダミーコード: gameInfoから役職を取得してprivateGameInfoを更新
            console.log("Role assignment result received, retrieving role from gameInfo");

            if (gameInfo && gameInfo.players) {
              const currentPlayer = gameInfo.players.find((player: any) => player.id === playerId);

              if (currentPlayer && currentPlayer.role) {
                console.log("Role retrieved from gameInfo:", currentPlayer.role);

                // privateGameInfoを更新
                const updatedInfo = updatePrivateGameInfo(roomId, playerId, {
                  playerRole: currentPlayer.role,
                });

                if (updatedInfo) {
                  console.log("privateGameInfo updated (gameInfo based):", updatedInfo);

                  addMessage({
                    id: Date.now().toString(),
                    sender: "System",
                    message: `Your role is "${currentPlayer.role}"`,
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                } else {
                  console.warn("Failed to update privateGameInfo. It may not be initialized.");

                  addMessage({
                    id: Date.now().toString(),
                    sender: "System",
                    message: "Failed to update role information. Please restart the game.",
                    timestamp: new Date().toISOString(),
                    type: "system",
                  });
                }
              } else {
                console.warn("Could not retrieve role information from gameInfo");
              }
            } else {
              console.warn("gameInfo is not available");
            }

            addMessage({
              id: Date.now().toString(),
              sender: "System",
              message: "Role assignment completed.",
              timestamp: new Date().toISOString(),
              type: "system",
            });
            break;
          case "winning_judge":
            setWinningJudgeResult(result.resultData);
            if (result.resultData.game_result !== "InProgress") {
              const resultMessage =
                result.resultData.game_result === "VillagerWin" ? "Villagers win!" : "Werewolves win!";
              addMessage({
                id: Date.now().toString(),
                sender: "System",
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
              sender: "System",
              message: `${result.resultData.executed_player_name} has been executed.`,
              timestamp: new Date().toISOString(),
              type: "system",
            });
            break;
          default:
            console.warn("Unknown computation type:", result.computationType);
        }
      } catch (error) {
        console.error("Computation result processing error:", error);
        addMessage({
          id: Date.now().toString(),
          sender: "System",
          message: `Computation result processing failed: ${result.computationType}`,
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
        console.log("Decrypting divination result as Seer");

        try {
          // ElGamal秘密鍵をJSONファイルから読み取り
          const secretKeyResponse = await fetch("/elgamal_secret_key.json");
          if (!secretKeyResponse.ok) {
            throw new Error("Failed to load ElGamal secret key");
          }

          const secretKeyText = await secretKeyResponse.text();
          const secretKey = JSONbigNative.parse(secretKeyText);

          console.log("Starting divination result decryption:", {
            ciphertext: data.ciphertext,
            secretKey: secretKey,
          });

          // TODO: Implement actual decryption process
          // Currently logging ciphertext and secret key instead of decryption logic
          console.log("Ciphertext:", data.ciphertext);
          console.log("Secret key:", secretKey);

          addMessage({
            id: Date.now().toString(),
            sender: "System",
            message: "Divination result decrypted. (Check console log for details)",
            timestamp: new Date().toISOString(),
            type: "system",
          });
        } catch (error) {
          console.error("Divination result decryption error:", error);
          addMessage({
            id: Date.now().toString(),
            sender: "System",
            message: `Divination result decryption failed: ${error}`,
            timestamp: new Date().toISOString(),
            type: "system",
          });
        }
      } else {
        addMessage({
          id: Date.now().toString(),
          sender: "System",
          message: "Divination result is ready.",
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
        sender: "System",
        message: "Role assignment completed.",
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
        const resultMessage = data.game_result === "VillagerWin" ? "Villagers win!" : "Werewolves win!";

        addMessage({
          id: Date.now().toString(),
          sender: "System",
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
        sender: "System",
        message: `${data.executed_player_name} has been executed.`,
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
        throw new Error("No divination result available");
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
          throw new Error(`Decryption failed: ${response.statusText}`);
        }

        const decryptedResult = await response.json();

        addMessage({
          id: Date.now().toString(),
          sender: "System",
          message: `Divination result: ${decryptedResult.is_werewolf ? "Werewolf" : "Not werewolf"}`,
          timestamp: new Date().toISOString(),
          type: "system",
        });

        return decryptedResult;
      } catch (error) {
        console.error("Decryption error:", error);
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
