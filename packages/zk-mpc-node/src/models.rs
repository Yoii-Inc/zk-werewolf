use ark_bls12_377::Fr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;
use zk_mpc::{circuits::circuit::MySimpleCircuit, marlin::MFr};

use ark_relations::r1cs::ConstraintSynthesizer;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
pub struct Opt {
    pub id: usize,
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofRequest {
    pub circuit_type: CircuitIdentifier,
    pub inputs: CircuitInputs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BuiltinCircuitInputs {
    MySimpleCircuit { a: u32, b: u32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitInputs {
    Built(BuiltinCircuitInputs),
    Custom(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum CircuitType {
    MySimpleCircuit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitIdentifier {
    Built(CircuitType),
    Custom(String),
}

impl ProofRequest {
    pub fn create_local_circuit(&self) -> impl ConstraintSynthesizer<Fr> {
        match self.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &self.inputs {
                CircuitInputs::Built(BuiltinCircuitInputs::MySimpleCircuit { a: _, b: _ }) => {
                    MySimpleCircuit { a: None, b: None }
                }
                _ => panic!("Unsupported circuit type for create_local_circuit"),
            },
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }

    pub fn create_mpc_circuit(&self) -> impl ConstraintSynthesizer<MFr> {
        match self.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &self.inputs {
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

    pub fn create_verify_inputs(&self) -> Vec<Fr> {
        match self.circuit_type {
            CircuitIdentifier::Built(CircuitType::MySimpleCircuit) => match &self.inputs {
                CircuitInputs::Built(BuiltinCircuitInputs::MySimpleCircuit { a, b }) => {
                    vec![Fr::from(*a) * Fr::from(*b)]
                }
                _ => panic!("Unsupported circuit type for create_local_circuit"),
            },
            _ => panic!("Unsupported circuit type for create_local_circuit"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofStatus {
    pub state: String,
    pub proof_id: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProofResponse {
    pub success: bool,
    pub message: String,
    pub proof_id: String,
}
