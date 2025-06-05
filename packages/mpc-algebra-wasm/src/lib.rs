use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub mod encryption;
pub mod types;
pub use encryption::*;
pub use types::*;

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeKey {
    node_id: String,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateInput {
    target_id: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretSharingScheme {
    total_shares: usize,
    modulus: u64,
}

#[wasm_bindgen]
pub fn voting_split_and_encrypt(input: JsValue) -> Result<JsValue, JsValue> {
    // Deserialize JsValue into Rust struct
    let input: AnonymousVotingInput = serde_wasm_bindgen::from_value(input)?;
    // Generate and encrypt shares
    let result = AnonymousVotingEncryption::create_encrypted_shares(&input)?;
    // Serialize the result into JsValue
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

// #[wasm_bindgen]
// pub fn key_publicize(input: JsValue) -> Result<JsValue, JsValue> {
//     let input: KeyPublicizeInput = serde_wasm_bindgen::from_value(input)?;
//     let result = KeyPublicizeEncryption::create_encrypted_shares(&input)?;
//     Ok(serde_wasm_bindgen::to_value(&result)?)
// }

// #[wasm_bindgen]
// pub fn role_assignment(input: JsValue) -> Result<JsValue, JsValue> {
//     let input: RoleAssignmentInput = serde_wasm_bindgen::from_value(input)?;
//     let result = RoleAssignmentEncryption::create_encrypted_shares(&input)?;
//     Ok(serde_wasm_bindgen::to_value(&result)?)
// }

// #[wasm_bindgen]
// pub fn divination(input: JsValue) -> Result<JsValue, JsValue> {
//     let input: DivinationInput = serde_wasm_bindgen::from_value(input)?;
//     let result = DivinationEncryption::create_encrypted_shares(&input)?;
//     Ok(serde_wasm_bindgen::to_value(&result)?)
// }

// #[wasm_bindgen]
// pub fn winning_judgement(input: JsValue) -> Result<JsValue, JsValue> {
//     let input: WinningJudgementInput = serde_wasm_bindgen::from_value(input)?;
//     let result = WinningJudgementEncryption::create_encrypted_shares(&input)?;
//     Ok(serde_wasm_bindgen::to_value(&result)?)
// }
