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

// ゲーム入力生成クラス
export class GameInputGenerator {
  private roomId: string;
  private username: string;
  private gameInfo: GameInfo | null;
  private cryptoParams: any | null;
  private loadedCryptoParams: any | null = null; // 静的ファイルから読み込んだ暗号パラメータをキャッシュ
  private myRandomness: bigint[] | null = null; // プレイヤー固有のランダムネス

  constructor(roomId: string, username: string, gameInfo: GameInfo | null = null, cryptoParams: any | null = null) {
    this.roomId = roomId;
    this.username = username;
    this.gameInfo = gameInfo;
    this.cryptoParams = cryptoParams;
  }

  // ランダムネスの初期化（ゲーム開始時に1回だけ実行）
  public async initializeRandomness(): Promise<void> {
    // LocalStorageから既存のランダムネスを確認
    const storageKey = `randomness_${this.roomId}_${this.username}`;
    const stored = localStorage.getItem(storageKey);

    if (stored) {
      try {
        this.myRandomness = JSONbigNative.parse(stored);
        console.log("Loaded existing randomness from localStorage");
        return;
      } catch (error) {
        console.warn("Failed to parse stored randomness, generating new one");
      }
    }

    // 新規生成（現在はダミー値、後でWASM関数に置き換え）
    // TODO: WASMのgeneratePedersenRandomness()を使用
    const randomness = [
      BigInt(Math.floor(Math.random() * 1000000000)),
      BigInt(Math.floor(Math.random() * 1000000000)),
      BigInt(Math.floor(Math.random() * 1000000000)),
      BigInt(Math.floor(Math.random() * 1000000000)),
    ];

    this.myRandomness = randomness;

    // LocalStorageに保存
    try {
      localStorage.setItem(storageKey, JSONbigNative.stringify(randomness));
      console.log("Generated and saved new randomness to localStorage");
    } catch (error) {
      console.error("Failed to save randomness to localStorage:", error);
    }

    // コミットメントを計算してサーバーに送信
    try {
      await this.submitCommitment(randomness);
    } catch (error) {
      console.error("Failed to submit commitment:", error);
    }
  }

  // コミットメントをサーバーに送信
  private async submitCommitment(randomness: bigint[]): Promise<void> {
    if (!this.cryptoParams?.pedersenParam) {
      console.warn("Pedersen parameters not available, skipping commitment submission");
      return;
    }

    try {
      // TODO: WASMのcomputePedersenCommitment()を使用
      // const commitment = await computePedersenCommitment(
      //   this.cryptoParams.pedersenParam,
      //   this.getMyPlayerId(),
      //   randomness
      // );

      // 現在はダミーコミットメントを送信
      const dummyCommitment = {
        player_id: this.getMyPlayerId(),
        commitment: randomness.map(r => r.toString()),
        created_at: new Date().toISOString(),
      };

      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/game/${this.roomId}/commitment`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
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

  // ランダムネスを取得（初期化されていない場合はエラー）
  private getMyRandomness(): bigint[] {
    if (!this.myRandomness) {
      throw new Error("Randomness not initialized. Call initializeRandomness() first.");
    }
    return this.myRandomness;
  }

  // ランダムネスが初期化されているか確認
  public isRandomnessInitialized(): boolean {
    return this.myRandomness !== null;
  }

  // 暗号パラメータを更新
  public updateCryptoParams(cryptoParams: any) {
    this.cryptoParams = cryptoParams;
    // 既に読み込み済みの暗号パラメータがあればマージ
    if (this.loadedCryptoParams) {
      this.cryptoParams = { ...this.cryptoParams, ...this.loadedCryptoParams };
    }
  }

  // ゲーム情報を更新
  public updateGameInfo(gameInfo: GameInfo) {
    this.gameInfo = gameInfo;
  }

  // プレイヤーIDを取得
  private getMyPlayerId(): number {
    if (!this.gameInfo) return -1;
    return this.gameInfo.players.findIndex((player: any) => player.name === this.username);
  }

  // 自分のプレイヤー情報を取得
  private getMyPlayerInfo(): any | null {
    if (!this.gameInfo) return null;
    return this.gameInfo.players.find((player: any) => player.name === this.username) || null;
  }

  // 自分の役職を取得
  private getMyRole(): string | null {
    const player = this.getMyPlayerInfo();
    return player?.role || null;
  }

  // 参加者総数を取得
  private getTotalPlayerCount(): number {
    return this.gameInfo?.players.length || 0;
  }

  // 生存者数を取得
  private getAlivePlayerCount(): number {
    if (!this.gameInfo) return 0;
    return this.gameInfo.players.filter((player: any) => !player.is_dead).length;
  }

  // 生存者リストを取得
  private getAlivePlayers(): any[] {
    if (!this.gameInfo) return [];
    return this.gameInfo.players.filter((player: any) => !player.is_dead);
  }

  // 人狼かどうかを判定
  private isWerewolf(): boolean {
    return this.getMyRole() === "Werewolf";
  }

  // 占い師かどうかを判定
  private isSeer(): boolean {
    return this.getMyRole() === "Seer" || this.getMyRole() === "FortuneTeller";
  }

  // NodeKeysを取得
  private getNodeKeys(): NodeKey[] {
    return [
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
  }

  // SecretSharingSchemeを取得
  private getScheme(): SecretSharingScheme {
    return {
      totalShares: 3,
      modulus: 97,
    };
  }

  // 暗号パラメータを静的ファイルから読み込む（一度だけ実行）
  private async loadCryptoParams(): Promise<any> {
    if (this.loadedCryptoParams) {
      return this.loadedCryptoParams; // キャッシュされていれば再利用
    }

    // pedersen_params2.jsonを読み込み
    const pedersenRes = await fetch("/pedersen_params2.json");
    const pedersenParams = await pedersenRes.text();
    const parsedPedersenParams = JSONbigNative.parse(pedersenParams);

    const commitres = await fetch("/pedersen_commitment_0.json");
    const commitment = await commitres.text();
    const parsedCommitment = JSONbigNative.parse(commitment);

    const randres = await fetch("/pedersen_randomness_0.json");
    const randomness = await randres.text();
    const parsedRandomness = JSONbigNative.parse(randomness);

    const elgamalparamres = await fetch("/elgamal_params.json");
    const elgamalparam = await elgamalparamres.text();
    const parsedElgamalParam = JSONbigNative.parse(elgamalparam);

    const elgamalpubkeyres = await fetch("/elgamal_public_key.json");
    const elgamalpubkey = await elgamalpubkeyres.text();
    const parsedElgamalPubkey = JSONbigNative.parse(elgamalpubkey);

    // 他の暗号パラメータも追加可能な構造で保存
    const cryptoParams = {
      pedersenParam: parsedPedersenParams,
      pedersenCommitment: parsedCommitment,
      pedersenRandomness: parsedRandomness,
      // 将来的に他のパラメータも追加可能
      elgamalParam: parsedElgamalParam,
      elgamalPublicKey: parsedElgamalPubkey,
    };

    // インスタンスにキャッシュ
    this.loadedCryptoParams = cryptoParams;

    // 既存のcryptoParamsも更新
    if (this.cryptoParams) {
      this.cryptoParams = { ...this.cryptoParams, ...cryptoParams };
    }

    return cryptoParams;
  }

  // 役職配布用の入力を生成
  public async getRoleAssignmentInput(): Promise<{
    input: RoleAssignmentInput;
  }> {
    if (!this.gameInfo || !this.cryptoParams) {
      throw new Error("Game info or crypto params not available");
    }

    const rinputres = await fetch("/test_role_assignment_input2.json");
    const rinput = await rinputres.text();
    const parsedRinput: RoleAssignmentInput = JSONbigNative.parse(rinput);

    parsedRinput.privateInput.id = this.getMyPlayerId();

    const myId = this.getMyPlayerId();

    // TODO: 実際の暗号パラメータ生成ロジックを実装
    const privateInput: RoleAssignmentPrivateInput = {
      id: myId,
      shuffleMatrices: null, // ダミー値
      randomness: null, // ダミー値
      playerRandomness: [0, 0, 0, 0], // ダミー値
    };

    const totalPlayers = this.getTotalPlayerCount();

    // gameInfoから暗号パラメータを読み込み（フォールバック: 静的ファイル）
    let pedersenParam;
    if (this.cryptoParams?.pedersenParam) {
      pedersenParam = this.cryptoParams.pedersenParam;
    } else {
      const loadedParams = await this.loadCryptoParams();
      pedersenParam = loadedParams.pedersenParam;
    }

    const publicInput: RoleAssignmentPublicInput = {
      numPlayers: totalPlayers,
      maxGroupSize: totalPlayers,
      pedersenParam, // 読み込んだパラメータを使用
      groupingParameter: {
        Villager: [Math.max(1, totalPlayers - 2), false], // 人狼と占い師以外
        FortuneTeller: [1, false], // 占い師1人
        Werewolf: [1, false], // 人狼1人
      },
      tauMatrix: null,
      roleCommitment: Array(totalPlayers).fill({} as PedersenCommitment),
      playerCommitment: Array(totalPlayers).fill({} as PedersenCommitment),
    };

    const nodeKeys = this.getNodeKeys();
    const scheme = this.getScheme();

    const input: RoleAssignmentInput = {
      privateInput: parsedRinput.privateInput,
      publicInput: parsedRinput.publicInput,
      nodeKeys,
      scheme,
    };

    // const input = parsedRinput;
    return { input };
  }

  // 占い用の入力を生成
  public async getDivinationInput(targetId: string): Promise<{
    input: DivinationInput;
  }> {
    if (!this.gameInfo || !this.cryptoParams) {
      throw new Error("Game info or crypto params not available");
    }

    const myId = this.getMyPlayerId();

    // gameInfoから暗号パラメータを読み込み（フォールバック: 静的ファイル）
    let pedersenParam;
    if (this.cryptoParams?.pedersenParam) {
      pedersenParam = this.cryptoParams.pedersenParam;
    } else {
      const loadedParams = await this.loadCryptoParams();
      pedersenParam = loadedParams.pedersenParam;
    }

    // 占い用の新規ランダムネス生成（1回の占いごとに新しいもの）
    // TODO: WASMのgeneratePedersenRandomness()を使用
    const divinationRandomness = [
      //   BigInt(Math.floor(Math.random() * 1000000000)),
      //   BigInt(Math.floor(Math.random() * 1000000000)),
      //   BigInt(Math.floor(Math.random() * 1000000000)),
      //   BigInt(Math.floor(Math.random() * 1000000000)),
      JSONbigNative.parse('["0","0","0","0"]'),
      null,
    ];

    // 実際のゲーム状態を使用
    const isWerewolfValue = this.isWerewolf()
      ? [
          JSONbigNative.parse(
            '["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]',
          ),
          null,
        ]
      : [JSONbigNative.parse('["0","0","0","0"]'), null];

    const privateInput: DivinationPrivateInput = {
      id: myId,
      isTarget: this.gameInfo.players.map((player: any) => [
        player.id === targetId
          ? JSONbigNative.parse(
              '["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]',
            )
          : JSONbigNative.parse('["0","0","0","0"]'),
        null,
      ]),
      isWerewolf: isWerewolfValue,
      randomness: divinationRandomness, // 占いごとに新規生成
    };

    const publicInput: DivinationPublicInput = {
      pedersenParam, // 読み込んだパラメータを使用
      elgamalParam: this.cryptoParams.elgamalParam || {},
      pubKey: this.cryptoParams.elgamalPublicKey || {},
      playerNum: this.getTotalPlayerCount(),
    };

    const nodeKeys = this.getNodeKeys();
    const scheme = this.getScheme();

    const input: DivinationInput = {
      privateInput,
      publicInput,
      nodeKeys,
      scheme,
    };

    return { input };
  }

  // 投票用の入力を生成（useVotingの実装を参考に改良）
  public async getVotingInput(votedForId: string): Promise<{
    input: AnonymousVotingInput;
  }> {
    if (!this.gameInfo || !this.cryptoParams) {
      throw new Error("Game info or crypto params not available");
    }

    // MPC公開鍵の確認（getNodeKeysを使用）
    const nodeKeys = this.getNodeKeys();
    if (nodeKeys.length !== 3 || nodeKeys.some(key => !key.publicKey)) {
      throw new Error("MPC node public keys are not properly configured");
    }

    const myId = this.getMyPlayerId();

    // 静的ファイルから暗号パラメータを読み込み
    const loadedParams = await this.loadCryptoParams();
    const { pedersenParam, pedersenCommitment } = loadedParams;

    // プレイヤーのランダムネスを使用（初期化時に生成済み）
    const myRandomness = this.getMyRandomness();

    // bigint[]を(number[] | null)[]に変換
    const randomnessForVoting = myRandomness.map(r => Array.from(r.toString().split("")).map(Number));

    const privateInput: AnonymousVotingPrivateInput = {
      id: myId,
      isTargetId: this.gameInfo.players.map((player: any) =>
        player.id === votedForId
          ? [
              JSONbigNative.parse(
                '["9015221291577245683","8239323489949974514","1646089257421115374","958099254763297437"]',
              ),
              null,
            ]
          : [JSONbigNative.parse('["0","0","0","0"]'), null],
      ),
      playerRandomness: randomnessForVoting, // 型変換して使用
    };

    const publicInput: AnonymousVotingPublicInput = {
      pedersenParam, // 読み込んだパラメータを使用
      playerCommitment: Array(this.getTotalPlayerCount()).fill(pedersenCommitment as PedersenCommitment),
      playerNum: this.getTotalPlayerCount(),
    };

    const scheme = this.getScheme();

    const input: AnonymousVotingInput = {
      privateInput,
      publicInput,
      nodeKeys,
      scheme,
    };

    return { input };
  }

  // 投票データの暗号化（useVotingから移植）
  public async encryptVotingData(votedForId: string): Promise<any> {
    const { input } = await this.getVotingInput(votedForId);
    return await MPCEncryption.encryptAnonymousVoting(input);
  }

  // 勝敗判定用の入力を生成
  public async getWinningJudgementInput(): Promise<{
    input: WinningJudgementInput;
  }> {
    if (!this.gameInfo || !this.cryptoParams) {
      throw new Error("Game info or crypto params not available");
    }

    const myId = this.getMyPlayerId();

    // 実際の役職情報を使用
    const amWerewolfValues = this.isWerewolf()
      ? JSONbigNative.parse(
          '["9015221291577245683", "8239323489949974514", "1646089257421115374", "958099254763297437"]',
        )
      : JSONbigNative.parse('["0", "0", "0", "0"]');

    // gameInfoから暗号パラメータを読み込み（フォールバック: 静的ファイル）
    let pedersenParam, pedersenCommitment;
    if (this.cryptoParams?.pedersenParam) {
      pedersenParam = this.cryptoParams.pedersenParam;
      pedersenCommitment = this.cryptoParams.playerCommitments?.[0] || null;
    }

    if (!pedersenParam) {
      const loadedParams = await this.loadCryptoParams();
      pedersenParam = loadedParams.pedersenParam;
      pedersenCommitment = loadedParams.pedersenCommitment;
    }

    // プレイヤーのランダムネスを使用（初期化時に生成済み）
    const myRandomness = this.getMyRandomness();

    // bigint[]を(bigint[] | null)[]に変換 - 各要素を配列として扱う
    const randomnessArray: (bigint[] | null)[] = [[...myRandomness], null];

    const privateInput: WinningJudgementPrivateInput = {
      id: myId,
      amWerewolf: [amWerewolfValues, null],
      playerRandomness: randomnessArray, // 初期化時のランダムネスを使用
    };

    const publicInput: WinningJudgementPublicInput = {
      pedersenParam, // 読み込んだパラメータを使用
      playerCommitment: Array(this.getTotalPlayerCount()).fill(pedersenCommitment),
    };

    const nodeKeys = this.getNodeKeys();
    const scheme = this.getScheme();

    const input: WinningJudgementInput = {
      privateInput,
      publicInput,
      nodeKeys,
      scheme,
    };

    return { input };
  }
}

// シングルトンインスタンス作成用のファクトリー関数
export const createGameInputGenerator = (
  roomId: string,
  username: string,
  gameInfo?: GameInfo | null,
  cryptoParams?: any | null,
): GameInputGenerator => {
  return new GameInputGenerator(roomId, username, gameInfo, cryptoParams);
};
