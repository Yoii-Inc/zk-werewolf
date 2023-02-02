#[cfg(test)]
#[allow(unused)]
mod test {

    use crate::homomorphic_encryption::el_gamal;
    use crate::utils::permutation::Permutation;
    use crate::vector_commitment::pedersen;
    use crate::zkp::arguments::partial_shuffle;
    use crate::zkp::ArgumentOfKnowledge;

    use ark_marlin::rng::FiatShamirRng;
    use ark_std::rand::thread_rng;
    use blake2::Blake2s;
    use starknet_curve;

    // Choose ellitptic curve setting
    type Curve = starknet_curve::Projective;
    type Scalar = starknet_curve::Fr;

    // Type aliases for concrete instances using the chosen EC.
    type Enc = el_gamal::ElGamal<Curve>;
    type Comm = pedersen::PedersenCommitment<Curve>;
    type Plaintext = el_gamal::Plaintext<Curve>;
    type Generator = el_gamal::Generator<Curve>;
    type Ciphertext = el_gamal::Ciphertext<Curve>;
    type Witness<'a> = partial_shuffle::Witness<'a>;
    type Statement<'a> = partial_shuffle::Statement;
    type PartialShuffleArg<'a> = partial_shuffle::PartialShuffle<'a, Scalar, Enc, Comm>;
    type FS = FiatShamirRng<Blake2s>;

    #[test]
    fn test_partial_shuffle() {
        let rng = &mut thread_rng();
        let m = 1;
        let n = 14;
        let number_of_ciphers = m * n;

        // construct parameters
        let parameters = partial_shuffle::Parameters::new();

        let permutation = Permutation {
            mapping: (0..number_of_ciphers).collect(),
            size: number_of_ciphers,
        };

        let witness = Witness::new(&permutation);

        let statement = Statement::new(3, number_of_ciphers);

        let mut fs_rng = FS::from_seed(b"Initialised with some input");
        let proof =
            PartialShuffleArg::prove(rng, &parameters, &statement, &witness, &mut fs_rng).unwrap();

        assert_eq!((), proof.verify(&statement).unwrap());
    }
}
