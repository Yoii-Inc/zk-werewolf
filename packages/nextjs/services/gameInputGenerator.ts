import JSONbig from "json-bigint";
import { GameInfo } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import { CryptoManager } from "~~/utils/crypto/encryption";
import {
  AnonymousVotingInput,
  AnonymousVotingPrivateInput,
  AnonymousVotingPublicInput,
  DivinationInput,
  DivinationPrivateInput,
  DivinationPublicInput,
  Field,
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
// 有限体の定数
// ============================================================================

// 有限体のゼロ要素: [0, 0, 0, 0]
const FINITE_FIELD_ZERO: Field[] = [JSONbigNative.parse('["0","0","0","0"]'), null] as const;

// 有限体のone要素
const FINITE_FIELD_ONE: Field[] = [
  JSONbigNative.parse('["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]'),
  null,
] as const;

// ============================================================================
// グローバルキャッシュ
// ============================================================================

// 暗号パラメータのキャッシュ（アプリ全体で1つ）
let cryptoParamsCache: any | null = null;

// ランダムネスのキャッシュ（ルーム×ユーザーごと）
const randomnessCache = new Map<string, Field[]>();

// ============================================================================
// 暗号パラメータ管理
// ============================================================================

/**
 * 暗号パラメータを読み込む（キャッシュあり）
 * gameInfo.cryptoParametersから取得し、なければ静的ファイルからフォールバック
 * 一度読み込んだらアプリ全体で再利用される
 */
export async function loadCryptoParams(gameInfo?: GameInfo): Promise<any> {
  if (cryptoParamsCache) {
    console.log("Using cached crypto params");
    // If gameInfo has updated player commitments, refresh that piece of cache
    const gp = gameInfo?.crypto_parameters;
    if (gp && gp.player_commitment) {
      cryptoParamsCache.playerCommitments = gp.player_commitment;
      // keep pedersenCommitment in sync with first element if available
      cryptoParamsCache.pedersenCommitment = gp.player_commitment?.[0] ?? cryptoParamsCache.pedersenCommitment;
      console.log("Updated cached playerCommitments from gameInfo");
    }
    return cryptoParamsCache;
  }

  // gameInfo.cryptoParametersがあればそれを使用
  const gp = gameInfo?.crypto_parameters;
  if (gp) {
    console.log("Loading crypto params from gameInfo.crypto_parameters...");

    cryptoParamsCache = {
      pedersenParam: gp.pedersen_param,
      pedersenCommitment: gp.player_commitment?.[0],
      pedersenRandomness: null, // ランダムネスは別途管理
      elgamalParam: gp.elgamal_param,
      elgamalPublicKey: gp.fortune_teller_public_key,
      playerCommitments: gp.player_commitment,
    };

    console.log("Crypto params loaded successfully from gameInfo");
    return cryptoParamsCache;
  }

  // フォールバック: 静的ファイルから読み込み
  console.log("gameInfo.crypto_parameters not found, falling back to static files...");

  const [pedersenRes, commitRes, elgamalParamRes, elgamalPubkeyRes] = await Promise.all([
    fetch("/pedersen_params2.json"),
    fetch("/pedersen_commitment_0.json"),
    fetch("/elgamal_params.json"),
    fetch("/elgamal_public_key.json"),
  ]);

  const [pedersenParams, commitment, elgamalParam, elgamalPubkey] = await Promise.all([
    pedersenRes.text(),
    commitRes.text(),
    elgamalParamRes.text(),
    elgamalPubkeyRes.text(),
  ]);

  cryptoParamsCache = {
    pedersenParam: JSONbigNative.parse(pedersenParams),
    pedersenCommitment: JSONbigNative.parse(commitment),
    pedersenRandomness: null,
    elgamalParam: JSONbigNative.parse(elgamalParam),
    elgamalPublicKey: JSONbigNative.parse(elgamalPubkey),
  };

  console.log("Crypto params loaded successfully from static files");
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
async function getRandomness(roomId: string, username: string): Promise<Field[]> {
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
  const randomness = await MPCEncryption.frRand();
  console.log("Generated new randomness:", randomness);

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

/**
 * 特定のルームのLocalStorageに保存されているランダムネスをクリア
 */
export function clearRandomnessFromStorage(roomId: string): void {
  // LocalStorageから該当ルームのランダムネスを削除
  const keys = Object.keys(localStorage);
  keys.forEach(key => {
    if (key.startsWith(`randomness_${roomId}_`)) {
      localStorage.removeItem(key);
      console.log(`Cleared randomness from localStorage: ${key}`);
    }
  });
}

/**
 * ゲームリセット時の全クライアント初期化状態クリア
 */
export function resetGameCryptoState(roomId: string): void {
  console.log(`Resetting game crypto state for room: ${roomId}`);

  // 暗号パラメータキャッシュをクリア
  clearCryptoParamsCache();

  // ランダムネスメモリキャッシュをクリア
  clearRandomnessCache();

  // LocalStorageから該当ルームのランダムネスを削除
  clearRandomnessFromStorage(roomId);

  console.log(`Game crypto state reset completed for room: ${roomId}`);
}

// ============================================================================
// ヘルパー関数
// ============================================================================

function getMyPlayerIndex(gameInfo: GameInfo, username: string): number {
  return gameInfo.players.findIndex(p => p.name === username);
}

function getMyPlayerId(gameInfo: GameInfo, username: string): string | null {
  const player = gameInfo.players.find(p => p.name === username);
  return player ? player.id : null;
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

/**
 * 個別のシャッフル行列を生成する（Rustのgenerate_individual_shuffle_matrixのTypeScript版）
 * 行列は行優先（row-major）で平坦化した配列として返す。
 * F::one() に相当する値は `FINITE_FIELD_ONE` を使用する。
 */
export function generateIndividualShuffleMatrix(n: number, m: number, rng?: () => number): Field[][] {
  const size = n + m;
  const total = size * size;

  // permutation 0..n-1 を生成してシャッフル
  const permutation: number[] = Array.from({ length: n }, (_, i) => i);
  for (let i = permutation.length - 1; i > 0; i--) {
    const r = rng ? rng() : Math.random();
    const j = Math.floor(r * (i + 1));
    const tmp = permutation[i];
    permutation[i] = permutation[j];
    permutation[j] = tmp;
  }

  // 平坦化された行列をゼロ要素で初期化
  const mat: Field[][] = new Array(size);
  for (let idx = 0; idx < total; idx++) {
    mat[idx] = FINITE_FIELD_ZERO;
  }

  // シャッフル部分: for i in 0..n set (i, permutation[i]) = ONE
  for (let i = 0; i < n; i++) {
    const row = i;
    const col = permutation[i];
    mat[row * size + col] = FINITE_FIELD_ONE;
  }

  // 固定部分: for i in n..n+m set (i,i) = ONE
  for (let i = n; i < n + m; i++) {
    mat[i * size + i] = FINITE_FIELD_ONE;
  }

  return mat;
}

/**
 * WASM に渡す形式（JSONbig.parse と同じ形）で生成するヘルパー
 * 返り値は [matrixFlatArray, size, size] の形になります。
 */
export function generateShuffleMatricesForWasm(n: number, m: number, rng?: () => number): [Field[][], number, number] {
  const mat = generateIndividualShuffleMatrix(n, m, rng);
  const size = n + m;
  return [mat, size, size];
}

/**
 * tau matrix を WASM/JSONbig と同じ形式で生成する。
 * groupingParameter の走査順はオブジェクトの列挙順に従う。
 */
export function generateTauMatrixForWasm(groupingParameter: any, numPlayers: number): [Field[], number, number] {
  // compute num_groups according to Rust logic
  let numGroups = 0;
  for (const key of Object.keys(groupingParameter)) {
    const [count, isNotAlone] = groupingParameter[key];
    if (isNotAlone) numGroups += 1;
    else numGroups += count;
  }

  const size = numPlayers + numGroups;
  const total = size * size;

  const mat: any[] = new Array(total);
  for (let i = 0; i < total; i++) mat[i] = FINITE_FIELD_ZERO;

  let playerIndex = 0;
  let groupIndex = 0;

  for (const key of Object.keys(groupingParameter)) {
    const [count, isNotAlone] = groupingParameter[key];
    if (isNotAlone) {
      if (count < 2) throw new Error("not alone group count must be >= 2");

      // group
      mat[playerIndex * size + (numPlayers + groupIndex)] = FINITE_FIELD_ONE;

      // players (chain)
      for (let k = 0; k < count - 1; k++) {
        mat[(playerIndex + 1) * size + playerIndex] = FINITE_FIELD_ONE;
        playerIndex += 1;
      }

      mat[(numPlayers + groupIndex) * size + playerIndex] = FINITE_FIELD_ONE;
      playerIndex += 1;
      groupIndex += 1;
    } else {
      for (let k = 0; k < count; k++) {
        // group
        mat[playerIndex * size + (numPlayers + groupIndex)] = FINITE_FIELD_ONE;
        // player
        mat[(numPlayers + groupIndex) * size + playerIndex] = FINITE_FIELD_ONE;
        playerIndex += 1;
        groupIndex += 1;
      }
    }
  }

  return [mat, size, size];
}

/**
 * コミットメントをサーバーに送信
 */
export async function submitCommitment(
  roomId: string,
  playerIndex: number,
  randomness: Field[],
  playerIdString: string,
): Promise<void> {
  const params = await loadCryptoParams();
  console.log("Submitting commitment for player index:", playerIndex);
  console.log("Loaded crypto params for commitment:", params);
  if (!params?.pedersenParam) {
    throw new Error("Pedersen parameters not available");
  }

  const pedersenInput = {
    pedersenParams: params.pedersenParam,
    x: randomness,
    pedersenRandomness: randomness, // 同じランダムネスを使用
  };

  console.log("Computing Pedersen commitment with input:", pedersenInput);

  const commitment = await MPCEncryption.pedersenCommitment(pedersenInput);

  console.log("Computed commitment:", commitment);

  // サーバーへ送信
  const base = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";
  const res = await fetch(`${base}/game/${roomId}/commitment`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ player_id: playerIdString, commitment }),
  });

  if (!res.ok) {
    const error = await res.json();
    throw new Error(`Failed to submit commitment: ${error.message || res.statusText}`);
  }

  const result = await res.json();
  console.log("Commitment submitted successfully:", result);
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
  await loadCryptoParams(gameInfo);

  // ランダムネスを取得（既存があればそれを使用、なければ生成）
  const randomness = await getRandomness(roomId, username);

  // プレイヤーIndexを取得
  const playerIndex = getMyPlayerIndex(gameInfo, username);

  const playerId = getMyPlayerId(gameInfo, username);

  if (playerId === null) {
    throw new Error("Player ID not found for username: " + username);
  }

  // コミットメントを計算してキャッシュ（サーバー送信は別途実装可）
  await submitCommitment(roomId, playerIndex, randomness, playerId);

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
  // 最新のゲーム状態を取得してコミットメントを確実に反映
  let latestGameInfo = gameInfo;
  try {
    const response = await fetch(
      `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api"}/game/${roomId}/state`,
    );
    if (response.ok) {
      const freshGameInfo = await response.json();
      if (freshGameInfo?.crypto_parameters) {
        latestGameInfo = freshGameInfo;
        console.log("Fetched latest game state with crypto parameters for role assignment");
      }
    }
  } catch (error) {
    console.warn("Failed to fetch latest game state, using provided gameInfo:", error);
  }

  const cryptoParams = await loadCryptoParams(latestGameInfo);
  const myIndex = getMyPlayerIndex(latestGameInfo, username);

  const groupingParameter = latestGameInfo.grouping_parameter;
  if (!groupingParameter) {
    throw new Error("Grouping parameter is missing in crypto parameters");
  }

  // villager + fortune teller + 1 (werewolf)
  const maxGroupSize = groupingParameter.Villager[0] + groupingParameter.FortuneTeller[0] + 1;

  const generatedShuffleMatrices = generateShuffleMatricesForWasm(
    latestGameInfo.players.length, // n (players.length used here as n)
    maxGroupSize, // m
  );
  const playerRandomness = await getRandomness(roomId, username);

  const privateInput: RoleAssignmentPrivateInput = {
    id: myIndex,
    shuffleMatrices: generatedShuffleMatrices,
    randomness: FINITE_FIELD_ZERO,
    playerRandomness,
  };

  const generatedTau = generateTauMatrixForWasm(groupingParameter, latestGameInfo.players.length);

  const dummyRoleCommitment = [
    { x: FINITE_FIELD_ZERO, y: FINITE_FIELD_ZERO, _params: null },
    { x: FINITE_FIELD_ZERO, y: FINITE_FIELD_ZERO, _params: null },
    { x: FINITE_FIELD_ZERO, y: FINITE_FIELD_ZERO, _params: null },
    { x: FINITE_FIELD_ZERO, y: FINITE_FIELD_ZERO, _params: null },
  ];

  console.log("cryptoParams before roleassignment input generation:", cryptoParams);
  console.log("playerCommitments available:", cryptoParams.playerCommitments);
  console.log("playerCommitments length:", cryptoParams.playerCommitments?.length);

  // プレイヤーコミットメントを取得（サーバーから取得できた場合はそれを使用、なければダミー）
  let playerCommitments: PedersenCommitment[];

  if (cryptoParams.playerCommitments && cryptoParams.playerCommitments.length > 0) {
    console.log("Using actual player commitments from server");
    playerCommitments = cryptoParams.playerCommitments;

    // プレイヤー数と一致しない場合は警告
    if (playerCommitments.length !== latestGameInfo.players.length) {
      console.warn(
        `Player commitments count mismatch: expected ${latestGameInfo.players.length}, got ${playerCommitments.length}`,
      );

      // 不足分をダミーで埋める
      const dummyCommitment = { x: FINITE_FIELD_ZERO, y: FINITE_FIELD_ZERO, _params: null };
      while (playerCommitments.length < latestGameInfo.players.length) {
        playerCommitments.push(dummyCommitment);
      }
    }
  } else {
    console.warn("No player commitments available, using dummy commitments");
    playerCommitments = Array(latestGameInfo.players.length).fill({
      x: FINITE_FIELD_ZERO,
      y: FINITE_FIELD_ZERO,
      _params: null,
    });
  }

  const publicInput: RoleAssignmentPublicInput = {
    numPlayers: latestGameInfo.players.length,
    maxGroupSize,
    pedersenParam: cryptoParams.pedersenParam,
    groupingParameter,
    tauMatrix: generatedTau,
    roleCommitment: dummyRoleCommitment,
    playerCommitment: playerCommitments,
  };

  // プレイヤーの公開鍵を取得または生成
  const playerId = getMyPlayerId(latestGameInfo, username);
  const cryptoManager = new CryptoManager(playerId || username);

  let publicKey: string | undefined;
  if (!cryptoManager.hasKeyPair()) {
    console.log("Generating new keypair for role assignment");
    const keyPair = cryptoManager.generateKeyPair(playerId || username);
    publicKey = keyPair.publicKey;
  } else {
    publicKey = cryptoManager.getPublicKey();
  }

  return {
    privateInput,
    publicInput,
    nodeKeys: getNodeKeys(),
    scheme: getScheme(),
    publicKey,
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
  const cryptoParams = await loadCryptoParams(gameInfo);
  const randomness = await getRandomness(roomId, username);
  const myIndex = getMyPlayerIndex(gameInfo, username);

  const isWerewolfValue = isWerewolf(gameInfo, username) ? FINITE_FIELD_ONE : FINITE_FIELD_ZERO;

  const privateInput: DivinationPrivateInput =
    isDummy === false
      ? {
          id: myIndex,
          isTarget: gameInfo.players.map((player: any) =>
            player.id === targetId ? FINITE_FIELD_ONE : FINITE_FIELD_ZERO,
          ),
          isWerewolf: isWerewolfValue,
          randomness: randomness,
        }
      : {
          id: myIndex,
          isTarget: gameInfo.players.map(() => FINITE_FIELD_ZERO),
          isWerewolf: isWerewolfValue,
          randomness: randomness,
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
  const cryptoParams = await loadCryptoParams(gameInfo);
  const randomness = await getRandomness(roomId, username);
  const myIndex = getMyPlayerIndex(gameInfo, username);

  // MPC公開鍵の確認
  const nodeKeys = getNodeKeys();
  if (nodeKeys.length !== 3 || nodeKeys.some(key => !key.publicKey)) {
    throw new Error("MPC node public keys are not properly configured");
  }

  const privateInput: AnonymousVotingPrivateInput = {
    id: myIndex,
    isTargetId: gameInfo.players.map((player: any) =>
      player.id === votedForId ? FINITE_FIELD_ONE : FINITE_FIELD_ZERO,
    ),
    playerRandomness: randomness,
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
  const cryptoParams = await loadCryptoParams(gameInfo);
  const randomness = await getRandomness(roomId, username);
  const myIndex = getMyPlayerIndex(gameInfo, username);
  const amWerewolfValues = isWerewolf(gameInfo, username) ? FINITE_FIELD_ONE : FINITE_FIELD_ZERO;

  const privateInput: WinningJudgementPrivateInput = {
    id: myIndex,
    amWerewolf: amWerewolfValues,
    playerRandomness: randomness,
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
