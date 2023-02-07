pub mod proof;
pub mod prover;
mod tests;

use crate::homomorphic_encryption::HomomorphicEncryptionScheme;
use crate::vector_commitment::HomomorphicCommitmentScheme;
use crate::zkp::ArgumentOfKnowledge;
use crate::{error::CryptoError, utils::permutation::Permutation};
use ark_ff::Field;
use ark_marlin::rng::FiatShamirRng;
use ark_std::{marker::PhantomData, rand::Rng};
use digest::Digest;

pub struct PartialShuffle<
    'a,
    F: Field,
    Enc: HomomorphicEncryptionScheme<F>,
    Comm: HomomorphicCommitmentScheme<F>,
> {
    _field: PhantomData<&'a F>,
    _encryption_scheme: PhantomData<&'a Enc>,
    _commitment_scheme: PhantomData<&'a Comm>,
}

impl<'a, F, Enc, Comm> ArgumentOfKnowledge for PartialShuffle<'a, F, Enc, Comm>
where
    F: Field,
    Enc: HomomorphicEncryptionScheme<F>,
    Comm: HomomorphicCommitmentScheme<F>,
{
    type CommonReferenceString = Parameters;
    type Statement = Statement;
    type Witness = Witness<'a>;
    type Proof = proof::Proof;

    #[allow(unused_variables)]

    fn prove<R: Rng, D: Digest>(
        rng: &mut R,
        common_reference_string: &Self::CommonReferenceString,
        statement: &Self::Statement,
        witness: &Self::Witness,
        fs_rng: &mut FiatShamirRng<D>,
    ) -> Result<Self::Proof, CryptoError> {
        let prover = prover::Prover::new(&common_reference_string, &statement, &witness);
        let proof = prover.prove()?;

        Ok(proof)
    }

    #[allow(unused_variables)]

    fn verify<D: Digest>(
        common_reference_string: &Self::CommonReferenceString,
        statement: &Self::Statement,
        proof: &Self::Proof,
        fs_rng: &mut FiatShamirRng<D>,
    ) -> Result<(), CryptoError> {
        proof.verify(statement)
    }
}

/// Parameters for the multi-exponentiation argument. Contains the encryption public key, a commitment key
/// and a public group generator which will be used for masking.
pub struct Parameters {}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Witness for the multi-exponentiation argument. Contains a hidden n-by-m matrix A, a vector of randoms r used to commit to
/// the columns of A and an aggregate re-encryption factor rho
pub struct Witness<'a> {
    pub permutation: &'a Permutation,
}

impl<'a> Witness<'a> {
    pub fn new(permutation: &'a Permutation) -> Self {
        Self { permutation }
    }
}

pub struct Statement {
    pub num_of_fixed: usize,
    pub num_of_total: usize,
}

impl Statement {
    pub fn new(num_of_fixed: usize, num_of_total: usize) -> Self {
        Self {
            num_of_fixed,
            num_of_total,
        }
    }
}
