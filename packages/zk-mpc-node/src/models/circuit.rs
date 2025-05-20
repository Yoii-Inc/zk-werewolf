use ark_bls12_377::Fr;
use ark_ff::{BigInteger, PrimeField};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::CanonicalSerialize;
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_algebra::{crh::pedersen, Reveal, ToLocal};
use serde::{Deserialize, Serialize};
use zk_mpc::{
    circuits::{
        circuit::MySimpleCircuit, AnonymousVotingCircuit, DivinationCircuit, ElGamalLocalOrMPC,
        KeyPublicizeCircuit, LocalOrMPC, RoleAssignmentCircuit, WinningJudgeCircuit,
    },
    input::{MpcInputTrait, WerewolfMpcInput},
    marlin::MFr,
};

use super::ProofRequest;

#[derive(Clone, Serialize, Deserialize)]
pub enum BuiltinCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    MySimple(MySimpleCircuit<F>),
    Divination(DivinationCircuit<F>),
    AnonymousVoting(AnonymousVotingCircuit<F>),
    WinningJudge(WinningJudgeCircuit<F>),
    RoleAssignment(RoleAssignmentCircuit<F>),
    KeyPublicize(KeyPublicizeCircuit<F>),
}

// TODO: implement Debug correctly for all circuits
impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> std::fmt::Debug for BuiltinCircuit<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuiltinCircuit::MySimple(_) => write!(f, "MySimple"),
            BuiltinCircuit::Divination(_) => write!(f, "Divination"),
            BuiltinCircuit::AnonymousVoting(_) => write!(f, "AnonymousVoting"),
            BuiltinCircuit::WinningJudge(_) => write!(f, "WinningJudge"),
            BuiltinCircuit::RoleAssignment(_) => write!(f, "RoleAssignment"),
            BuiltinCircuit::KeyPublicize(_) => write!(f, "KeyPublicize"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitIdentifier<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    Built(BuiltinCircuit<F>),
    Custom(String),
}

pub struct CircuitFactory;

impl CircuitFactory {
    pub fn create_local_circuit(request: &ProofRequest) -> BuiltinCircuit<Fr> {
        match &request.circuit_type {
            CircuitIdentifier::Built(circuit) => match circuit {
                BuiltinCircuit::MySimple(ref c) => {
                    BuiltinCircuit::MySimple(MySimpleCircuit { a: None, b: None })
                }
                BuiltinCircuit::Divination(ref c) => {
                    let rng = &mut test_rng();
                    let local_input = WerewolfMpcInput::<Fr>::rand(rng);
                    BuiltinCircuit::Divination(DivinationCircuit {
                        mpc_input: local_input,
                    })
                }
                BuiltinCircuit::AnonymousVoting(ref c) => {
                    let pedersen_param = c.pedersen_param.to_local();
                    BuiltinCircuit::AnonymousVoting(AnonymousVotingCircuit {
                        is_target_id: vec![
                            vec![Fr::default(); c.is_target_id[0].len()];
                            c.is_target_id.len()
                        ],
                        pedersen_param: pedersen_param.clone(),
                        player_randomness: vec![Fr::default(); c.player_randomness.len()],
                        player_commitment: vec![
                            <Fr as LocalOrMPC<Fr>>::PedersenComScheme::commit(
                                &pedersen_param,
                                &Fr::default().into_repr().to_bytes_le(),
                                &<Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
                            )
                            .unwrap();
                            c.player_commitment.len()
                        ],
                    })
                }
                BuiltinCircuit::WinningJudge(ref c) => {
                    BuiltinCircuit::WinningJudge(WinningJudgeCircuit {
                        player_commitment: todo!(),
                        player_randomness: todo!(),
                        pedersen_param: todo!(),
                        num_alive: todo!(),
                        am_werewolf: todo!(),
                        game_state: todo!(),
                    })
                }
                BuiltinCircuit::RoleAssignment(ref c) => {
                    BuiltinCircuit::RoleAssignment(RoleAssignmentCircuit {
                        num_players: todo!(),
                        max_group_size: todo!(),
                        pedersen_param: todo!(),
                        tau_matrix: todo!(),
                        role_commitment: todo!(),
                        player_commitment: todo!(),
                        shuffle_matrices: todo!(),
                        randomness: todo!(),
                        player_randomness: todo!(),
                    })
                }
                BuiltinCircuit::KeyPublicize(ref c) => {
                    BuiltinCircuit::KeyPublicize(KeyPublicizeCircuit { mpc_input: todo!() })
                }
            },
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn create_mpc_circuit(request: &ProofRequest) -> BuiltinCircuit<MFr> {
        match &request.circuit_type {
            CircuitIdentifier::Built(circuit) => circuit.clone(),
            _ => panic!("Unsupported circuit type for create_mpc_circuit"),
        }
    }

    pub fn create_verify_inputs(request: &ProofRequest) -> Vec<Fr> {
        match &request.circuit_type {
            CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit)) => {
                vec![circuit.a.unwrap().sync_reveal() * circuit.b.unwrap().sync_reveal()]
            }
            CircuitIdentifier::Built(BuiltinCircuit::AnonymousVoting(circuit)) => {
                let mut inputs = circuit
                    .player_commitment
                    .iter()
                    .flat_map(|c| {
                        let d = c.to_local();
                        vec![d.x, d.y]
                    })
                    .collect::<Vec<_>>();

                let most_voted_id = circuit.calculate_output();

                inputs.push(most_voted_id.sync_reveal());
                inputs
            }
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn get_circuit_outputs(request: &ProofRequest) -> Vec<u8> {
        match &request.circuit_type {
            CircuitIdentifier::Built(BuiltinCircuit::MySimple(circuit)) => {
                let c = circuit.a.unwrap() * circuit.b.unwrap();
                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&c, &mut buffer).unwrap();
                buffer
            }
            CircuitIdentifier::Built(BuiltinCircuit::Divination(circuit)) => {
                let peculiar = circuit.mpc_input.peculiar.clone().unwrap();
                let is_target_vec = peculiar.is_target;
                let is_werewolf_vec = peculiar.is_werewolf;

                let mut sum = MFr::default();
                for (t, w) in is_target_vec.iter().zip(is_werewolf_vec.iter()) {
                    sum += t.input * w.input;
                }
                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&sum, &mut buffer).unwrap();
                buffer
            }
            CircuitIdentifier::Built(BuiltinCircuit::AnonymousVoting(circuit)) => {
                let most_voted_id = circuit.calculate_output().sync_reveal();

                let mut buffer = Vec::new();
                CanonicalSerialize::serialize(&most_voted_id, &mut buffer).unwrap();
                buffer
            }
            CircuitIdentifier::Built(BuiltinCircuit::WinningJudge(circuit)) => {
                // let player_commitment = circuit.player_commitment.clone();
                // let player_randomness = circuit.player_randomness.clone();
                // let pedersen_param = circuit.pedersen_param.clone();
                // let num_alive = circuit.num_alive;
                // let am_werewolf = circuit.am_werewolf;
                // let game_state = circuit.game_state.clone();

                let game_state = circuit.game_state.clone();
                let mut buffer = Vec::new();
                // CanonicalSerialize::serialize(&player_commitment, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&player_randomness, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&pedersen_param, &mut buffer).unwrap();
                buffer
            }
            CircuitIdentifier::Built(BuiltinCircuit::RoleAssignment(circuit)) => {
                let num_players = circuit.num_players;
                let max_group_size = circuit.max_group_size;
                let pedersen_param = circuit.pedersen_param.clone();
                let tau_matrix = circuit.tau_matrix.clone();
                let role_commitment = circuit.role_commitment.clone();
                let player_commitment = circuit.player_commitment.clone();
                let shuffle_matrices = circuit.shuffle_matrices.clone();
                let randomness = circuit.randomness.clone();
                let player_randomness = circuit.player_randomness.clone();
                let mut buffer = Vec::new();
                // CanonicalSerialize::serialize(&num_players, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&max_group_size, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&pedersen_param, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&tau_matrix, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&role_commitment, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&player_commitment, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&shuffle_matrices, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&randomness, &mut buffer).unwrap();
                // CanonicalSerialize::serialize(&player_randomness, &mut buffer).unwrap();
                buffer
            }
            _ => panic!("Unsupported circuit type for get_circuit_outputs"),
        }
    }
}

impl ConstraintSynthesizer<Fr> for BuiltinCircuit<Fr> {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        match self {
            Self::MySimple(c) => c.generate_constraints(cs),
            Self::Divination(c) => c.generate_constraints(cs),
            Self::AnonymousVoting(c) => c.generate_constraints(cs),
            Self::WinningJudge(c) => c.generate_constraints(cs),
            Self::RoleAssignment(c) => c.generate_constraints(cs),
            Self::KeyPublicize(c) => c.generate_constraints(cs),
        }
    }
}

impl ConstraintSynthesizer<MFr> for BuiltinCircuit<MFr> {
    fn generate_constraints(self, cs: ConstraintSystemRef<MFr>) -> Result<(), SynthesisError> {
        match self {
            Self::MySimple(c) => c.generate_constraints(cs),
            Self::Divination(c) => c.generate_constraints(cs),
            Self::AnonymousVoting(c) => c.generate_constraints(cs),
            Self::WinningJudge(c) => c.generate_constraints(cs),
            Self::RoleAssignment(c) => c.generate_constraints(cs),
            Self::KeyPublicize(c) => c.generate_constraints(cs),
        }
    }
}
