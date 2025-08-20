use ark_bls12_377::Fr;
use ark_serialize::CanonicalSerialize;
use mpc_algebra::FromLocal;
use mpc_algebra::{crh::pedersen, reveal::Reveal};
use zk_mpc::circuits::{circuit, ElGamalLocalOrMPC};
use zk_mpc::{
    circuits::{circuit::MySimpleCircuit, LocalOrMPC},
    marlin::MFr,
};

use mpc_algebra_wasm::{
    AnonymousVotingEncryption, CircuitEncryptedInputIdentifier, NodeEncryptedShare,
    SplitAndEncrypt, WinningJudgementEncryption,
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
                // let player_num = c[0].public_input.player_num;
                // let alive_player_num = c.len();

                // BuiltinCircuit::Divination(DivinationCircuit {
                //     private_input: (0..alive_player_num)
                //         .map(|_| DivinationPrivateInput::<Fr> {
                //             id: 0,
                //             is_werewolf: Fr::default(),
                //             is_target: vec![Fr::default(); player_num],
                //             randomness: <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalRandomness::default(),
                //         })
                //         .collect::<Vec<_>>(),
                //     public_input: DivinationPublicInput::<Fr> {
                //         pedersen_param: c[0].public_input.pedersen_param.clone(),
                //         player_commitment: c[0].public_input.player_commitment.clone(),
                //         player_num,
                //     },
                // })
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
            BuiltinCircuit::WinningJudge(circuit) => {
                let mut inputs = Vec::new();

                let num_alive = Fr::from(circuit.private_input.len() as u32);

                // let game_state = circuit.calculate_output();

                inputs.push(num_alive);
                // inputs.push(game_state.sync_reveal());
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

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&roles, &mut buffer).unwrap();
                buffer
            }
            _ => panic!("Unsupported circuit type for get_circuit_outputs"),
        }
    }
}
