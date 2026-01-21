"use client";

import init, {
  init as RustInit,
  divination,
  elgamal_decrypt,
  elgamal_keygen,
  fr_rand,
  key_publicize,
  pedersen_commitment,
  role_assignment,
  voting_split_and_encrypt,
  winning_judgement,
} from "../../../mpc-algebra-wasm/pkg-node/mpc_algebra_wasm";
import {
  AnonymousVotingInput,
  AnonymousVotingOutput,
  DivinationInput,
  DivinationOutput,
  ElGamalDecryptInput,
  ElGamalDecryptOutput,
  ElGamalKeygenInput,
  ElGamalKeygenOutput,
  KeyPublicizeInput,
  KeyPublicizeOutput,
  RoleAssignmentInput,
  RoleAssignmentOutput,
  WinningJudgementInput,
  WinningJudgementOutput,
} from "./type";

export class MPCEncryption {
  private static isInitialized = false;

  /**
   * WAMSの初期化
   */
  private static async initializeWasm(): Promise<void> {
    if (!this.isInitialized) {
      await init();
      RustInit();
      this.isInitialized = true;
    }
  }

  /**
   * 匿名投票の暗号化
   */
  public static async encryptAnonymousVoting(input: AnonymousVotingInput): Promise<AnonymousVotingOutput> {
    await this.initializeWasm();
    try {
      return await voting_split_and_encrypt(input);
    } catch (error) {
      console.error("Anonymous voting encryption failed:", error);
      throw new Error(`Failed to encrypt anonymous vote`);
    }
  }

  /**
   * 鍵公開の暗号化
   */
  public static async encryptKeyPublicize(input: KeyPublicizeInput): Promise<KeyPublicizeOutput> {
    await this.initializeWasm();
    try {
      return key_publicize(input);
    } catch (error) {
      console.error("Key publicize encryption failed:", error);
      throw new Error(`Failed to encrypt key publicize`);
    }
  }

  /**
   * 役職割り当ての暗号化
   */
  public static async encryptRoleAssignment(input: RoleAssignmentInput): Promise<RoleAssignmentOutput> {
    await this.initializeWasm();
    try {
      return role_assignment(input);
    } catch (error) {
      console.error("Role assignment encryption failed:", error);
      throw new Error(`Failed to encrypt role assignment`);
    }
  }

  /**
   * 占い師の暗号化
   */
  public static async encryptDivination(input: DivinationInput): Promise<DivinationOutput> {
    await this.initializeWasm();
    try {
      return divination(input);
    } catch (error) {
      console.error("Divination encryption failed:", error);
      throw new Error(`Failed to encrypt divination`);
    }
  }

  /**
   * 勝利判定の暗号化
   */
  public static async encryptWinningJudgement(input: WinningJudgementInput): Promise<WinningJudgementOutput> {
    await this.initializeWasm();
    try {
      return winning_judgement(input);
    } catch (error) {
      console.error("Winning judgement encryption failed:", error);
      throw new Error(`Failed to encrypt winning judgement`);
    }
  }

  /**
   * ElGamal復号化
   */
  public static async decryptElGamal(input: ElGamalDecryptInput): Promise<any> {
    await this.initializeWasm();
    try {
      const result = elgamal_decrypt(input);
      return JSON.parse(result);
    } catch (error) {
      console.error("ElGamal decryption failed:", error);
      throw new Error(`Failed to decrypt ElGamal cipher: ${error}`);
    }
  }

  /**
   * Fr random generator
   */
  public static async frRand(): Promise<any> {
    await this.initializeWasm();
    try {
      const result = fr_rand();
      return JSON.parse(result);
    } catch (error) {
      console.error("Fr random generation failed:", error);
      throw new Error(`Failed to generate Fr random: ${error}`);
    }
  }

  /**
   * Pedersen commitment wrapper
   * Expects input: { pedersen_params, x, pedersen_randomness }
   */
  public static async pedersenCommitment(input: any): Promise<any> {
    await this.initializeWasm();
    try {
      const result = pedersen_commitment(input);
      return JSON.parse(result);
    } catch (error) {
      console.error("Pedersen commitment failed:", error);
      throw new Error(`Failed to compute pedersen commitment`);
    }
  }
  /**
   * ElGamal鍵ペア生成
   * Expects input: { elgamalParams }
   */
  public static async elgamalKeygen(input: any): Promise<any> {
    await this.initializeWasm();
    try {
      const result = elgamal_keygen(input);
      return JSON.parse(result);
    } catch (error) {
      console.error("ElGamal keygen failed:", error);
      throw new Error(`Failed to generate ElGamal keypair: ${error}`);
    }
  }
}
