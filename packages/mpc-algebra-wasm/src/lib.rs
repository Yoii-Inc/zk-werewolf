use base64::decode;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::box_;
use sodiumoxide::crypto::box_::PublicKey;
use sodiumoxide::crypto::sealedbox;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
pub mod input;
pub use input::*;

impl AnonymousVotingInput {
    pub fn new(voter_id: u64, candidate_id: u64) -> Self {
        Self {
            voter_id,
            candidate_id,
        }
    }
}

#[wasm_bindgen]
pub struct SharingScheme {
    pub num_parties: usize,
    pub modulus: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeKey {
    node_id: String,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct VoteShare {
    node_id: String,
    encrypted_share: String,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptAndShareInput {
    private_input: PrivateInput,
    public_input: PublicInput,
    node_keys: Vec<NodeKey>,
    scheme: SecretSharingScheme,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateInput {
    target_id: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PublicInput {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretSharingScheme {
    total_shares: usize,
    modulus: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptedVoteResult {
    shares: Vec<VoteShare>,
    public_input: PublicVoteInput,
}

#[derive(Serialize, Deserialize)]
pub struct PublicVoteInput {
    pedersen_param: String,
    player_commitment: Vec<String>,
}

// シンプルな分割: secret = s1 + s2 + ... + sn (mod modulus)
fn split(secret: u64, total_shares: usize, modulus: u64) -> Vec<u64> {
    let mut shares = Vec::with_capacity(total_shares);
    let mut sum = 0u64;
    let mut rng = rand::thread_rng();

    for i in 0..(total_shares - 1) {
        let share = rand::Rng::gen_range(&mut rng, 0..modulus);
        shares.push(share);
        sum = (sum + share) % modulus;
    }
    let last_share = (modulus + secret - sum) % modulus;
    shares.push(last_share);

    shares
}

fn create_encrypted_shares(input: &EncryptAndShareInput) -> Result<EncryptedVoteResult, JsValue> {
    // シェアの生成とペダーセンコミットメントの計算をここで実装
    let mut shares = Vec::new(); // シェアの初期化

    // Shamirの秘密分散法によるシェア生成のダミー実装
    let total_shares = input.scheme.total_shares;
    let modulus = input.scheme.modulus;
    let secret = input.private_input.target_id as u64;

    let plain_shares = split(secret, total_shares, modulus);

    // 各ノードに対してシェアを生成
    for (i, node_key) in input.node_keys.iter().enumerate() {
        let recipient_key_bytes = decode(node_key.public_key.clone())
            .map_err(|e| JsValue::from_str(&format!("Base64 decode error: {}", e)))?;
        let recipient_key = PublicKey::from_slice(&recipient_key_bytes)
            .ok_or_else(|| JsValue::from_str("Invalid recipient public key"))?;

        let encrypted_share = sealedbox::seal(&plain_shares[i].to_ne_bytes(), &recipient_key);

        shares.push(VoteShare {
            node_id: node_key.node_id.clone(),
            encrypted_share: String::from_utf8(encrypted_share).unwrap(),
        });
    }

    Ok(EncryptedVoteResult {
        shares,
        public_input: PublicVoteInput {
            pedersen_param: "dummy_param".to_string(),
            player_commitment: vec!["dummy_commitment".to_string()],
        },
    })
}

#[wasm_bindgen]
pub fn encrypt_and_share(input: JsValue) -> Result<JsValue, JsValue> {
    // JsValueからRustの構造体にデシリアライズ
    let input: EncryptAndShareInput = serde_wasm_bindgen::from_value(input)?;

    // シェアの生成と暗号化
    let result = create_encrypted_shares(&input)?;

    // 結果をJsValueにシリアライズ
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
