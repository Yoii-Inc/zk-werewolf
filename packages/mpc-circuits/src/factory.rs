use ark_bls12_377::Fr;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use ark_std::{test_rng, UniformRand};
use mpc_algebra::{crh::pedersen, reveal::Reveal};
use mpc_algebra::{reveal, FromLocal};
use zk_mpc::circuits::{circuit, ElGamalLocalOrMPC};
use zk_mpc::{
    circuits::{circuit::MySimpleCircuit, LocalOrMPC},
    marlin::MFr,
};

use mpc_algebra_wasm::{
    AnonymousVotingEncryption, CircuitEncryptedInputIdentifier, DivinationEncryption,
    KeyPublicizeEncryption, NodeEncryptedShare, RoleAssignmentEncryption, SplitAndEncrypt,
    WinningJudgementEncryption,
};

use crate::*;

pub struct CircuitFactory;

impl CircuitFactory {
    pub fn create_local_circuit(
        circuit_type: &CircuitEncryptedInputIdentifier,
    ) -> BuiltinCircuit<Fr> {
        match circuit_type {
            CircuitEncryptedInputIdentifier::Divination(c) => {
                let player_num = c[0].public_input.player_num;
                let alive_player_num = c.len();
                let rng = &mut test_rng();

                let elgamal_randomness =
                    <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalRandomness::rand(rng);

                BuiltinCircuit::Divination(DivinationCircuit {
                    private_input: (0..alive_player_num)
                        .map(|_| DivinationPrivateInput::<Fr> {
                            id: 0,
                            is_werewolf: Fr::default(),
                            is_target: vec![Fr::default(); player_num],
                            randomness: elgamal_randomness.clone(),
                        })
                        .collect::<Vec<_>>(),
                    public_input: DivinationPublicInput::<Fr> {
                        pedersen_param: c[0].public_input.pedersen_param.clone(),
                        elgamal_param: c[0].public_input.elgamal_param.clone(),
                        pub_key: c[0].public_input.pub_key,
                        player_num,
                    },
                })
            }
            CircuitEncryptedInputIdentifier::AnonymousVoting(c) => {
                let player_num = c[0].public_input.player_num;
                let alive_player_num = c.len();

                BuiltinCircuit::AnonymousVoting(AnonymousVotingCircuit {
                    private_input: (0..alive_player_num)
                        .map(|_| AnonymousVotingPrivateInput::<Fr> {
                            id: 0,
                            is_target_id: vec![Fr::default(); player_num],
                            player_randomness: Fr::default(),
                        })
                        .collect::<Vec<_>>(),
                    public_input: AnonymousVotingPublicInput::<Fr> {
                        pedersen_param: c[0].public_input.pedersen_param.clone(),
                        player_commitment: c[0].public_input.player_commitment.clone(),
                        player_num,
                    },
                })
            }
            CircuitEncryptedInputIdentifier::WinningJudge(ref c) => {
                let alive_player_num = c.len();

                BuiltinCircuit::WinningJudge(WinningJudgementCircuit {
                    private_input: (0..alive_player_num)
                        .map(|_| WinningJudgementPrivateInput::<Fr> {
                            id: 0,
                            am_werewolf: Fr::default(),
                            player_randomness: Fr::default(),
                        })
                        .collect::<Vec<_>>(),
                    public_input: WinningJudgementPublicInput::<Fr> {
                        pedersen_param: c[0].public_input.pedersen_param.clone(),
                        player_commitment: c[0].public_input.player_commitment.clone(),
                    },
                })
            }
            CircuitEncryptedInputIdentifier::RoleAssignment(ref c) => {
                let player_num = c.len();

                let mut rng = ark_std::test_rng();

                let public_input = c[0].public_input.clone();

                let n = public_input.grouping_parameter.get_num_players();
                let m = public_input.grouping_parameter.get_num_groups();
                BuiltinCircuit::RoleAssignment(RoleAssignmentCircuit {
                    private_input: (0..player_num)
                        .map(|_| RoleAssignmentPrivateInput::<Fr> {
                            id: 0,
                            shuffle_matrices: nalgebra::DMatrix::<Fr>::zeros(n + m, n + m),
                            player_randomness: Fr::default(),
                            randomness:
                                ark_crypto_primitives::commitment::pedersen::Randomness::rand(
                                    &mut rng,
                                ),
                        })
                        .collect::<Vec<_>>(),
                    public_input: RoleAssignmentPublicInput::<Fr> {
                        num_players: public_input.num_players,
                        max_group_size: public_input.num_players,
                        tau_matrix: public_input.tau_matrix,
                        role_commitment: public_input.role_commitment,
                        player_commitment: public_input.player_commitment,
                        pedersen_param: public_input.pedersen_param,
                        grouping_parameter: public_input.grouping_parameter.clone(),
                    },
                })
            }
            CircuitEncryptedInputIdentifier::KeyPublicize(ref c) => {
                let alive_player_num = c.len();

                BuiltinCircuit::KeyPublicize(KeyPublicizeCircuit {
                    private_input: (0..alive_player_num)
                        .map(|_| KeyPublicizePrivateInput::<Fr>::default())
                        .collect::<Vec<_>>(),
                    public_input: KeyPublicizePublicInput::<Fr> {
                        pedersen_param: c[0].public_input.pedersen_param.clone(),
                    },
                })
            }

            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn create_mpc_circuit(
        circuit_type: &CircuitEncryptedInputIdentifier,
        my_node_id: &str,
        secret_key: &str,
    ) -> BuiltinCircuit<MFr> {
        match circuit_type {
            CircuitEncryptedInputIdentifier::Divination(circuit) => {
                let mut private_input = Vec::new();

                for i in 0..circuit.len() {
                    let private_encrypted_input = circuit[i]
                        .shares
                        .iter()
                        .find(|share| share.node_id == my_node_id)
                        .expect("No share found for this node");

                    // mpc-algebra-wasmにおけるcreate_encrypted_sharesの反転が必要。
                    let decrypted_input =
                        DivinationEncryption::decrypt(private_encrypted_input, secret_key)
                            .expect("Failed to decrypt input");

                    private_input.push(DivinationPrivateInput::<MFr> {
                        id: decrypted_input.id,
                        is_target: decrypted_input
                            .is_target
                            .iter()
                            .map(|&x| MFr::from_add_shared(x))
                            .collect(),
                        is_werewolf: MFr::from_add_shared(decrypted_input.is_werewolf),
                        randomness:
                            <MFr as ElGamalLocalOrMPC<MFr>>::ElGamalRandomness::from_add_shared(
                                decrypted_input.randomness,
                            ),
                    });
                }

                BuiltinCircuit::Divination(DivinationCircuit {
                    private_input,
                    public_input: DivinationPublicInput::<MFr> {
                        pedersen_param: <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(
                            &circuit[0].public_input.pedersen_param,
                        ),
                        elgamal_param: <MFr as ElGamalLocalOrMPC<MFr>>::ElGamalParam::from_public(
                            circuit[0].public_input.elgamal_param.clone(),
                        ),
                        pub_key: <MFr as ElGamalLocalOrMPC<MFr>>::ElGamalPubKey::from_public(
                            circuit[0].public_input.pub_key,
                        ),
                        player_num: circuit[0].public_input.player_num,
                    },
                })
            }
            CircuitEncryptedInputIdentifier::AnonymousVoting(circuit) => {
                // private_input部分は復号化してそのまま入れるイメージ。

                // let my_node_id = "0";

                let mut private_input = Vec::new();

                for i in 0..circuit.len() {
                    let private_encrypted_input = circuit[i]
                        .shares
                        .iter()
                        .find(|share| share.node_id == my_node_id)
                        .expect("No share found for this node");

                    // mpc-algebra-wasmにおけるcreate_encrypted_sharesの反転が必要。
                    let decrypted_input =
                        AnonymousVotingEncryption::decrypt(private_encrypted_input, secret_key)
                            .expect("Failed to decrypt input");

                    private_input.push(AnonymousVotingPrivateInput::<MFr> {
                        id: decrypted_input.id,
                        is_target_id: decrypted_input
                            .is_target_id
                            .iter()
                            .map(|&x| MFr::from_add_shared(x))
                            .collect(),
                        player_randomness: MFr::from_add_shared(decrypted_input.player_randomness),
                    });
                }

                BuiltinCircuit::AnonymousVoting(AnonymousVotingCircuit {
                    private_input,
                    public_input: AnonymousVotingPublicInput::<MFr> {
                        pedersen_param: <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(
                            &circuit[0].public_input.pedersen_param,
                        ),
                        player_commitment: circuit[0]
                            .public_input
                            .player_commitment
                            .iter()
                            .map(|c| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_local(&c))
                            .collect::<Vec<_>>(),
                        player_num: circuit[0].public_input.player_num,
                    },
                })
            }
            CircuitEncryptedInputIdentifier::WinningJudge(circuit) => {
                // private_input部分は復号化してそのまま入れるイメージ。

                // let my_node_id = "0";

                let mut private_input = Vec::new();

                for i in 0..circuit.len() {
                    let private_encrypted_input = circuit[i]
                        .shares
                        .iter()
                        .find(|share| share.node_id == my_node_id)
                        .expect("No share found for this node");

                    // mpc-algebra-wasmにおけるcreate_encrypted_sharesの反転が必要。
                    let decrypted_input =
                        WinningJudgementEncryption::decrypt(private_encrypted_input, secret_key)
                            .expect("Failed to decrypt input");

                    private_input.push(WinningJudgementPrivateInput::<MFr> {
                        id: decrypted_input.id,
                        am_werewolf: MFr::from_add_shared(decrypted_input.am_werewolf),
                        player_randomness: MFr::from_add_shared(decrypted_input.player_randomness),
                    });
                }

                BuiltinCircuit::WinningJudge(WinningJudgementCircuit {
                    private_input,
                    public_input: WinningJudgementPublicInput::<MFr> {
                        pedersen_param: <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(
                            &circuit[0].public_input.pedersen_param,
                        ),
                        player_commitment: circuit[0]
                            .public_input
                            .player_commitment
                            .iter()
                            .map(|c| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_local(&c))
                            .collect::<Vec<_>>(),
                    },
                })
            }
            CircuitEncryptedInputIdentifier::RoleAssignment(circuit) => {
                let mut private_input: Vec<RoleAssignmentPrivateInput<MFr>> = Vec::new();

                for i in 0..circuit.len() {
                    let private_encrypted_input = circuit[i]
                        .shares
                        .iter()
                        .find(|share| share.node_id == my_node_id)
                        .expect("No share found for this node");

                    // mpc-algebra-wasmにおけるcreate_encrypted_sharesの反転が必要。
                    let decrypted_input =
                        RoleAssignmentEncryption::decrypt(private_encrypted_input, secret_key)
                            .expect("Failed to decrypt input");

                    private_input.push(RoleAssignmentPrivateInput::<MFr> {
                        id: decrypted_input.id,
                        shuffle_matrices: decrypted_input
                            .shuffle_matrices
                            .map(MFr::from_add_shared),
                        player_randomness: MFr::from_add_shared(decrypted_input.player_randomness),
                        randomness: <MFr as LocalOrMPC<MFr>>::PedersenRandomness::from_add_shared(
                            decrypted_input.randomness,
                        ),
                    });
                }

                let grouping_parameter = circuit[0].public_input.grouping_parameter.clone();

                BuiltinCircuit::RoleAssignment(RoleAssignmentCircuit {
                    private_input,
                    public_input: RoleAssignmentPublicInput::<MFr> {
                        num_players: circuit[0].public_input.num_players,
                        max_group_size: circuit[0].public_input.max_group_size,
                        tau_matrix: grouping_parameter.generate_tau_matrix(),
                        role_commitment: circuit[0]
                            .public_input
                            .role_commitment
                            .iter()
                            .map(|c| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_local(&c))
                            .collect::<Vec<_>>(),

                        player_commitment: circuit[0]
                            .public_input
                            .player_commitment
                            .iter()
                            .map(|c| <MFr as LocalOrMPC<MFr>>::PedersenCommitment::from_local(&c))
                            .collect::<Vec<_>>(),
                        pedersen_param: <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(
                            &circuit[0].public_input.pedersen_param,
                        ),
                        grouping_parameter,
                    },
                })
            }
            CircuitEncryptedInputIdentifier::KeyPublicize(circuit) => {
                // private_input部分は復号化してそのまま入れるイメージ。

                // let my_node_id = "0";

                let mut private_input = Vec::new();

                for i in 0..circuit.len() {
                    let private_encrypted_input = circuit[i]
                        .shares
                        .iter()
                        .find(|share| share.node_id == my_node_id)
                        .expect("No share found for this node");

                    // mpc-algebra-wasmにおけるcreate_encrypted_sharesの反転が必要。
                    let decrypted_input =
                        KeyPublicizeEncryption::decrypt(private_encrypted_input, secret_key)
                            .expect("Failed to decrypt input");

                    private_input.push(KeyPublicizePrivateInput::<MFr> {
                        id: decrypted_input.id,
                        pub_key_or_dummy_x: MFr::from_add_shared(
                            decrypted_input.pub_key_or_dummy_x,
                        ),
                        pub_key_or_dummy_y: MFr::from_add_shared(
                            decrypted_input.pub_key_or_dummy_y,
                        ),
                        is_fortune_teller: MFr::from_add_shared(decrypted_input.is_fortune_teller),
                    });
                }

                BuiltinCircuit::KeyPublicize(KeyPublicizeCircuit {
                    private_input,
                    public_input: KeyPublicizePublicInput::<MFr> {
                        pedersen_param: <MFr as LocalOrMPC<MFr>>::PedersenParam::from_local(
                            &circuit[0].public_input.pedersen_param,
                        ),
                    },
                })
            }
            _ => panic!("Unsupported circuit type for create_mpc_circuit"),
        }
    }

    // TODO: implement for all circuits
    pub fn create_verify_inputs(circuit_type: &BuiltinCircuit<MFr>) -> Vec<Fr> {
        match circuit_type {
            // CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit)) => {
            //     vec![circuit.a.unwrap().sync_reveal() * circuit.b.unwrap().sync_reveal()]
            // }
            BuiltinCircuit::Divination(circuit) => {
                let mut inputs = Vec::new();
                let is_target_werewolf = circuit.calculate_output();

                let revealed_is_target_werewolf = is_target_werewolf.sync_reveal();

                inputs.push(circuit.public_input.elgamal_param.generator.sync_reveal().x);
                inputs.push(circuit.public_input.elgamal_param.generator.sync_reveal().y);

                inputs.push(circuit.public_input.pub_key.sync_reveal().x);
                inputs.push(circuit.public_input.pub_key.sync_reveal().y);

                // elgamal ciphertext
                inputs.push(revealed_is_target_werewolf.0.x);
                inputs.push(revealed_is_target_werewolf.0.y);
                inputs.push(revealed_is_target_werewolf.1.x);
                inputs.push(revealed_is_target_werewolf.1.y);
                inputs
            }
            BuiltinCircuit::AnonymousVoting(circuit) => {
                let mut inputs = Vec::new();
                // let mut inputs = circuit
                //     .player_commitment
                //     .iter()
                //     .flat_map(|c| {
                //         let d = c.to_local();
                //         vec![d.x, d.y]
                //     })
                //     .collect::<Vec<_>>();

                let most_voted_id = circuit.calculate_output();

                inputs.push(most_voted_id.sync_reveal());
                inputs
            }
            BuiltinCircuit::WinningJudge(circuit) => {
                let mut inputs = Vec::new();

                let num_alive = Fr::from(circuit.private_input.len() as u32);

                let game_state = circuit.calculate_output();

                inputs.push(num_alive);
                inputs.push(game_state.sync_reveal());
                inputs
            }
            BuiltinCircuit::RoleAssignment(circuit) => {
                let mut inputs = Vec::new();

                let tau_matrix = &circuit.public_input.tau_matrix;

                for i in 0..tau_matrix.nrows() {
                    for j in 0..tau_matrix.ncols() {
                        inputs.push(tau_matrix[(i, j)].sync_reveal());
                    }
                }

                inputs
            }
            BuiltinCircuit::KeyPublicize(circuit) => {
                let mut inputs = Vec::new();
                let (pub_key_x, pub_key_y) = circuit.calculate_output();

                let revealed_pub_key_x = pub_key_x.sync_reveal();
                let revealed_pub_key_y = pub_key_y.sync_reveal();

                // 公開鍵のX座標とY座標を入力として返す
                // inputs.push(revealed_pub_key_x);
                // inputs.push(revealed_pub_key_y);
                inputs
            }
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn get_circuit_outputs(circuit_type: &BuiltinCircuit<MFr>) -> Vec<u8> {
        match circuit_type {
            BuiltinCircuit::Divination(circuit) => {
                let is_target_werewolf = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&is_target_werewolf, &mut buffer).unwrap();
                buffer
            }
            BuiltinCircuit::AnonymousVoting(circuit) => {
                let most_voted_id = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&most_voted_id, &mut buffer).unwrap();
                buffer
            }
            BuiltinCircuit::KeyPublicize(circuit) => {
                let public_key = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&public_key, &mut buffer).unwrap();
                buffer
            }
            BuiltinCircuit::WinningJudge(circuit) => {
                let game_state = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&game_state, &mut buffer).unwrap();
                buffer
            }
            BuiltinCircuit::RoleAssignment(circuit) => {
                let roles = circuit.calculate_output().sync_reveal();

                // Vec<Fr>を文字列の配列に変換してJSON化
                let role_strings: Vec<String> = roles
                    .iter()
                    .map(|role_field| role_field.into_repr().to_string())
                    .collect();

                // JSONとしてシリアライズ
                serde_json::to_vec(&role_strings).unwrap()
            }
            _ => panic!("Unsupported circuit type for get_circuit_outputs"),
        }
    }
}
