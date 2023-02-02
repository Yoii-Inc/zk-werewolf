use super::proof::Proof;
use super::{Parameters, Statement, Witness};

use crate::error::CryptoError;

#[allow(unused)]

pub struct Prover<'a> {
    parameters: &'a Parameters,
    statement: &'a Statement,
    witness: &'a Witness<'a>,
}

impl<'a> Prover<'a> {
    pub fn new(
        parameters: &'a Parameters,
        statement: &'a Statement,
        witness: &'a Witness<'a>,
    ) -> Self {
        //TODO add dimension assertions
        Self {
            parameters,
            statement,
            witness,
        }
    }

    pub fn prove(&self) -> Result<Proof, CryptoError> {
        let num_of_fixed = self.statement.num_of_fixed;

        let partial_permutation = self.witness.permutation.mapping[num_of_fixed..].to_vec();

        let proof = Proof {
            partial_permutation,
        };

        Ok(proof)
    }
}
