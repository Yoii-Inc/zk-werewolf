import * as ed from "@noble/ed25519";

export class KeyManager {
  private keyPair: { publicKey: Uint8Array; privateKey: Uint8Array } | null = null;

  async generateKeyPair() {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = await ed.getPublicKey(privateKey);
    this.keyPair = { publicKey, privateKey };
    return {
      publicKey: Buffer.from(publicKey).toString("base64"),
      privateKey: Buffer.from(privateKey).toString("base64"),
    };
  }

  getPublicKey(): string | null {
    if (!this.keyPair) return null;
    return Buffer.from(this.keyPair.publicKey).toString("base64");
  }

  async sign(message: string): Promise<string> {
    if (!this.keyPair) throw new Error("No key pair available");
    const messageBytes = new TextEncoder().encode(message);
    const signature = await ed.sign(messageBytes, this.keyPair.privateKey);
    return Buffer.from(signature).toString("base64");
  }
}
