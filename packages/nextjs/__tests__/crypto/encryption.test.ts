import { CryptoManager } from "../../utils/crypto/encryption";
import { expect } from "@jest/globals";
import fs from "fs";
import path from "path";

describe("Crypto Integration Test", () => {
  let cryptoManager: CryptoManager;
  const testDataDir = path.join(process.cwd(), "../../test-data");

  beforeAll(() => {
    // テストデータディレクトリが存在しない場合は作成
    if (!fs.existsSync(testDataDir)) {
      fs.mkdirSync(testDataDir, { recursive: true });
    }
  });
  beforeEach(() => {
    cryptoManager = new CryptoManager();
  });

  it("should encrypt data for rust backend", async () => {
    // Rustから生成された公開鍵を読み込む
    let nodePublicKey: string;
    try {
      const keysJson = fs.readFileSync(path.join(testDataDir, "node_keys.json"), "utf8");
      const keys = JSON.parse(keysJson);
      nodePublicKey = keys.nodePublicKey;
      if (!nodePublicKey) {
        throw new Error("Node public key not found in keys.json");
      }
    } catch (error) {
      console.error("Failed to read node_keys.json:", error);
      throw error;
    }

    // キーペアを生成
    const keyPair = cryptoManager.generateKeyPair();

    // テストデータ
    const testData = {
      voterId: "test-voter-1",
      targetId: "test-target-1",
      timestamp: Date.now(),
    };

    // データを暗号化
    const encrypted = cryptoManager.encrypt(JSON.stringify(testData), nodePublicKey);

    // テストデータを保存
    const testOutput = {
      encrypted: encrypted.encrypted,
      nonce: encrypted.nonce,
      sender_public_key: cryptoManager.getPublicKey(),
      original_data: testData,
    };

    fs.writeFileSync(path.join(testDataDir, "encrypted_test_data.json"), JSON.stringify(testOutput, null, 2)); // 基本的な検証
    expect(encrypted.encrypted).toBeTruthy();
    expect(encrypted.nonce).toBeTruthy();
    expect(testOutput.sender_public_key).toBeTruthy();
    expect(nodePublicKey).toBeTruthy();
  });
});
