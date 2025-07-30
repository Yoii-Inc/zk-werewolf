import { box, randomBytes } from "tweetnacl";
import { decodeBase64, encodeBase64 } from "tweetnacl-util";

export class TweetNaclKeyManager {
  private keyPair: { publicKey: Uint8Array; secretKey: Uint8Array } | null = null;

  generateKeyPair() {
    this.keyPair = box.keyPair();
    return {
      public_key: encodeBase64(this.keyPair.publicKey),
      secret_key: encodeBase64(this.keyPair.secretKey),
    };
  }

  getPublicKey(): string | null {
    if (!this.keyPair) return null;
    return encodeBase64(this.keyPair.publicKey);
  }

  encrypt(message: string, theirPublicKey: string): string {
    if (!this.keyPair) throw new Error("No key pair available");

    const messageUint8 = new TextEncoder().encode(message);
    const theirPublicKeyUint8 = decodeBase64(theirPublicKey);
    const ephemeralKeyPair = box.keyPair();
    const nonce = randomBytes(box.nonceLength);

    const encryptedMessage = box(messageUint8, nonce, theirPublicKeyUint8, this.keyPair.secretKey);

    const fullMessage = new Uint8Array(nonce.length + encryptedMessage.length);
    fullMessage.set(nonce);
    fullMessage.set(encryptedMessage, nonce.length);

    return encodeBase64(fullMessage);
  }

  decrypt(encryptedMessage: string, theirPublicKey: string): string {
    if (!this.keyPair) throw new Error("No key pair available");

    const messageWithNonceAsUint8 = decodeBase64(encryptedMessage);
    const theirPublicKeyUint8 = decodeBase64(theirPublicKey);

    const nonce = messageWithNonceAsUint8.slice(0, box.nonceLength);
    const message = messageWithNonceAsUint8.slice(box.nonceLength);

    const decryptedMessage = box.open(message, nonce, theirPublicKeyUint8, this.keyPair.secretKey);

    if (!decryptedMessage) throw new Error("Could not decrypt message");

    return new TextDecoder().decode(decryptedMessage);
  }

  setKeyPair(publicKey: string, secretKey: string) {
    this.keyPair = {
      publicKey: decodeBase64(publicKey),
      secretKey: decodeBase64(secretKey),
    };
  }

  async saveKeyPair(playerId: string) {
    if (!this.keyPair) throw new Error("No key pair available");

    const keyData = {
      public_key: encodeBase64(this.keyPair.publicKey),
      secret_key: encodeBase64(this.keyPair.secretKey),
    };

    const response = await fetch("/api/keys/save", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        playerId,
        keyData,
      }),
    });

    if (!response.ok) {
      throw new Error("Failed to save key pair");
    }

    return keyData.public_key;
  }

  async loadKeyPairFromApi(playerId: string): Promise<boolean> {
    try {
      const response = await fetch(`/api/keys/load?playerId=${playerId}`);
      if (!response.ok) {
        return false;
      }

      const keyData = await response.json();
      this.setKeyPair(keyData.public_key, keyData.secret_key);
      return true;
    } catch (error) {
      console.error("Error loading key pair:", error);
      return false;
    }
  }

  async generateAndSaveKeyPair(playerId: string): Promise<string> {
    const keys = this.generateKeyPair();
    return this.saveKeyPair(playerId);
  }
}
