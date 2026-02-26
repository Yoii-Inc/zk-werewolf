import { useEffect, useState } from "react";
import { getFortuneTellerSecretKey, loadCryptoParams } from "~~/services/gameInputGenerator";
import type { ChatMessage, PrivateGameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { CryptoManager } from "~~/utils/crypto/encryption";
import { getPrivateGameInfo, setPrivateGameInfo, updatePrivateGameInfo } from "~~/utils/privateGameInfoUtils";

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

interface PersistedDivinationLog {
  id: string;
  batchId: string;
  timestamp: string;
  message: string;
}

const getDivinationLogsKey = (roomId: string, playerId: string) => `divination_logs_${roomId}_${playerId}`;

const parseTimestamp = (timestamp: string) => {
  const value = new Date(timestamp).getTime();
  return Number.isNaN(value) ? 0 : value;
};

const loadDivinationLogs = (roomId: string, playerId: string): PersistedDivinationLog[] => {
  if (typeof window === "undefined" || !roomId || !playerId) return [];
  try {
    const stored = localStorage.getItem(getDivinationLogsKey(roomId, playerId));
    if (!stored) return [];
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      item =>
        item &&
        typeof item.id === "string" &&
        typeof item.batchId === "string" &&
        typeof item.timestamp === "string" &&
        typeof item.message === "string",
    );
  } catch (error) {
    console.error("Failed to load divination logs:", error);
    return [];
  }
};

const saveDivinationLogs = (roomId: string, playerId: string, logs: PersistedDivinationLog[]) => {
  if (typeof window === "undefined" || !roomId || !playerId) return;
  try {
    localStorage.setItem(getDivinationLogsKey(roomId, playerId), JSON.stringify(logs));
  } catch (error) {
    console.error("Failed to save divination logs:", error);
  }
};

const upsertDivinationLog = (roomId: string, playerId: string, log: PersistedDivinationLog) => {
  const current = loadDivinationLogs(roomId, playerId);
  const filtered = current.filter(existing => existing.batchId !== log.batchId);
  filtered.push(log);
  filtered.sort((a, b) => {
    const timeDiff = parseTimestamp(a.timestamp) - parseTimestamp(b.timestamp);
    if (timeDiff !== 0) return timeDiff;
    return a.id.localeCompare(b.id);
  });
  saveDivinationLogs(roomId, playerId, filtered);
};

const clearDivinationLogs = (roomId: string, playerId: string) => {
  if (typeof window === "undefined" || !roomId || !playerId) return;
  localStorage.removeItem(getDivinationLogsKey(roomId, playerId));
};

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

  useEffect(() => {
    if (!roomId || !playerId) return;
    const persisted = loadDivinationLogs(roomId, playerId).sort(
      (a, b) => parseTimestamp(a.timestamp) - parseTimestamp(b.timestamp),
    );

    persisted.forEach(log => {
      addMessage({
        id: log.id,
        sender: "System",
        message: log.message,
        timestamp: log.timestamp,
        type: "system",
      });
    });
  }, [addMessage, playerId, roomId]);

  useEffect(() => {
    if (!roomId || !playerId) return;

    const handleGameReset = () => {
      clearDivinationLogs(roomId, playerId);
    };

    window.addEventListener("gameResetNotification", handleGameReset);
    return () => {
      window.removeEventListener("gameResetNotification", handleGameReset);
    };
  }, [playerId, roomId]);

  // WebSocketからの計算結果通知を処理
  useEffect(() => {
    const handleComputationResult = async (event: Event) => {
      const customEvent = event as CustomEvent;
      const result: ComputationResult = customEvent.detail;

      console.log(`Computation result received: ${result.computationType}`, result);
      console.log(`Target player ID: ${result.targetPlayerId}, My player ID: ${playerId}`);

      // 対象プレイヤーのチェック（指定がある場合）
      if (result.targetPlayerId && result.targetPlayerId !== playerId) {
        console.log(`Skipping message not for me (target: ${result.targetPlayerId}, me: ${playerId})`);
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
                // KeyPublicize時に保存したElGamal秘密鍵を取得
                const secretKey = getFortuneTellerSecretKey(roomId, playerId);

                if (!secretKey) {
                  throw new Error("ElGamal secret key not found. Please complete KeyPublicize first.");
                }

                console.log("ElGamal secret key loaded from localStorage");

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

                // DivinationCircuitでは 0=default(), 1=prime_subgroup_generator() を平文として使う。
                // 現在の曲線は ed_on_bn254（旧BLS12-377値から置換）。
                const notWerewolf = {
                  x: [["0", "0", "0", "0"], null],
                  y: [
                    ["12436184717236109307", "3962172157175319849", "7381016538464732718", "1011752739694698287"],
                    null,
                  ],
                  _params: null,
                };

                const werewolf = {
                  x: [
                    ["15389767686415328915", "4532183014000888185", "6625844415766270035", "470379343721047487"],
                    null,
                  ],
                  y: [
                    ["10215293119099184011", "9361858917463510870", "15793394060027790616", "2556078677302762916"],
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

                // 占い対象のプレイヤー名を取得
                const targetPlayerId = localStorage.getItem(`divination_target_${roomId}`);
                const targetPlayerName = localStorage.getItem(`divination_target_name_${roomId}`);
                let targetName = targetPlayerName || "Unknown";
                // gameInfoから最新の名前も確認
                if (targetPlayerId && gameInfo?.players) {
                  const targetPlayer = gameInfo.players.find((p: any) => p.id === targetPlayerId);
                  if (targetPlayer) {
                    targetName = targetPlayer.name;
                  }
                }

                const divinationMessage = isWerewolf
                  ? `🐺 Divination result: ${targetName} is a Werewolf`
                  : `✅ Divination result: ${targetName} is not a Werewolf`;
                const divinationTimestamp =
                  typeof result.resultData?.performed_at === "string"
                    ? result.resultData.performed_at
                    : result.timestamp || new Date().toISOString();
                const divinationBatchId = result.batchId || `unknown-${divinationTimestamp}`;
                const divinationMessageId = `divination-${divinationBatchId}`;

                addMessage({
                  id: divinationMessageId,
                  sender: "System",
                  message: divinationMessage,
                  timestamp: divinationTimestamp,
                  type: "system",
                });
                upsertDivinationLog(roomId, playerId, {
                  id: divinationMessageId,
                  batchId: divinationBatchId,
                  timestamp: divinationTimestamp,
                  message: divinationMessage,
                });

                // 占い処理完了をグローバルイベントで通知
                window.dispatchEvent(new CustomEvent("divinationCompleted"));
                console.log("Divination completion event dispatched");
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

            // Role情報の復号処理
            try {
              console.log("Starting role assignment decryption process");
              console.log("Result data:", result.resultData);

              // 暗号化されたRoleデータを取得
              if (!result.resultData.encrypted_role) {
                throw new Error("No encrypted role data in result");
              }

              const { encrypted, nonce, node_id } = result.resultData.encrypted_role;

              if (!encrypted || !nonce || node_id === undefined) {
                throw new Error("Invalid encrypted role data structure");
              }

              // node_idからMPCノードの公開鍵を取得
              const MPC_NODE_PUBLIC_KEYS = [
                process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "",
                process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "",
                process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "",
              ];

              const sender_public_key = MPC_NODE_PUBLIC_KEYS[node_id];

              if (!sender_public_key) {
                throw new Error(`MPC node ${node_id} public key not configured`);
              }

              // CryptoManagerで復号
              const cryptoManager = new CryptoManager(playerId);

              if (!cryptoManager.hasKeyPair()) {
                throw new Error("No keypair found. Cannot decrypt role.");
              }

              console.log("Decrypting role with CryptoManager");
              console.log("Encrypted (first 50 chars):", encrypted.substring(0, 50));
              console.log("Nonce:", nonce);
              console.log("Sender public key (first 20 chars):", sender_public_key.substring(0, 20));

              // バイナリデータとして復号
              const decryptedBinary = cryptoManager.decryptBinary(encrypted, nonce, sender_public_key);

              console.log("Role decrypted successfully. Binary length:", decryptedBinary.length);

              // バイナリデータをUTF8文字列に変換
              const decoder = new TextDecoder("utf-8");
              const decryptedString = decoder.decode(decryptedBinary);

              console.log("Decrypted string:", decryptedString);

              // JSONとしてパース（Vec<String>形式を想定）
              let roleData: string[] | null = null;
              try {
                roleData = JSON.parse(decryptedString);
                console.log("Parsed role data:", roleData);
              } catch (parseError) {
                console.error("Failed to parse role data as JSON:", parseError);
                console.log("Raw data (first 200 chars):", decryptedString.substring(0, 200));
                throw new Error("Invalid role data format");
              }

              // roleDataから実際のRole情報を抽出
              // 修正後: 各プレイヤーには自分のRole IDのみが配列として送られる
              // 例: ["0000000000000000000000000000000000000000000000000000000000000002"]
              // 値はBigInt形式の16進数文字列で、0=Villager, 1=FortuneTeller, 2=Werewolf

              if (!roleData || roleData.length === 0) {
                throw new Error("Empty role data received");
              }

              // 配列の最初（唯一）の要素がこのプレイヤーのRole ID
              const roleIdStr = roleData[0];

              // 16進数文字列をBigIntとしてパース
              const roleIdBigInt = BigInt("0x" + roleIdStr);
              const roleId = roleIdBigInt % BigInt(3); // 0, 1, 2 のいずれか

              const ROLE_MAPPING: Record<string, string> = {
                "0": "Villager",
                "1": "Seer",
                "2": "Werewolf",
              };

              const roleName = ROLE_MAPPING[roleId.toString()] || "Unknown";

              console.log("Role ID:", roleId.toString(), "Role Name:", roleName);

              // 復号したRoleをprivateGameInfoに保存
              // まず既存の情報を確認し、なければ初期化してから更新
              let existingInfo = getPrivateGameInfo(roomId, playerId);

              if (!existingInfo) {
                console.log("PrivateGameInfo not found, initializing before role assignment");
                const initialInfo: PrivateGameInfo = {
                  playerId: playerId,
                  playerRole: null as any,
                  hasActed: false,
                };
                setPrivateGameInfo(roomId, initialInfo);
                existingInfo = initialInfo;
              }

              const updatedInfo = updatePrivateGameInfo(roomId, playerId, {
                playerRole: roleName as "Villager" | "Werewolf" | "Seer",
              });

              if (updatedInfo) {
                console.log("PrivateGameInfo updated successfully:", updatedInfo);
              } else {
                console.error("Failed to update PrivateGameInfo even after initialization attempt");
              }

              addMessage({
                id: Date.now().toString(),
                sender: "System",
                message: `Your role has been assigned: ${roleName} (from node ${node_id})`,
                timestamp: new Date().toISOString(),
                type: "system",
              });

              // Role割り当て完了イベントを発火
              window.dispatchEvent(new CustomEvent("roleAssignmentCompleted"));
              console.log("Role assignment completion event dispatched");
            } catch (error) {
              console.error("Role decryption error:", error);
              addMessage({
                id: Date.now().toString(),
                sender: "System",
                message: `Failed to decrypt role: ${error instanceof Error ? error.message : String(error)}`,
                timestamp: new Date().toISOString(),
                type: "system",
              });
            }
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

  return {
    divinationResult,
    roleAssignmentResult,
    winningJudgeResult,
    votingResult,
    isProcessing,
  };
};
