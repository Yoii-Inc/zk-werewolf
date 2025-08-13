use ark_bls12_377::Fr;
use ark_serialize::CanonicalSerialize;
use mpc_algebra::FromLocal;
use mpc_algebra::{crh::pedersen, reveal::Reveal};
use zk_mpc::circuits::circuit;
use zk_mpc::{
    circuits::{circuit::MySimpleCircuit, LocalOrMPC},
    marlin::MFr,
};

use mpc_algebra_wasm::{
    AnonymousVotingEncryption, CircuitEncryptedInputIdentifier, NodeEncryptedShare, SplitAndEncrypt,
};

use crate::*;

pub struct CircuitFactory;

impl CircuitFactory {
    pub fn create_local_circuit(
        circuit_type: &CircuitEncryptedInputIdentifier,
    ) -> BuiltinCircuit<Fr> {
        match circuit_type {
            CircuitEncryptedInputIdentifier::Divination(c) => {
                // BuiltinCircuit::Divination(DivinationCircuit {
                //     mpc_input: c.clone(),
                // })
                todo!()
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
            CircuitEncryptedInputIdentifier::Divination(ref c) => {
                // let rng = &mut test_rng();
                // let local_input = WerewolfMpcInput::<Fr>::rand(rng);
                // BuiltinCircuit::Divination(DivinationCircuit {
                //     mpc_input: local_input,
                // })
                todo!()
            }
            CircuitEncryptedInputIdentifier::AnonymousVoting(ref c) => {
                // let pedersen_param = c.pedersen_param.to_local();
                // BuiltinCircuit::AnonymousVoting(AnonymousVotingCircuit {
                //     is_target_id: vec![
                //         vec![Fr::default(); c.is_target_id[0].len()];
                //         c.is_target_id.len()
                //     ],
                //     pedersen_param: pedersen_param.clone(),
                //     player_randomness: vec![Fr::default(); c.player_randomness.len()],
                //     player_commitment: vec![
                //         <Fr as LocalOrMPC<Fr>>::PedersenComScheme::commit(
                //             &pedersen_param,
                //             &Fr::default().into_repr().to_bytes_le(),
                //             &<Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
                //         )
                //         .unwrap();
                //         c.player_commitment.len()
                //     ],
                // })
                todo!()
            }
            CircuitEncryptedInputIdentifier::WinningJudge(ref c) => {
                // BuiltinCircuit::WinningJudge(WinningJudgeCircuit {
                //     player_commitment: todo!(),
                //     player_randomness: todo!(),
                //     pedersen_param: todo!(),
                //     num_alive: todo!(),
                //     am_werewolf: todo!(),
                //     game_state: todo!(),
                // })
                todo!()
            }
            CircuitEncryptedInputIdentifier::RoleAssignment(ref c) => {
                // BuiltinCircuit::RoleAssignment(RoleAssignmentCircuit {
                //     num_players: todo!(),
                //     max_group_size: todo!(),
                //     pedersen_param: todo!(),
                //     tau_matrix: todo!(),
                //     role_commitment: todo!(),
                //     player_commitment: todo!(),
                //     shuffle_matrices: todo!(),
                //     randomness: todo!(),
                //     player_randomness: todo!(),
                // })
                todo!()
            }
            CircuitEncryptedInputIdentifier::KeyPublicize(ref c) => {
                // BuiltinCircuit::KeyPublicize(KeyPublicizeCircuit { mpc_input: todo!() })
                todo!()
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
            // CircuitIdentifier::Built(circuit) => circuit.clone(),
            CircuitEncryptedInputIdentifier::Divination(circuit) => {
                // BuiltinCircuit::Divination(DivinationCircuit {
                //     mpc_input: circuit.clone(),
                // })
                todo!()
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
            _ => panic!("Unsupported circuit type for create_mpc_circuit"),
        }
    }

    pub fn create_verify_inputs(circuit_type: &BuiltinCircuit<MFr>) -> Vec<Fr> {
        match circuit_type {
            // CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit)) => {
            //     vec![circuit.a.unwrap().sync_reveal() * circuit.b.unwrap().sync_reveal()]
            // }
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
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn get_circuit_outputs(
        circuit_type: &BuiltinCircuit<MFr>,
        // output_type: &ProofOutputType,
    ) -> Vec<u8> {
        match circuit_type {
            // CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit)) => {
            //     let c = circuit.a.unwrap() * circuit.b.unwrap();
            //     let mut buffer = Vec::new();
            //     CanonicalSerialize::serialize(&c, &mut buffer).unwrap();
            //     buffer
            // }
            BuiltinCircuit::Divination(circuit) => {
                // let peculiar = circuit.mpc_input.peculiar.clone().unwrap();
                // let is_target_vec = peculiar.is_target;
                // let is_werewolf_vec = peculiar.is_werewolf;

                // let mut sum = MFr::default();
                // for (t, w) in is_target_vec.iter().zip(is_werewolf_vec.iter()) {
                //     sum += t.input * w.input;
                // }
                // let mut buffer = Vec::new();
                // CanonicalSerialize::serialize(&sum, &mut buffer).unwrap();
                // buffer
                todo!()
            }
            BuiltinCircuit::AnonymousVoting(circuit) => {
                let most_voted_id = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&most_voted_id, &mut buffer).unwrap();
                buffer
            }
            BuiltinCircuit::WinningJudge(circuit) => {
                // // let player_commitment = circuit.player_commitment.clone();
                // // let player_randomness = circuit.player_randomness.clone();
                // // let pedersen_param = circuit.pedersen_param.clone();
                // // let num_alive = circuit.num_alive;
                // // let am_werewolf = circuit.am_werewolf;
                // // let game_state = circuit.game_state.clone();

                // let game_state = circuit.game_state.clone();
                // let mut buffer = Vec::new();
                // // CanonicalSerialize::serialize(&player_commitment, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&player_randomness, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&pedersen_param, &mut buffer).unwrap();
                // buffer
                todo!()
            }
            BuiltinCircuit::RoleAssignment(circuit) => {
                // let num_players = circuit.num_players;
                // let max_group_size = circuit.max_group_size;
                // let pedersen_param = circuit.pedersen_param.clone();
                // let tau_matrix = circuit.tau_matrix.clone();
                // let role_commitment = circuit.role_commitment.clone();
                // let player_commitment = circuit.player_commitment.clone();
                // let shuffle_matrices = circuit.shuffle_matrices.clone();
                // let randomness = circuit.randomness.clone();
                // let player_randomness = circuit.player_randomness.clone();
                // let mut buffer = Vec::new();
                // // CanonicalSerialize::serialize(&num_players, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&max_group_size, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&pedersen_param, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&tau_matrix, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&role_commitment, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&player_commitment, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&shuffle_matrices, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&randomness, &mut buffer).unwrap();
                // // CanonicalSerialize::serialize(&player_randomness, &mut buffer).unwrap();
                // buffer
                todo!()
            }
            _ => panic!("Unsupported circuit type for get_circuit_outputs"),
        }
    }
}
