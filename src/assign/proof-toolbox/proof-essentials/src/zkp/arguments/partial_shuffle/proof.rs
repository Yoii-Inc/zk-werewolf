use super::Statement;

use crate::error::CryptoError;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
use ark_std::io::{Read, Write};

#[derive(CanonicalDeserialize, CanonicalSerialize)]
pub struct Proof {
    pub(crate) partial_permutation: Vec<usize>,
}

impl Proof {
    pub fn verify(&self, statement: &Statement) -> Result<(), CryptoError> {
        let left = &self.partial_permutation;
        let right: &Vec<usize> =
            &(statement.num_of_total - statement.num_of_fixed..statement.num_of_total).collect();
        if left != right {
            return Err(CryptoError::ProofVerificationError(String::from(
                "Partial Shuffle",
            )));
        }

        Ok(())
    }
}
