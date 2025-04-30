use ark_bls12_377::Fr;
use ark_ff::PrimeField;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use mpc_algebra::Reveal;
use serde::{Deserialize, Serialize};
use zk_mpc::{
    circuits::{
        circuit::MySimpleCircuit, AnonymousVotingCircuit, DivinationCircuit, ElGamalLocalOrMPC,
        KeyPublicizeCircuit, LocalOrMPC, RoleAssignmentCircuit, WinningJudgeCircuit,
    },
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
                    BuiltinCircuit::Divination(DivinationCircuit { mpc_input: todo!() })
                }
                BuiltinCircuit::AnonymousVoting(ref c) => {
                    BuiltinCircuit::AnonymousVoting(AnonymousVotingCircuit {
                        is_target_id: todo!(),
                        is_most_voted_id: todo!(),
                        pedersen_param: todo!(),
                        player_randomness: todo!(),
                        player_commitment: todo!(),
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
            // CircuitIdentifier::Built(BuiltinCircuit::MySimple(_)) => {
            //     MySimpleCircuit { a: None, b: None }
            // }
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
            _ => panic!("Unsupported circuit type for create_local_circuit"),
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
