import { box, randomBytes } from "tweetnacl";
import { decodeBase64, decodeUTF8, encodeBase64, encodeUTF8 } from "tweetnacl-util";

export interface KeyPair {
  publicKey: string;
  privateKey: string;
}

export interface EncryptedMessage {
  encrypted: string;
  nonce: string;
}

export class CryptoManager {
  private keyPair: KeyPair | null = null;
  private userId: string | null = null;

  constructor(userId?: string) {
    this.userId = userId || null;
    if (userId) {
      this.loadKeyPairFromStorage(userId);
    }
  }

  /**
   * キーペアを生成し、Base64エンコードされた形式で保存
   */
  generateKeyPair(userId?: string): KeyPair {
    const rawKeyPair = box.keyPair();
    this.keyPair = {
      publicKey: encodeBase64(rawKeyPair.publicKey),
      privateKey: encodeBase64(rawKeyPair.secretKey),
    };

    // userIdが指定されている場合、localStorageに保存
    if (userId || this.userId) {
      this.saveKeyPairToStorage(userId || this.userId!);
    }

    return this.keyPair;
  }

  /**
   * localStorageからキーペアを読み込む
   */
  loadKeyPairFromStorage(userId: string): KeyPair | null {
    if (typeof window === "undefined") return null;

    try {
      const publicKey = localStorage.getItem(`user_public_key_${userId}`);
      const privateKey = localStorage.getItem(`user_secret_key_${userId}`);

      if (publicKey && privateKey) {
        this.keyPair = { publicKey, privateKey };
        this.userId = userId;
        return this.keyPair;
      }
    } catch (error) {
      console.error("Failed to load keypair from storage:", error);
    }

    return null;
  }

  /**
   * localStorageにキーペアを保存
   */
  saveKeyPairToStorage(userId: string): void {
    if (typeof window === "undefined" || !this.keyPair) return;

    try {
      localStorage.setItem(`user_public_key_${userId}`, this.keyPair.publicKey);
      localStorage.setItem(`user_secret_key_${userId}`, this.keyPair.privateKey);
      this.userId = userId;
    } catch (error) {
      console.error("Failed to save keypair to storage:", error);
      throw new Error("キーペアの保存に失敗しました");
    }
  }

  /**
   * localStorageからキーペアを削除
   */
  clearKeyPairFromStorage(userId: string): void {
    if (typeof window === "undefined") return;

    try {
      localStorage.removeItem(`user_public_key_${userId}`);
      localStorage.removeItem(`user_secret_key_${userId}`);
      if (this.userId === userId) {
        this.keyPair = null;
        this.userId = null;
      }
    } catch (error) {
      console.error("Failed to clear keypair from storage:", error);
    }
  }

  /**
   * メッセージを暗号化
   */
  encrypt(message: string, recipientPublicKey: string): EncryptedMessage {
    if (!this.keyPair) {
      throw new Error("キーペアが生成されていません");
    }

    const nonce = randomBytes(box.nonceLength);
    const messageUint8 = decodeUTF8(message);
    const recipientPublicKeyUint8 = decodeBase64(recipientPublicKey);
    const senderPrivateKeyUint8 = decodeBase64(this.keyPair.privateKey);

    const encryptedMessage = box(messageUint8, nonce, recipientPublicKeyUint8, senderPrivateKeyUint8);

    return {
      encrypted: encodeBase64(encryptedMessage),
      nonce: encodeBase64(nonce),
    };
  }

  /**
   * メッセージを復号
   * @param encrypted Base64エンコードされた暗号文
   * @param nonce Base64エンコードされたnonce
   * @param senderPublicKey Base64エンコードされた送信者の公開鍵
   * @returns 復号されたメッセージ
   */
  decrypt(encrypted: string, nonce: string, senderPublicKey: string): string {
    if (!this.keyPair) {
      throw new Error("キーペアが生成されていません");
    }

    const encryptedUint8 = decodeBase64(encrypted);
    const nonceUint8 = decodeBase64(nonce);
    const senderPublicKeyUint8 = decodeBase64(senderPublicKey);
    const recipientPrivateKeyUint8 = decodeBase64(this.keyPair.privateKey);

    const decryptedMessage = box.open(encryptedUint8, nonceUint8, senderPublicKeyUint8, recipientPrivateKeyUint8);

    if (!decryptedMessage) {
      throw new Error("復号に失敗しました");
    }

    return encodeUTF8(decryptedMessage);
  }

  /**
   * 公開鍵を取得
   */
  getPublicKey(): string {
    if (!this.keyPair) {
      throw new Error("キーペアが生成されていません");
    }
    return this.keyPair.publicKey;
  }

  /**
   * 秘密鍵を取得（注意: セキュリティリスクあり）
   */
  getPrivateKey(): string {
    if (!this.keyPair) {
      throw new Error("キーペアが生成されていません");
    }
    return this.keyPair.privateKey;
  }

  /**
   * キーペアが存在するかチェック
   */
  hasKeyPair(): boolean {
    return this.keyPair !== null;
  }

  /**
   * 現在のキーペアを取得
   */
  getKeyPair(): KeyPair | null {
    return this.keyPair;
  }
}
