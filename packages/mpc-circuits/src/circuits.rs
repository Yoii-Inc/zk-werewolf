use ark_bls12_377::Fr;
use ark_ff::PrimeField;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use serde::{Deserialize, Serialize};

use zk_mpc::{
    circuits::{circuit::MySimpleCircuit, ElGamalLocalOrMPC, LocalOrMPC},
    marlin::MFr,
};

pub mod anonymous_voting;
pub mod divination;
pub mod key_publicize;
pub mod role_assignment;
pub mod winning_judgement;

pub use anonymous_voting::*;
pub use divination::*;
pub use key_publicize::*;
pub use role_assignment::*;
pub use winning_judgement::*;

#[derive(Clone, Serialize, Deserialize)]
pub enum BuiltinCircuit<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    MySimple(MySimpleCircuit<F>),
    Divination(DivinationCircuit<F>),
    AnonymousVoting(AnonymousVotingCircuit<F>),
    WinningJudge(WinningJudgementCircuit<F>),
    RoleAssignment(RoleAssignmentCircuit<F>),
    KeyPublicize(KeyPublicizeCircuit<F>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CircuitIdentifier<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> {
    Built(BuiltinCircuit<F>),
    Custom(String),
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
