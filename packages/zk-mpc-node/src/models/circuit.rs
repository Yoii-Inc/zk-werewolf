use ark_bls12_377::Fr;
use ark_ff::PrimeField;
use ark_relations::r1cs::ConstraintSynthesizer;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BuiltinCircuitInputs {
    MySimpleCircuit { a: u32, b: u32 },
    DivinationCircuit { mpc_input: u32 },
    AnonymousVotingCircuit { a: u32 },
    WinningJudgeCircuit { a: u32 },
    RoleAssignmentCircuit { a: u32 },
    KeyPublicizeCircuit { a: u32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitInputs {
    Built(BuiltinCircuitInputs),
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum CircuitType {
    MySimpleCircuit,
    DivinationCircuit,
    AnonymousVotingCircuit,
    WinningJudgeCircuit,
    RoleAssignmentCircuit,
    KeyPublicizeCircuit,
}

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
    pub fn create_local_circuit(request: &ProofRequest) -> impl ConstraintSynthesizer<Fr> {
        match request.circuit_type {
            CircuitIdentifier::Built(BuiltinCircuit::MySimple(_)) => {
                MySimpleCircuit { a: None, b: None }
            }
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn create_mpc_circuit(request: &ProofRequest) -> impl ConstraintSynthesizer<MFr> {
        match &request.circuit_type {
            CircuitIdentifier::Built(BuiltinCircuit::MySimple(my_simple_circuit)) => {
                my_simple_circuit.clone()
            }
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
