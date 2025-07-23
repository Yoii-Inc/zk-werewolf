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

  /**
   * キーペアを生成し、Base64エンコードされた形式で保存
   */
  generateKeyPair(): KeyPair {
    const rawKeyPair = box.keyPair();
    this.keyPair = {
      publicKey: encodeBase64(rawKeyPair.publicKey),
      privateKey: encodeBase64(rawKeyPair.secretKey),
    };
    return this.keyPair;
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
   * 公開鍵を取得
   */
  getPublicKey(): string {
    if (!this.keyPair) {
      throw new Error("キーペアが生成されていません");
    }
    return this.keyPair.publicKey;
  }
}
