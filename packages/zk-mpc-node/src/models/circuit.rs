use ark_bls12_377::Fr;
use ark_relations::r1cs::ConstraintSynthesizer;
use serde::{Deserialize, Serialize};
use zk_mpc::{circuits::circuit::MySimpleCircuit, marlin::MFr};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitIdentifier {
    Built(CircuitType),
    Custom(String),
}

pub struct CircuitFactory;

impl CircuitFactory {
    pub fn create_local_circuit(request: &ProofRequest) -> impl ConstraintSynthesizer<Fr> {
        match request.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &request.inputs {
                CircuitInputs::Built(BuiltinCircuitInputs::MySimpleCircuit { a: _, b: _ }) => {
                    MySimpleCircuit { a: None, b: None }
                }
                _ => panic!("Unsupported circuit type for create_local_circuit"),
            },
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn create_mpc_circuit(request: &ProofRequest) -> impl ConstraintSynthesizer<MFr> {
        match request.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &request.inputs {
                CircuitInputs::Built(BuiltinCircuitInputs::MySimpleCircuit { a, b }) => {
                    MySimpleCircuit {
                        a: Some(MFr::from(*a)),
                        b: Some(MFr::from(*b)),
                    }
                }
                _ => panic!("Unsupported circuit type for create_mpc_circuit"),
            },
            _ => panic!("Unsupported circuit type for create_mpc_circuit"),
        }
    }

    pub fn create_verify_inputs(request: &ProofRequest) -> Vec<Fr> {
        match request.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &request.inputs {
                CircuitInputs::Built(BuiltinCircuitInputs::MySimpleCircuit { a, b }) => {
                    vec![Fr::from(*a) * Fr::from(*b)]
                }
                _ => panic!("Unsupported circuit type for create_local_circuit"),
            },
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }
}
