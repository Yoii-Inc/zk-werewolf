use crate::mpc_circuits_wasm::*;
use ark_bls12_377::Fr;
use ark_ff::PubUniformRand;
use ark_ff::Zero;
use base64::{decode, encode};
use crypto_box::{
    aead::{Aead, AeadCore},
    PublicKey, SalsaBox, SecretKey,
};
use rand::rngs::OsRng;
use serde::Serialize;
use wasm_bindgen::JsValue;

use crate::{types::*, NodeKey, SecretSharingScheme};

pub trait SplitAndEncrypt {
    type Input;
    type Output;
    type ShareForNode: Serialize;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode>;

    fn combine(shares: Vec<Self::ShareForNode>) -> Self::Input {
        unimplemented!()
    }

    fn encrypt(
        plain_share: Self::ShareForNode,
        key: &NodeKey,
    ) -> Result<NodeEncryptedShare, JsValue> {
        // Base64デコードされた公開鍵をPublicKeyに変換
        let recipient_key_bytes = decode(&key.public_key)
            .map_err(|e| JsValue::from_str(&format!("Base64 decode error: {}", e)))?;

        let recipient_key = PublicKey::from(
            <[u8; 32]>::try_from(recipient_key_bytes.as_slice())
                .map_err(|_| JsValue::from_str("Invalid public key length"))?,
        );

        // エフェメラルキーペアの生成
        let ephemeral_secret = SecretKey::generate(&mut OsRng);
        let box_ = SalsaBox::new(&recipient_key, &ephemeral_secret);

        // シェアデータのシリアライズ
        let plain_data = serde_json::to_vec(&plain_share)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        // 暗号化
        // Get a random nonce to encrypt the message under
        let nonce = SalsaBox::generate_nonce(&mut OsRng);
        let encrypted_data = box_
            .encrypt(&nonce, plain_data.as_slice())
            .map_err(|e| JsValue::from_str(&format!("Encryption error: {}", e)))?;

        // Base64エンコード
        let encrypted_share = encode(&encrypted_data);

        Ok(NodeEncryptedShare {
            node_id: key.node_id.clone(),
            encrypted_share,
        })
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue>;
}

pub(crate) struct AnonymousVotingEncryption;
pub(crate) struct KeyPublicizeEncryption;
pub(crate) struct RoleAssignmentEncryption;
pub(crate) struct DivinationEncryption;
pub(crate) struct WinningJudgementEncryption;

impl SplitAndEncrypt for AnonymousVotingEncryption {
    type Input = AnonymousVotingInput;
    type Output = AnonymousVotingOutput;

    type ShareForNode = AnonymousVotingPrivateInput;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        let scheme = &input.scheme;
        let private_input = &input.private_input;

        let is_target_share = split_vec_fr(private_input.is_target_id.clone(), scheme);
        let player_randomness_share = split_fr(private_input.player_randomness, scheme);

        (0..scheme.total_shares)
            .map(|i| AnonymousVotingPrivateInput {
                id: private_input.id,
                is_target_id: is_target_share[i].clone(),
                player_randomness: player_randomness_share[i],
            })
            .collect::<Vec<_>>()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key)?;

            shares.push(encrypted_share);
        }

        Ok(AnonymousVotingOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
}

impl SplitAndEncrypt for KeyPublicizeEncryption {
    type Input = KeyPublicizeInput;
    type Output = KeyPublicizeOutput;

    type ShareForNode = KeyPublicizePrivateInput;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        let scheme = &input.scheme;
        let private_input = &input.private_input;

        let pub_key_or_dummy_x_share =
            split_vec_fr(private_input.pub_key_or_dummy_x.clone(), scheme);
        let pub_key_or_dummy_y_share =
            split_vec_fr(private_input.pub_key_or_dummy_y.clone(), scheme);
        let is_fortune_teller_share = split_vec_fr(private_input.is_fortune_teller.clone(), scheme);

        (0..scheme.total_shares)
            .map(|i| KeyPublicizePrivateInput {
                id: private_input.id,
                pub_key_or_dummy_x: pub_key_or_dummy_x_share[i].clone(),
                pub_key_or_dummy_y: pub_key_or_dummy_y_share[i].clone(),
                is_fortune_teller: is_fortune_teller_share[i].clone(),
            })
            .collect::<Vec<_>>()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key)?;

            shares.push(encrypted_share);
        }

        Ok(KeyPublicizeOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
}

impl SplitAndEncrypt for RoleAssignmentEncryption {
    type Input = RoleAssignmentInput;
    type Output = RoleAssignmentOutput;

    type ShareForNode = RoleAssignmentPrivateInput;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key)?;

            shares.push(encrypted_share);
        }

        Ok(RoleAssignmentOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
}

impl SplitAndEncrypt for DivinationEncryption {
    type Input = DivinationInput;
    type Output = DivinationOutput;

    type ShareForNode = DivinationPrivateInput;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key)?;

            shares.push(encrypted_share);
        }

        Ok(DivinationOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
}

impl SplitAndEncrypt for WinningJudgementEncryption {
    type Input = WinningJudgementInput;
    type Output = WinningJudgementOutput;

    type ShareForNode = WinningJudgementPrivateInput;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key)?;

            shares.push(encrypted_share);
        }

        Ok(WinningJudgementOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
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

fn split_vec_fr(vec_x: Vec<Fr>, scheme: &SecretSharingScheme) -> Vec<Vec<Fr>> {
    vec_x.iter().map(|&x| split_fr(x, scheme)).collect()
}

fn split_fr(x: Fr, scheme: &SecretSharingScheme) -> Vec<Fr> {
    let mut shares = Vec::new();
    let mut sum = Fr::zero();

    let rng = &mut rand::thread_rng();

    for i in 0..(scheme.total_shares - 1) {
        let share = Fr::pub_rand(rng);
        shares.push(share);
        sum += share;
    }
    let last_share = x - sum;
    shares.push(last_share);

    shares
}

// fn combine(shares: Vec<MFr>) -> Fr {
//     let mut sum = MFr::from_add_shared(Fr::zero());
//     for share in shares {
//         sum += share;
//     }
//     sum.unwrap_as_public()
// }

#[cfg(test)]
mod tests {

    // use mpc_algebra::crh::pedersen;

    use super::*;

    #[test]
    fn test_split_combine_simple() {
        let scheme = SecretSharingScheme {
            total_shares: 3,
            modulus: 100,
        };

        let rng = &mut rand::thread_rng();

        let x = Fr::pub_rand(rng);

        let shares = split_fr(x, &scheme);
        assert_eq!(shares.len(), scheme.total_shares);

        // let combined: Fr = combine(shares);
        let combined: Fr = shares.iter().sum();
        assert_eq!(combined, x);
    }

    // #[test]
    // fn test_split_combine_voting() {
    //     let scheme = SecretSharingScheme {
    //         total_shares: 3,
    //         modulus: 100,
    //     };

    //     let rng = &mut rand::thread_rng();

    //     let x = Fr::pub_rand(rng);

    //     let private_input = AnonymousVotingPrivateInput {
    //         id: 1,
    //         is_target_id: vec![Fr::pub_rand(rng)],
    //         player_randomness: x,
    //     };

    //     let pedersen_param = pedersen::PedersenHash::hash(&[private_input.player_randomness]);

    //     let input = AnonymousVotingInput {
    //         private_input,
    //         public_input: AnonymousVotingPublicInput {
    //             pedersen_param: Fr::zero(),          // Dummy value
    //             player_commitment: vec![Fr::zero()], // Dummy value
    //         },
    //         node_keys: vec![], // Dummy value, not used in this test
    //         scheme,
    //     };

    //     let shares = AnonymousVotingEncryption::split(&input);
    //     assert_eq!(shares.len(), scheme.total_shares);

    //     let combined = AnonymousVotingEncryption::combine(shares);
    //     assert_eq!(combined, input);
    // }
}
