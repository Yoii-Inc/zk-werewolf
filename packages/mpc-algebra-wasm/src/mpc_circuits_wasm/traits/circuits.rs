use crate::*;

use ark_ff::PrimeField;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> MpcCircuit<F>
    for AnonymousVotingCircuit<F>
{
    type Private = AnonymousVotingPrivateInput<F>;
    type Public = AnonymousVotingPublicInput<F>;

    fn combine_inputs(individuals: Vec<Self::Private>, public: Self::Public) -> Self {
        AnonymousVotingCircuit {
            private_input: individuals,
            public_input: public,
        }
    }

    fn validate(&self) -> Result<(), anyhow::Error> {
        // Implement validation logic here
        Ok(())
    }
}
