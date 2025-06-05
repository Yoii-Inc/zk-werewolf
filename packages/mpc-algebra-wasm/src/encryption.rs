use ark_bls12_377::Fr;
use ark_ff::PubUniformRand;
use ark_ff::Zero;
use ark_ff::{fields::PrimeField, BigInteger};
use base64::decode;
use mpc_algebra::Reveal;
use mpc_circuits::AnonymousVotingPrivateInput;
use mpc_circuits::AnonymousVotingPublicInput;
use mpc_circuits::DivinationPrivateInput;
use mpc_circuits::KeyPublicizePrivateInput;
use mpc_circuits::RoleAssignmentPrivateInput;
use mpc_circuits::WinningJudgementPrivateInput;
use serde::Serialize;
use sodiumoxide::crypto::{box_::PublicKey, sealedbox};
use wasm_bindgen::JsValue;

use crate::{types::*, NodeKey, SecretSharingScheme};

use mpc_algebra::malicious_majority::MpcField;

pub type MFr = MpcField<Fr>;

pub trait SplitAndEncrypt {
    type Input;
    type Output;
    type ShareForNode: Serialize;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode>;

    fn combine(shares: Vec<Self::ShareForNode>) -> Self::Input {
        unimplemented!()
    }

    fn encrypt(plain_share: Self::ShareForNode, key: &NodeKey) -> NodeEncryptedShare {
        let recipient_key_bytes = decode(key.public_key.clone())
            .map_err(|e| JsValue::from_str(&format!("Base64 decode error: {}", e)))
            .unwrap();
        let recipient_key = PublicKey::from_slice(&recipient_key_bytes)
            .ok_or_else(|| JsValue::from_str("Invalid recipient public key"))
            .unwrap();

        let json_string = serde_json::to_string(&plain_share).unwrap();

        let encrypted_share = sealedbox::seal(json_string.as_bytes(), &recipient_key);

        NodeEncryptedShare {
            node_id: key.node_id.clone(),
            encrypted_share: String::from_utf8(encrypted_share).unwrap(),
        }
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

    type ShareForNode = AnonymousVotingPrivateInput<MFr>;

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
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key);

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

    type ShareForNode = KeyPublicizePrivateInput<MFr>;

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
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key);

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

    type ShareForNode = RoleAssignmentPrivateInput<MFr>;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key);

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

    type ShareForNode = DivinationPrivateInput<MFr>;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key);

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

    type ShareForNode = WinningJudgementPrivateInput<MFr>;

    fn split(input: &Self::Input) -> Vec<Self::ShareForNode> {
        todo!()
    }

    fn create_encrypted_shares(input: &Self::Input) -> Result<Self::Output, JsValue> {
        let mut shares = Vec::new();

        let plain_shares = Self::split(input);

        for (i, node_key) in input.node_keys.iter().enumerate() {
            let encrypted_share = Self::encrypt(plain_shares[i].clone(), node_key);

            shares.push(encrypted_share);
        }

        Ok(WinningJudgementOutput {
            shares,
            public_input: input.public_input.clone(),
        })
    }
}

// fn voting_create_encrypted_shares(
//     input: &EncryptAndShareInput,
// ) -> Result<EncryptedVoteResult, JsValue> {
//     // シェアの生成とペダーセンコミットメントの計算をここで実装
//     let mut shares = Vec::new(); // シェアの初期化

//     // Shamirの秘密分散法によるシェア生成のダミー実装
//     let total_shares = input.scheme.total_shares;
//     let modulus = input.scheme.modulus;
//     let secret = input.private_input.target_id as u64;

//     let plain_shares = split(secret, total_shares, modulus);

//     // 各ノードに対してシェアを生成
//     for (i, node_key) in input.node_keys.iter().enumerate() {
//         let recipient_key_bytes = decode(node_key.public_key.clone())
//             .map_err(|e| JsValue::from_str(&format!("Base64 decode error: {}", e)))?;
//         let recipient_key = PublicKey::from_slice(&recipient_key_bytes)
//             .ok_or_else(|| JsValue::from_str("Invalid recipient public key"))?;

//         let encrypted_share = sealedbox::seal(&plain_shares[i].to_ne_bytes(), &recipient_key);

//         shares.push(VoteShare {
//             node_id: node_key.node_id.clone(),
//             encrypted_share: String::from_utf8(encrypted_share).unwrap(),
//         });
//     }

//     Ok(EncryptedVoteResult {
//         shares,
//         public_input: PublicVoteInput {
//             pedersen_param: "dummy_param".to_string(),
//             player_commitment: vec!["dummy_commitment".to_string()],
//         },
//     })
// }

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

fn split_vec_fr(vec_x: Vec<Fr>, scheme: &SecretSharingScheme) -> Vec<Vec<MFr>> {
    vec_x.iter().map(|&x| split_fr(x, scheme)).collect()
}

fn split_fr(x: Fr, scheme: &SecretSharingScheme) -> Vec<MFr> {
    let mut shares = Vec::new();
    let mut sum = MFr::from_add_shared(Fr::zero());

    let rng = &mut rand::thread_rng();

    for i in 0..(scheme.total_shares - 1) {
        let share = MFr::from_add_shared(Fr::pub_rand(rng));
        shares.push(share);
        sum += share;
    }
    let last_share = MFr::from_add_shared(x) - sum;
    shares.push(last_share);

    shares
}

fn combine(shares: Vec<MFr>) -> Fr {
    let mut sum = MFr::from_add_shared(Fr::zero());
    for share in shares {
        sum += share;
    }
    sum.unwrap_as_public()
}

#[cfg(test)]
mod tests {

    use mpc_algebra::crh::pedersen;

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

        let combined: Fr = combine(shares);
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
