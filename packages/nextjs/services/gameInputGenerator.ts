import JSONbig from "json-bigint";
import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import {
  AnonymousVotingInput,
  AnonymousVotingPrivateInput,
  AnonymousVotingPublicInput,
  DivinationInput,
  DivinationPrivateInput,
  DivinationPublicInput,
  NodeKey,
  PedersenCommitment,
  PedersenParam,
  RoleAssignmentInput,
  RoleAssignmentPrivateInput,
  RoleAssignmentPublicInput,
  SecretSharingScheme,
  WinningJudgementInput,
  WinningJudgementPrivateInput,
  WinningJudgementPublicInput,
} from "~~/utils/crypto/type";

const JSONbigNative = JSONbig({ useNativeBigInt: true });

// ============================================================================
// グローバルキャッシュ
// ============================================================================

// 暗号パラメータのキャッシュ（アプリ全体で1つ）
let cryptoParamsCache: any | null = null;

// ランダムネスのキャッシュ（ルーム×ユーザーごと）
const randomnessCache = new Map<string, bigint[]>();

// ============================================================================
// 暗号パラメータ管理
// ============================================================================

/**
 * 暗号パラメータを読み込む（キャッシュあり）
 * 一度読み込んだらアプリ全体で再利用される
 */
export async function loadCryptoParams(): Promise<any> {
  if (cryptoParamsCache) {
    console.log("Using cached crypto params");
    return cryptoParamsCache;
  }

  console.log("Loading crypto params from static files...");

  // 並列で全ファイルを取得
  const [pedersenRes, commitRes, randRes, elgamalParamRes, elgamalPubkeyRes] = await Promise.all([
    fetch("/pedersen_params2.json"),
    fetch("/pedersen_commitment_0.json"),
    fetch("/pedersen_randomness_0.json"),
    fetch("/elgamal_params.json"),
    fetch("/elgamal_public_key.json"),
  ]);

  const [pedersenParams, commitment, randomness, elgamalParam, elgamalPubkey] = await Promise.all([
    pedersenRes.text(),
    commitRes.text(),
    randRes.text(),
    elgamalParamRes.text(),
    elgamalPubkeyRes.text(),
  ]);

  cryptoParamsCache = {
    pedersenParam: JSONbigNative.parse(pedersenParams),
    pedersenCommitment: JSONbigNative.parse(commitment),
    pedersenRandomness: JSONbigNative.parse(randomness),
    elgamalParam: JSONbigNative.parse(elgamalParam),
    elgamalPublicKey: JSONbigNative.parse(elgamalPubkey),
  };

  console.log("Crypto params loaded successfully");
  return cryptoParamsCache;
}

/**
 * 暗号パラメータのキャッシュをクリア（テスト用）
 */
export function clearCryptoParamsCache(): void {
  cryptoParamsCache = null;
}

// ============================================================================
// ランダムネス管理
// ============================================================================

/**
 * プレイヤーのランダムネスを取得（キャッシュあり）
 * LocalStorageから読み込み、なければ新規生成
 */
function getRandomness(roomId: string, username: string): bigint[] {
  const cacheKey = `${roomId}_${username}`;

  // メモリキャッシュをチェック
  if (randomnessCache.has(cacheKey)) {
    return randomnessCache.get(cacheKey)!;
  }

  // LocalStorageから読み込み
  const storageKey = `randomness_${roomId}_${username}`;
  const stored = localStorage.getItem(storageKey);

  if (stored) {
    try {
      const randomness = JSONbigNative.parse(stored);
      randomnessCache.set(cacheKey, randomness);
      console.log("Loaded randomness from localStorage");
      return randomness;
    } catch (error) {
      console.warn("Failed to parse stored randomness, generating new one");
    }
  }

  // 新規生成（TODO: WASMのgeneratePedersenRandomness()を使用）
  const randomness = [
    BigInt(Math.floor(Math.random() * 1000000000)),
    BigInt(Math.floor(Math.random() * 1000000000)),
    BigInt(Math.floor(Math.random() * 1000000000)),
    BigInt(Math.floor(Math.random() * 1000000000)),
  ];

  // キャッシュに保存
  randomnessCache.set(cacheKey, randomness);

  // LocalStorageに保存
  try {
    localStorage.setItem(storageKey, JSONbigNative.stringify(randomness));
    console.log("Generated and saved new randomness");
  } catch (error) {
    console.error("Failed to save randomness to localStorage:", error);
  }

  return randomness;
}

/**
 * ランダムネスのキャッシュをクリア（テスト用）
 */
export function clearRandomnessCache(roomId?: string, username?: string): void {
  if (roomId && username) {
    const cacheKey = `${roomId}_${username}`;
    randomnessCache.delete(cacheKey);
  } else {
    randomnessCache.clear();
  }
}

// ============================================================================
// ヘルパー関数
// ============================================================================

function getMyPlayerId(gameInfo: GameInfo, username: string): number {
  return gameInfo.players.findIndex(p => p.name === username);
}

function getMyRole(gameInfo: GameInfo, username: string): string | null {
  const player = gameInfo.players.find(p => p.name === username);
  return player?.role || null;
}

function isWerewolf(gameInfo: GameInfo, username: string): boolean {
  return getMyRole(gameInfo, username) === "Werewolf";
}

function getNodeKeys(): NodeKey[] {
  return [
    { nodeId: "0", publicKey: process.env.NEXT_PUBLIC_MPC_NODE0_PUBLIC_KEY || "" },
    { nodeId: "1", publicKey: process.env.NEXT_PUBLIC_MPC_NODE1_PUBLIC_KEY || "" },
    { nodeId: "2", publicKey: process.env.NEXT_PUBLIC_MPC_NODE2_PUBLIC_KEY || "" },
  ];
}

function getScheme(): SecretSharingScheme {
  return { totalShares: 3, modulus: 97 };
}

// ============================================================================
// コミットメント送信
// ============================================================================

/**
 * コミットメントをサーバーに送信
 */
async function submitCommitment(roomId: string, playerId: number, randomness: bigint[]): Promise<void> {
  try {
    // TODO: WASMのcomputePedersenCommitment()を使用
    const dummyCommitment = {
      player_id: playerId,
      commitment: randomness.map(r => r.toString()),
      created_at: new Date().toISOString(),
    };

    const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/game/${roomId}/commitment`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(dummyCommitment),
    });

    if (!response.ok) {
      throw new Error(`Failed to submit commitment: ${response.statusText}`);
    }

    console.log("Commitment submitted successfully");
  } catch (error) {
    console.error("Error submitting commitment:", error);
    throw error;
  }
}

// ============================================================================
// 初期化
// ============================================================================

/**
 * ゲーム暗号化の初期化
 * 暗号パラメータのロードとランダムネスの生成・コミットメント送信を行う
 */
export async function initializeGameCrypto(roomId: string, username: string, gameInfo: GameInfo): Promise<void> {
  console.log("Initializing game crypto...");

  // 暗号パラメータをロード
  await loadCryptoParams();

  // ランダムネスを取得（既存があればそれを使用、なければ生成）
  const randomness = getRandomness(roomId, username);

  // プレイヤーIDを取得
  const playerId = getMyPlayerId(gameInfo, username);

  // コミットメントを送信
  await submitCommitment(roomId, playerId, randomness);

  console.log("Game crypto initialized successfully");
}

/**
 * 初期化済みかどうかを確認
 */
export function isInitialized(roomId: string, username: string): boolean {
  const cacheKey = `${roomId}_${username}`;
  return cryptoParamsCache !== null && randomnessCache.has(cacheKey);
}

// ============================================================================
// 入力生成関数
// ============================================================================

/**
 * 役職配布用の入力を生成
 */
export async function generateRoleAssignmentInput(
  roomId: string,
  username: string,
  gameInfo: GameInfo,
): Promise<RoleAssignmentInput> {
  const cryptoParams = await loadCryptoParams();
  const myId = getMyPlayerId(gameInfo, username);

  // テストデータを読み込み
  const rinputRes = await fetch("/test_role_assignment_input2.json");
  const rinput = await rinputRes.text();
  const parsedRinput: RoleAssignmentInput = JSONbigNative.parse(rinput);
  parsedRinput.privateInput.id = myId;

  return {
    privateInput: parsedRinput.privateInput,
    publicInput: parsedRinput.publicInput,
    nodeKeys: getNodeKeys(),
    scheme: getScheme(),
  };
}

/**
 * 占い用の入力を生成
 */
export async function generateDivinationInput(
  roomId: string,
  username: string,
  gameInfo: GameInfo,
  targetId: string,
  isDummy: boolean,
): Promise<DivinationInput> {
  const cryptoParams = await loadCryptoParams();
  const myId = getMyPlayerId(gameInfo, username);

  // 占い用の新規ランダムネス生成（1回の占いごとに新しいもの）
  // TODO: WASMのgeneratePedersenRandomness()を使用
  const divinationRandomness = [JSONbigNative.parse('["0","0","0","0"]'), null];

  // 実際のゲーム状態を使用
  const isWerewolfValue = isWerewolf(gameInfo, username)
    ? [
        JSONbigNative.parse('["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]'),
        null,
      ]
    : [JSONbigNative.parse('["0","0","0","0"]'), null];

  const privateInput: DivinationPrivateInput =
    isDummy === false
      ? {
          id: myId,
          isTarget: gameInfo.players.map((player: any) => [
            player.id === targetId
              ? JSONbigNative.parse(
                  '["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]',
                )
              : JSONbigNative.parse('["0","0","0","0"]'),
            null,
          ]),
          isWerewolf: isWerewolfValue,
          randomness: divinationRandomness,
        }
      : {
          id: myId,
          isTarget: gameInfo.players.map(() => [JSONbigNative.parse('["0","0","0","0"]'), null]),
          isWerewolf: isWerewolfValue,
          randomness: divinationRandomness,
        };

  const publicInput: DivinationPublicInput = {
    pedersenParam: cryptoParams.pedersenParam,
    elgamalParam: cryptoParams.elgamalParam || {},
    pubKey: cryptoParams.elgamalPublicKey || {},
    playerNum: gameInfo.players.length,
  };

  return {
    privateInput,
    publicInput,
    nodeKeys: getNodeKeys(),
    scheme: getScheme(),
  };
}

/**
 * 投票用の入力を生成
 */
export async function generateVotingInput(
  roomId: string,
  username: string,
  gameInfo: GameInfo,
  votedForId: string,
): Promise<AnonymousVotingInput> {
  const cryptoParams = await loadCryptoParams();
  const randomness = getRandomness(roomId, username);
  const myId = getMyPlayerId(gameInfo, username);

  // MPC公開鍵の確認
  const nodeKeys = getNodeKeys();
  if (nodeKeys.length !== 3 || nodeKeys.some(key => !key.publicKey)) {
    throw new Error("MPC node public keys are not properly configured");
  }

  // bigint[]を(number[] | null)[]に変換
  //   const randomnessForVoting = randomness.map(r => Array.from(r.toString().split("")).map(Number));
  const randomnessForVoting = [JSONbigNative.parse('["0","0","0","0"]'), null];

  const privateInput: AnonymousVotingPrivateInput = {
    id: myId,
    isTargetId: gameInfo.players.map((player: any) =>
      player.id === votedForId
        ? [
            JSONbigNative.parse(
              '["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]',
            ),
            null,
          ]
        : [JSONbigNative.parse('["0","0","0","0"]'), null],
    ),
    playerRandomness: randomnessForVoting,
  };

  const publicInput: AnonymousVotingPublicInput = {
    pedersenParam: cryptoParams.pedersenParam,
    playerCommitment: Array(gameInfo.players.length).fill(cryptoParams.pedersenCommitment as PedersenCommitment),
    playerNum: gameInfo.players.length,
  };

  return {
    privateInput,
    publicInput,
    nodeKeys,
    scheme: getScheme(),
  };
}

/**
 * 投票データの暗号化
 */
export async function encryptVotingData(
  roomId: string,
  username: string,
  gameInfo: GameInfo,
  votedForId: string,
): Promise<any> {
  const input = await generateVotingInput(roomId, username, gameInfo, votedForId);
  return await MPCEncryption.encryptAnonymousVoting(input);
}

/**
 * 勝敗判定用の入力を生成
 */
export async function generateWinningJudgementInput(
  roomId: string,
  username: string,
  gameInfo: GameInfo,
): Promise<WinningJudgementInput> {
  const cryptoParams = await loadCryptoParams();
  const randomness = getRandomness(roomId, username);
  const myId = getMyPlayerId(gameInfo, username);

  // 実際の役職情報を使用
  const amWerewolfValues = isWerewolf(gameInfo, username)
    ? JSONbigNative.parse('["9015221291577245683", "8239323489949974514", "1646089257421115374", "958099254763297437"]')
    : JSONbigNative.parse('["0", "0", "0", "0"]');

  // bigint[]を(bigint[] | null)[]に変換
  //   const randomnessArray: (bigint[] | null)[] = [[...randomness], null];
  const randres = await fetch("/pedersen_randomness_0.json");
  const randomnessjson = await randres.text();
  const randomnessArray = JSONbigNative.parse(randomnessjson);

  const privateInput: WinningJudgementPrivateInput = {
    id: myId,
    amWerewolf: [amWerewolfValues, null],
    playerRandomness: randomnessArray,
  };

  const publicInput: WinningJudgementPublicInput = {
    pedersenParam: cryptoParams.pedersenParam,
    playerCommitment: Array(gameInfo.players.length).fill(cryptoParams.pedersenCommitment),
  };

  return {
    privateInput,
    publicInput,
    nodeKeys: getNodeKeys(),
    scheme: getScheme(),
  };
}
