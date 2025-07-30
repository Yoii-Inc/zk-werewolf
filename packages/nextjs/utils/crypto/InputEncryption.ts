"use client";

import {
  divination,
  init,
  key_publicize,
  role_assignment,
  voting_split_and_encrypt,
  winning_judgement,
} from "../../../mpc-algebra-wasm/pkg-node/mpc_algebra_wasm";

// 後で書く
export type AnonymousVotingInput = any;
export type AnonymousVotingOutput = any;

export class MPCEncryption {
  private static isInitialized = false;

  /**
   * WAMSの初期化
   */
  private static async initializeWasm(): Promise<void> {
    if (!this.isInitialized) {
      await init();
      this.isInitialized = true;
    }
  }

  /**
   * 匿名投票の暗号化
   */
  public static async encryptAnonymousVoting(input: AnonymousVotingInput): Promise<AnonymousVotingOutput> {
    await this.initializeWasm();
    try {
      const result = await voting_split_and_encrypt(JSON.stringify(input));
      return JSON.parse(result);
    } catch (error) {
      console.error("Anonymous voting encryption failed:", error);
      throw new Error(`Failed to encrypt anonymous vote`);
    }
  }

  //   /**
  //    * 鍵公開の暗号化
  //    */
  //   public static async encryptKeyPublicize(input: KeyPublicizeInput): Promise<KeyPublicizeOutput> {
  //     await this.initializeWasm();
  //     try {
  //       const result = await key_publicize(JSON.stringify(input));
  //       return JSON.parse(result);
  //     } catch (error) {
  //       console.error("Key publicize encryption failed:", error);
  //       throw new Error(`Failed to encrypt key publicize: ${error.message}`);
  //     }
  //   }

  //   /**
  //    * 役職割り当ての暗号化
  //    */
  //   public static async encryptRoleAssignment(input: RoleAssignmentInput): Promise<RoleAssignmentOutput> {
  //     await this.initializeWasm();
  //     try {
  //       const result = await role_assignment(JSON.stringify(input));
  //       return JSON.parse(result);
  //     } catch (error) {
  //       console.error("Role assignment encryption failed:", error);
  //       throw new Error(`Failed to encrypt role assignment: ${error.message}`);
  //     }
  //   }

  //   /**
  //    * 占い師の暗号化
  //    */
  //   public static async encryptDivination(input: DivinationInput): Promise<DivinationOutput> {
  //     await this.initializeWasm();
  //     try {
  //       const result = await divination(JSON.stringify(input));
  //       return JSON.parse(result);
  //     } catch (error) {
  //       console.error("Divination encryption failed:", error);
  //       throw new Error(`Failed to encrypt divination: ${error.message}`);
  //     }
  //   }

  //   /**
  //    * 勝利判定の暗号化
  //    */
  //   public static async encryptWinningJudgement(input: WinningJudgementInput): Promise<WinningJudgementOutput> {
  //     await this.initializeWasm();
  //     try {
  //       const result = await winning_judgement(JSON.stringify(input));
  //       return JSON.parse(result);
  //     } catch (error) {
  //       console.error("Winning judgement encryption failed:", error);
  //       throw new Error(`Failed to encrypt winning judgement: ${error.message}`);
  //     }
  //   }
}
