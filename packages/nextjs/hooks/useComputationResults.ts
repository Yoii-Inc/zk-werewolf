import { useCallback, useEffect, useRef, useState } from "react";
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
  encrypted_role_share?: {
    encrypted: string;
    nonce: string;
    node_id: number | string;
    required_shares: number | string;
    schema_version?: string;
    share_encoding?: string;
    role_share_encoding?: string;
    werewolf_mates_mask_share_encoding?: string;
  };
  player_order?: string[];
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

const divinationTargetIdByDayKey = (roomId: string, dayCount: number): string =>
  `divination_target_${roomId}_${dayCount}`;

const divinationTargetNameByDayKey = (roomId: string, dayCount: number): string =>
  `divination_target_name_${roomId}_${dayCount}`;

const latestDivinationTargetIdKey = (roomId: string): string => `divination_target_${roomId}`;

const latestDivinationTargetNameKey = (roomId: string): string => `divination_target_name_${roomId}`;

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

const BN254_SCALAR_MODULUS = BigInt("21888242871839275222246405745257275088548364400416034343698204186575808495617");

const normalizeFieldElement = (value: bigint): bigint => {
  const reduced = value % BN254_SCALAR_MODULUS;
  return reduced >= 0n ? reduced : reduced + BN254_SCALAR_MODULUS;
};

const decodeRoleName = (roleId: bigint): "Villager" | "Seer" | "Werewolf" => {
  const normalized = normalizeFieldElement(roleId) % 3n;
  if (normalized === 1n) return "Seer";
  if (normalized === 2n) return "Werewolf";
  return "Villager";
};

const decodeWerewolfTeammateIds = (
  maskValue: bigint,
  playerOrderIds: string[] | undefined,
  myPlayerId: string,
  myRole: "Villager" | "Seer" | "Werewolf",
): string[] => {
  if (myRole !== "Werewolf" || !playerOrderIds || playerOrderIds.length === 0) {
    return [];
  }

  const normalizedMask = normalizeFieldElement(maskValue);
  const teammateIds: string[] = [];
  for (let index = 0; index < playerOrderIds.length; index += 1) {
    const bit = (normalizedMask >> BigInt(index)) & 1n;
    if (bit !== 1n) continue;
    const teammateId = playerOrderIds[index];
    if (!teammateId || teammateId === myPlayerId) continue;
    teammateIds.push(teammateId);
  }
  return Array.from(new Set(teammateIds));
};

const parsePlayerOrderIds = (raw: unknown): string[] | undefined => {
  if (!Array.isArray(raw)) return undefined;
  const ids = raw.map(id => String(id)).filter(id => id.length > 0);
  return ids.length > 0 ? ids : undefined;
};

interface NormalizedDivinationPoint {
  x: [string, string, string, string];
  y: [string, string, string, string];
}

const NOT_WEREWOLF_POINT: NormalizedDivinationPoint = {
  x: ["0", "0", "0", "0"],
  y: ["12436184717236109307", "3962172157175319849", "7381016538464732718", "1011752739694698287"],
};

const WEREWOLF_POINT: NormalizedDivinationPoint = {
  x: ["15389767686415328915", "4532183014000888185", "6625844415766270035", "470379343721047487"],
  y: ["10215293119099184011", "9361858917463510870", "15793394060027790616", "2556078677302762916"],
};

const normalizeDivinationLimb = (value: unknown): string | null => {
  if (typeof value === "string") return value;
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "number" && Number.isFinite(value)) return Math.trunc(value).toString();
  return null;
};

const normalizeDivinationField = (value: unknown): [string, string, string, string] | null => {
  if (!Array.isArray(value) || value.length === 0 || !Array.isArray(value[0]) || value[0].length < 4) {
    return null;
  }
  const normalized = value[0].slice(0, 4).map(normalizeDivinationLimb);
  if (normalized.some(limb => limb === null)) {
    return null;
  }
  return normalized as [string, string, string, string];
};

const normalizeDivinationPoint = (value: unknown): NormalizedDivinationPoint | null => {
  if (!value || typeof value !== "object") return null;
  const candidate = value as { x?: unknown; y?: unknown };
  const x = normalizeDivinationField(candidate.x);
  const y = normalizeDivinationField(candidate.y);
  if (!x || !y) return null;
  return { x, y };
};

const isSameDivinationPoint = (lhs: NormalizedDivinationPoint, rhs: NormalizedDivinationPoint): boolean =>
  lhs.x.every((value, index) => value === rhs.x[index]) && lhs.y.every((value, index) => value === rhs.y[index]);

export const resolveDivinationWerewolfFlag = (decryptedResult: unknown): boolean | null => {
  const normalized = normalizeDivinationPoint(decryptedResult);
  if (!normalized) return null;
  if (isSameDivinationPoint(normalized, NOT_WEREWOLF_POINT)) return false;
  if (isSameDivinationPoint(normalized, WEREWOLF_POINT)) return true;
  return null;
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
  const roleShareBuffersRef = useRef<
    Map<
      string,
      {
        requiredShares: number;
        roleSharesByNode: Map<number, bigint>;
        werewolfMaskSharesByNode: Map<number, bigint>;
        playerOrderIds?: string[];
      }
    >
  >(new Map());
  const completedRoleBatchRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    roleShareBuffersRef.current.clear();
    completedRoleBatchRef.current.clear();
  }, [playerId, roomId]);

  const finalizeRoleAssignmentBatch = useCallback(
    (
      batchKey: string,
      buffer: {
        requiredShares: number;
        roleSharesByNode: Map<number, bigint>;
        werewolfMaskSharesByNode: Map<number, bigint>;
        playerOrderIds?: string[];
      },
    ): boolean => {
      if (completedRoleBatchRef.current.has(batchKey)) {
        return true;
      }
      if (buffer.roleSharesByNode.size < buffer.requiredShares) {
        return false;
      }

      const fallbackPlayerOrder = Array.isArray(gameInfo?.players)
        ? gameInfo.players.map((player: any) => String(player.id))
        : undefined;
      const playerOrderIds = buffer.playerOrderIds ?? fallbackPlayerOrder;
      if (!playerOrderIds || playerOrderIds.length === 0) {
        return false;
      }

      let combinedRoleShare = 0n;
      for (const share of buffer.roleSharesByNode.values()) {
        combinedRoleShare = normalizeFieldElement(combinedRoleShare + share);
      }

      let combinedWerewolfMaskShare = 0n;
      for (const share of buffer.werewolfMaskSharesByNode.values()) {
        combinedWerewolfMaskShare = normalizeFieldElement(combinedWerewolfMaskShare + share);
      }

      const roleName = decodeRoleName(combinedRoleShare);
      const werewolfTeammateIds = decodeWerewolfTeammateIds(
        combinedWerewolfMaskShare,
        playerOrderIds,
        playerId,
        roleName,
      );

      const existingInfo = getPrivateGameInfo(roomId, playerId);
      if (!existingInfo) {
        const initialInfo: PrivateGameInfo = {
          playerId,
          playerRole: null as any,
          werewolfTeammateIds: [],
          hasActed: false,
        };
        setPrivateGameInfo(roomId, initialInfo);
      }

      const updatedInfo = updatePrivateGameInfo(roomId, playerId, {
        playerRole: roleName as "Villager" | "Werewolf" | "Seer",
        werewolfTeammateIds,
      });

      if (!updatedInfo) {
        console.error("Failed to update PrivateGameInfo even after initialization attempt");
        return false;
      }

      completedRoleBatchRef.current.add(batchKey);
      roleShareBuffersRef.current.delete(batchKey);

      const werewolfTeammateNames = werewolfTeammateIds
        .map(id => gameInfo?.players?.find((player: any) => String(player.id) === String(id))?.name || id)
        .filter((name, index, self) => self.indexOf(name) === index);

      addMessage({
        id: Date.now().toString(),
        sender: "System",
        message:
          roleName === "Werewolf" && werewolfTeammateNames.length > 0
            ? `Your role has been assigned: ${roleName} (teammates: ${werewolfTeammateNames.join(", ")})`
            : `Your role has been assigned: ${roleName}`,
        timestamp: new Date().toISOString(),
        type: "system",
      });

      window.dispatchEvent(new CustomEvent("roleAssignmentCompleted"));
      return true;
    },
    [addMessage, gameInfo?.players, playerId, roomId],
  );

  useEffect(() => {
    if (!playerId || !roomId) return;
    for (const [batchKey, buffer] of roleShareBuffersRef.current.entries()) {
      finalizeRoleAssignmentBatch(batchKey, buffer);
    }
  }, [finalizeRoleAssignmentBatch, playerId, roomId]);

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
                const isWerewolf = resolveDivinationWerewolfFlag(decryptedResult);
                if (isWerewolf === null) {
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
                const resultDayCountRaw = result.resultData?.day_count;
                const resultDayCount =
                  typeof resultDayCountRaw === "number"
                    ? resultDayCountRaw
                    : Number.parseInt(String(resultDayCountRaw ?? ""), 10);

                const targetPlayerIdByDay = Number.isFinite(resultDayCount)
                  ? localStorage.getItem(divinationTargetIdByDayKey(roomId, resultDayCount))
                  : null;
                const targetPlayerNameByDay = Number.isFinite(resultDayCount)
                  ? localStorage.getItem(divinationTargetNameByDayKey(roomId, resultDayCount))
                  : null;

                const targetPlayerId = targetPlayerIdByDay || localStorage.getItem(latestDivinationTargetIdKey(roomId));
                const targetPlayerName =
                  targetPlayerNameByDay || localStorage.getItem(latestDivinationTargetNameKey(roomId));

                let targetName = targetPlayerName || "Unknown";
                // gameInfoから最新の名前も確認
                if (targetPlayerId && gameInfo?.players) {
                  const targetPlayer = gameInfo.players.find((p: any) => String(p.id) === String(targetPlayerId));
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

            try {
              const shareData = result.resultData?.encrypted_role_share;
              if (!shareData) {
                throw new Error("No encrypted role share found in role_assignment result");
              }

              const {
                encrypted,
                nonce,
                node_id: nodeIdRaw,
                required_shares: requiredSharesRaw,
                share_encoding: shareEncoding,
                role_share_encoding: roleShareEncodingRaw,
                werewolf_mates_mask_share_encoding: werewolfMaskEncodingRaw,
              } = shareData;

              const nodeId = typeof nodeIdRaw === "string" ? Number(nodeIdRaw) : nodeIdRaw;
              const requiredShares =
                typeof requiredSharesRaw === "string" ? Number(requiredSharesRaw) : requiredSharesRaw;
              const roleShareEncoding = roleShareEncodingRaw ?? shareEncoding;
              const werewolfMaskEncoding = werewolfMaskEncodingRaw ?? "player_index_bitmask_lsb0";

              if (!encrypted || !nonce || !Number.isFinite(nodeId)) {
                throw new Error("Invalid encrypted role share payload");
              }
              if (!Number.isFinite(requiredShares) || requiredShares <= 0) {
                throw new Error(`Invalid required_shares value: ${String(requiredSharesRaw)}`);
              }
              if (roleShareEncoding && roleShareEncoding !== "bn254_fr_decimal_string") {
                throw new Error(`Unsupported role share encoding: ${roleShareEncoding}`);
              }
              if (werewolfMaskEncoding && werewolfMaskEncoding !== "player_index_bitmask_lsb0") {
                throw new Error(`Unsupported werewolf mask encoding: ${werewolfMaskEncoding}`);
              }

              const MPC_NODE_PUBLIC_KEYS = [
                process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "",
                process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "",
                process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "",
              ];

              const senderPublicKey = MPC_NODE_PUBLIC_KEYS[nodeId];

              if (!senderPublicKey) {
                throw new Error(`MPC node ${nodeId} public key not configured`);
              }

              const cryptoManager = new CryptoManager(playerId);
              if (!cryptoManager.hasKeyPair()) {
                throw new Error("No keypair found. Cannot decrypt role share.");
              }

              const batchKey = `${result.batchId}:${playerId}`;
              if (completedRoleBatchRef.current.has(batchKey)) {
                break;
              }

              const playerOrderIds = parsePlayerOrderIds(result.resultData?.player_order);

              const decryptedBinary = cryptoManager.decryptBinary(encrypted, nonce, senderPublicKey);
              const decoder = new TextDecoder("utf-8");
              const decryptedString = decoder.decode(decryptedBinary);

              let roleShareString: string | null = null;
              let werewolfMatesMaskShareString = "0";
              try {
                const parsed = JSON.parse(decryptedString) as {
                  role_share?: string;
                  werewolf_mates_mask_share?: string;
                  role_share_encoding?: string;
                  werewolf_mates_mask_share_encoding?: string;
                };
                if (parsed && typeof parsed.role_share === "string") {
                  roleShareString = parsed.role_share;
                }
                if (parsed && typeof parsed.werewolf_mates_mask_share === "string") {
                  werewolfMatesMaskShareString = parsed.werewolf_mates_mask_share;
                }

                if (parsed?.role_share_encoding && parsed.role_share_encoding !== "bn254_fr_decimal_string") {
                  throw new Error(`Unsupported decrypted role share encoding: ${parsed.role_share_encoding}`);
                }
                if (
                  parsed?.werewolf_mates_mask_share_encoding &&
                  parsed.werewolf_mates_mask_share_encoding !== "player_index_bitmask_lsb0"
                ) {
                  throw new Error(
                    `Unsupported decrypted werewolf mask encoding: ${parsed.werewolf_mates_mask_share_encoding}`,
                  );
                }
              } catch (parseError) {
                throw new Error(
                  `Invalid role share payload: ${parseError instanceof Error ? parseError.message : String(parseError)}`,
                );
              }
              if (!roleShareString) {
                throw new Error("Missing role_share in decrypted payload");
              }

              const roleShare = normalizeFieldElement(BigInt(roleShareString));
              const werewolfMatesMaskShare = normalizeFieldElement(BigInt(werewolfMatesMaskShareString));
              const existingBuffer = roleShareBuffersRef.current.get(batchKey) ?? {
                requiredShares,
                roleSharesByNode: new Map<number, bigint>(),
                werewolfMaskSharesByNode: new Map<number, bigint>(),
                playerOrderIds,
              };
              existingBuffer.requiredShares = Math.max(existingBuffer.requiredShares, requiredShares);
              if (playerOrderIds && playerOrderIds.length > 0) {
                existingBuffer.playerOrderIds = playerOrderIds;
              }
              if (existingBuffer.roleSharesByNode.has(nodeId)) {
                roleShareBuffersRef.current.set(batchKey, existingBuffer);
                finalizeRoleAssignmentBatch(batchKey, existingBuffer);
                break;
              }
              existingBuffer.roleSharesByNode.set(nodeId, roleShare);
              existingBuffer.werewolfMaskSharesByNode.set(nodeId, werewolfMatesMaskShare);
              roleShareBuffersRef.current.set(batchKey, existingBuffer);

              finalizeRoleAssignmentBatch(batchKey, existingBuffer);
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
  }, [playerId, addMessage, roomId, gameInfo, finalizeRoleAssignmentBatch]);

  return {
    divinationResult,
    roleAssignmentResult,
    winningJudgeResult,
    votingResult,
    isProcessing,
  };
};
