use ark_crypto_primitives::{
    commitment::pedersen::Commitment, encryption::AsymmetricEncryptionScheme,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub mod encryption;
pub mod mpc_circuits_wasm;
pub mod types;
pub mod werewolf;
pub use encryption::*;
pub use mpc_circuits_wasm::*;
pub use types::*;
pub use werewolf::*;

pub const PERDERSON_WINDOW_SIZE: usize = 256;
pub const PERDERSON_WINDOW_NUM: usize = 1;

#[derive(Clone)]
pub struct Window;
impl ark_crypto_primitives::crh::pedersen::Window for Window {
    const WINDOW_SIZE: usize = PERDERSON_WINDOW_SIZE;
    const NUM_WINDOWS: usize = PERDERSON_WINDOW_NUM;
}

type PedersenComScheme = Commitment<ark_ed_on_bls12_377::EdwardsProjective, Window>;
type PedersenParam = <PedersenComScheme as ark_crypto_primitives::CommitmentScheme>::Parameters;
type PedersenCommitment = <PedersenComScheme as ark_crypto_primitives::CommitmentScheme>::Output;
type PedersenRandomness =
    <PedersenComScheme as ark_crypto_primitives::CommitmentScheme>::Randomness;

type ElGamalScheme =
    ark_crypto_primitives::encryption::elgamal::ElGamal<ark_ed_on_bls12_377::EdwardsProjective>;
type ElGamalParam = <ElGamalScheme as AsymmetricEncryptionScheme>::Parameters;
type ElGamalPubKey = <ElGamalScheme as AsymmetricEncryptionScheme>::PublicKey;
type ElGamalRandomness = <ElGamalScheme as AsymmetricEncryptionScheme>::Randomness;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeKey {
    pub node_id: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateInput {
    target_id: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretSharingScheme {
    pub total_shares: usize,
    pub modulus: u64,
}

#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn voting_split_and_encrypt(input: JsValue) -> Result<JsValue, JsValue> {
    // Deserialize JsValue into Rust struct
    let input: AnonymousVotingInput = serde_wasm_bindgen::from_value(input)?;
    // Generate and encrypt shares
    let result = AnonymousVotingEncryption::create_encrypted_shares(&input)?;
    // Serialize the result into JsValue(String)
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn key_publicize(input: JsValue) -> Result<JsValue, JsValue> {
    let input: KeyPublicizeInput = serde_wasm_bindgen::from_value(input)?;
    let result = KeyPublicizeEncryption::create_encrypted_shares(&input)?;
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn role_assignment(input: JsValue) -> Result<JsValue, JsValue> {
    let input: RoleAssignmentInput = serde_wasm_bindgen::from_value(input)?;
    let result = RoleAssignmentEncryption::create_encrypted_shares(&input)?;
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn divination(input: JsValue) -> Result<JsValue, JsValue> {
    let input: DivinationInput = serde_wasm_bindgen::from_value(input)?;
    let result = DivinationEncryption::create_encrypted_shares(&input)?;
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[wasm_bindgen]
pub fn winning_judgement(input: JsValue) -> Result<JsValue, JsValue> {
    let input: WinningJudgementInput = serde_wasm_bindgen::from_value(input)?;
    let result = WinningJudgementEncryption::create_encrypted_shares(&input)?;
    let json_str = serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialize error: {}", e)))?;
    Ok(JsValue::from_str(&json_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::*;
    use ark_bls12_377::Fr;
    use ark_crypto_primitives::CommitmentScheme;
    use ark_ff::{BigInteger, PrimeField, Zero};
    use ark_std::UniformRand;
    use base64::encode;
    use crypto_box::SecretKey;
    use rand::{rngs::OsRng, thread_rng};

    #[test]
    fn test_anonymous_voting_input_serialize() {
        let pedersen_param = PedersenComScheme::setup(&mut ark_std::test_rng()).unwrap();

        let secret_key = SecretKey::generate(&mut OsRng);
        let public_key = secret_key.public_key();

        let input = AnonymousVotingInput {
            private_input: AnonymousVotingPrivateInput {
                id: 1,
                is_target_id: vec![Fr::zero(); 3],
                player_randomness: Fr::from(42),
            },
            public_input: AnonymousVotingPublicInput {
                pedersen_param,
                player_commitment: vec![
                    PedersenCommitment::default(),
                    PedersenCommitment::default(),
                    PedersenCommitment::default(),
                ],
                player_num: 3, // Assuming 3 players for this test
            },
            node_keys: vec![
                NodeKey {
                    node_id: "node1".to_string(),
                    // public_key: "key1".to_string(),
                    public_key: encode(public_key.to_bytes()),
                },
                NodeKey {
                    node_id: "node2".to_string(),
                    public_key: encode(public_key.to_bytes()),
                },
                NodeKey {
                    node_id: "node3".to_string(),
                    public_key: encode(public_key.to_bytes()),
                },
            ],
            scheme: SecretSharingScheme {
                total_shares: 3,
                modulus: 97,
            },
        };

        let json = serde_json::to_string(&input).unwrap();
        std::fs::write("test_output2.json", &json).unwrap();
        let expected = json.clone();
        let json = &std::fs::read_to_string("test_output2.json").unwrap();
        let read_input: AnonymousVotingInput = serde_json::from_str(json).unwrap();

        assert_eq!(read_input.private_input.id, input.private_input.id);
        assert_eq!(
            read_input.private_input.is_target_id,
            input.private_input.is_target_id
        );
        assert_eq!(
            read_input.private_input.player_randomness,
            input.private_input.player_randomness
        );
        assert_eq!(read_input.node_keys.len(), input.node_keys.len());
        for (i, key) in read_input.node_keys.iter().enumerate() {
            assert_eq!(key.node_id, input.node_keys[i].node_id);
            assert_eq!(key.public_key, input.node_keys[i].public_key);
        }
        assert_eq!(read_input.scheme.total_shares, input.scheme.total_shares);
        assert_eq!(read_input.scheme.modulus, input.scheme.modulus);
        // assert_eq!(
        //     read_input.public_input.pedersen_param,
        //     input.public_input.pedersen_param
        // );
    }

    use std::fs;
    use std::path::Path;

    #[test]
    #[ignore]
    fn test_pedersen_param_serialization() {
        // Create pedersen parameters
        let mut rng = thread_rng();
        let pedersen_params = PedersenComScheme::setup(&mut rng).unwrap();

        // Convert to JSON
        let json = serde_json::to_string_pretty(&pedersen_params).unwrap();

        // Write to file
        let path = "test_pedersen_params.json";
        fs::write(path, json).unwrap();

        // Verify file exists
        assert!(Path::new(path).exists());

        // Read from file and deserialize
        let json_from_file = fs::read_to_string(path).unwrap();
        let loaded_params: PedersenParam = serde_json::from_str(&json_from_file).unwrap();

        // Verify parameters match
        assert_eq!(pedersen_params.generators, loaded_params.generators);
        assert_eq!(
            pedersen_params.randomness_generator,
            loaded_params.randomness_generator
        );
    }

    #[test]
    #[ignore]
    fn test_randomness_commitment_serialization() {
        // user num
        let n = 3;

        let mut rng = thread_rng();

        for i in 0..n {
            let randomness = PedersenRandomness::rand(&mut rng);

            let path = "test_pedersen_params.json";

            let json_from_file = fs::read_to_string(path).unwrap();
            let loaded_params: PedersenParam = serde_json::from_str(&json_from_file).unwrap();

            let message = Fr::default().into_repr().to_bytes_le();
            let commitment =
                PedersenComScheme::commit(&loaded_params, &message, &randomness).unwrap();

            // serialize
            let json = serde_json::to_string_pretty(&randomness).unwrap();

            let path = format!("pedersen_randomness_{}.json", i);
            fs::write(path, json).unwrap();

            let json = serde_json::to_string_pretty(&commitment).unwrap();

            let path = format!("pedersen_commitment_{}.json", i);
            fs::write(path, json).unwrap();
        }
    }
}
