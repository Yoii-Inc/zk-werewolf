use ark_ff::PrimeField;
use serde::{de::DeserializeOwned, Serialize};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

pub mod circuits;

pub trait MpcCircuit<F> {
    type Private;
    type Public;

    fn combine_inputs(individuals: Vec<Self::Private>, public: Self::Public) -> Self;
    fn validate(&self) -> Result<(), anyhow::Error>;
}
