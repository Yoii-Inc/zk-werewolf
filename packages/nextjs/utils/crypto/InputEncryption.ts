"use client";

import init, {
  init as RustInit,
  divination,
  key_publicize,
  role_assignment,
  voting_split_and_encrypt,
  winning_judgement,
} from "../../../mpc-algebra-wasm/pkg-node/mpc_algebra_wasm";
import {
  AnonymousVotingInput,
  AnonymousVotingOutput,
  DivinationInput,
  DivinationOutput,
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
}
