/**
 * 暗号化処理ヘルパー
 * E2Eテストで使用する暗号化関連のユーティリティ
 */
import fs from "fs/promises";
import JSONbig from "json-bigint";
import path from "path";
import { loadCryptoParams } from "~~/services/gameInputGenerator";
import type { CryptoParameters } from "~~/types/game";
import { MPCEncryption } from "~~/utils/crypto/InputEncryption";
import type {
  //   CryptoParams,
  //   ElGamalKeyPair,
  AnonymousVotingInput,
  DivinationInput,
  ElGamalKeygenOutput,
  ElGamalParam,
  KeyPublicizeInput,
  RoleAssignmentInput,
  WinningJudgementInput,
} from "~~/utils/crypto/type";

const JSONbigNative = JSONbig({ useNativeBigInt: true });

export class CryptoHelper {
  /**
   * 暗号パラメータをロード
   * 本番環境と同じように、gameInfoから優先的に取得
   */
  static async loadParams(gameInfo?: any): Promise<CryptoParameters> {
    try {
      // 本番環境と同じロジック: gameInfoがあればそれを使用
      return await loadCryptoParams(gameInfo);
    } catch (e) {
      // Fallback for Node/Jest environment: load static JSON files from packages/mpc-algebra-wasm
      const base = path.resolve(process.cwd(), "..", "mpc-algebra-wasm");
      const pedersenRaw = await fs.readFile(path.join(base, "pedersen_params2.json"), "utf8");
      const commitRaw = await fs.readFile(path.join(base, "pedersen_commitment_0.json"), "utf8");
      const elgamalParamRaw = await fs.readFile(path.join(base, "elgamal_params.json"), "utf8");
      const elgamalPubKeyRaw = await fs.readFile(path.join(base, "elgamal_public_key.json"), "utf8");

      const pedersen = JSONbigNative.parse(pedersenRaw);
      const commitment = JSONbigNative.parse(commitRaw);
      const elgamalParam = JSONbigNative.parse(elgamalParamRaw);
      const elgamalPubKey = JSONbigNative.parse(elgamalPubKeyRaw);

      return {
        pedersen_param: pedersen,
        player_commitment: [commitment],
        fortune_teller_public_key: elgamalPubKey,
        elgamal_param: elgamalParam,
      } as CryptoParameters;
    }
  }

  /**
   * ElGamal鍵ペアを生成
   */
  static async generateKeyPair(params: CryptoParameters): Promise<ElGamalKeygenOutput> {
    return await MPCEncryption.elgamalKeygen({
      elgamalParams: params.elgamal_param,
    });
  }

  /**
   * 回路タイプに応じた暗号化処理
   */
  static async encryptForCircuit(
    circuitType: "RoleAssignment" | "KeyPublicize" | "Divination" | "AnonymousVoting" | "WinningJudgement",
    input: any,
  ): Promise<any> {
    switch (circuitType) {
      case "RoleAssignment":
        return await MPCEncryption.encryptRoleAssignment(input as RoleAssignmentInput);
      case "KeyPublicize":
        return await MPCEncryption.encryptKeyPublicize(input as KeyPublicizeInput);
      case "Divination":
        return await MPCEncryption.encryptDivination(input as DivinationInput);
      case "AnonymousVoting":
        return await MPCEncryption.encryptAnonymousVoting(input as AnonymousVotingInput);
      case "WinningJudgement":
        return await MPCEncryption.encryptWinningJudgement(input as WinningJudgementInput);
      default:
        throw new Error(`Unknown circuit type: ${circuitType}`);
    }
  }
}
