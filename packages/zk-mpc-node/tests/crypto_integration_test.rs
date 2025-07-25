use base64;
use crypto_box::{
    aead::{Aead, AeadCore, OsRng},
    PublicKey, SalsaBox, SecretKey,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestData {
    voter_id: String,
    target_id: String,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedTestData {
    encrypted: String,
    nonce: String,
    sender_public_key: String,
    original_data: TestData,
}

#[tokio::test]
async fn test_decrypt_frontend_data() -> anyhow::Result<()> {
    // Generate keypair
    let secret_key = SecretKey::generate(&mut OsRng);
    let public_key = PublicKey::from(&secret_key);

    // Encode public key to Base64
    let public_key_base64 = base64::encode(public_key.to_bytes());

    // Save public key to file (for frontend test)
    let key_data = serde_json::json!({
        "nodePublicKey": public_key_base64
    });

    let test_data_dir = PathBuf::from("../../test-data");
    fs::create_dir_all(&test_data_dir)?;

    fs::write(
        test_data_dir.join("node_keys.json"),
        serde_json::to_string_pretty(&key_data)?,
    )?;

    // Determine npm command based on platform
    let npm_command = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let nextjs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("nextjs");

    println!("Running npm test in directory: {:?}", nextjs_dir);

    let output = std::process::Command::new(npm_command)
        .args(["test", "__tests__/crypto/encryption.test.ts"])
        .current_dir(&nextjs_dir)
        .output()?;

    // Display test output
    println!(
        "Frontend test stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    println!(
        "Frontend test stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let status = output.status;

    if !status.success() {
        return Err(anyhow::anyhow!("Frontend test failed"));
    }

    // Wait for file creation
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Load encrypted data from file
    let encrypted_data: EncryptedTestData = serde_json::from_str(&fs::read_to_string(
        test_data_dir.join("encrypted_test_data.json"),
    )?)?;

    // Decrypt the data
    let encrypted_bytes = base64::decode(&encrypted_data.encrypted)?;
    let nonce_bytes = base64::decode(&encrypted_data.nonce)?;
    let sender_public_key_bytes = base64::decode(&encrypted_data.sender_public_key)?;

    let nonce = *crypto_box::Nonce::from_slice(&nonce_bytes);
    let sender_public_key = PublicKey::from_slice(&sender_public_key_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid public key"))?;

    // 復号化処理
    let salsa_box = SalsaBox::new(&sender_public_key, &secret_key);
    let decrypted = salsa_box
        .decrypt(&nonce, encrypted_bytes.as_slice())
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

    // Parse decrypted data as JSON
    let decrypted_json: TestData = serde_json::from_slice(&decrypted)?;

    // Compare with original data
    assert_eq!(
        decrypted_json.voter_id,
        encrypted_data.original_data.voter_id
    );
    assert_eq!(
        decrypted_json.target_id,
        encrypted_data.original_data.target_id
    );
    assert_eq!(
        decrypted_json.timestamp,
        encrypted_data.original_data.timestamp
    );

    Ok(())
}
