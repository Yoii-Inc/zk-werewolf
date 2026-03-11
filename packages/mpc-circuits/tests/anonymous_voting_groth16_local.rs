use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::CommitmentScheme;
use ark_groth16::{
    create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof,
};
use ark_std::test_rng;
use mpc_circuits::{
    AnonymousVotingCircuit, AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
};
use zk_mpc::circuits::LocalOrMPC;

fn one_hot(index: usize, len: usize) -> Vec<Fr> {
    (0..len)
        .map(|i| if i == index { Fr::from(1u64) } else { Fr::from(0u64) })
        .collect()
}

fn build_anonymous_voting_circuit(targets: &[usize], player_num: usize) -> AnonymousVotingCircuit<Fr> {
    let mut rng = test_rng();
    let pedersen_param =
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::setup(&mut rng).unwrap();

    let private_input = targets
        .iter()
        .enumerate()
        .map(|(id, target)| AnonymousVotingPrivateInput::<Fr> {
            id,
            is_target_id: one_hot(*target, player_num),
            player_randomness: Fr::from((id + 1) as u64),
        })
        .collect::<Vec<_>>();

    AnonymousVotingCircuit::<Fr> {
        private_input,
        public_input: AnonymousVotingPublicInput::<Fr> {
            pedersen_param,
            player_commitment: vec![
                <Fr as LocalOrMPC<Fr>>::PedersenCommitment::default();
                player_num
            ],
            player_num,
        },
    }
}

fn build_public_inputs(circuit: &AnonymousVotingCircuit<Fr>) -> Vec<Fr> {
    vec![circuit.calculate_output()]
}

fn prove_and_verify_anonymous_voting(targets: &[usize], player_num: usize) -> bool {
    let circuit = build_anonymous_voting_circuit(targets, player_num);
    let public_inputs = build_public_inputs(&circuit);

    let mut rng = test_rng();
    let params = generate_random_parameters::<Bn254, _, _>(circuit.clone(), &mut rng).unwrap();
    let proof = create_random_proof(circuit, &params, &mut rng).unwrap();
    let pvk = prepare_verifying_key(&params.vk);

    verify_proof(&pvk, &proof, &public_inputs).unwrap()
}

#[test]
fn anonymous_voting_groth16_local_n4_ring_prove_and_verify() {
    // n=4 ring: [1,2,3,0]
    let ok = prove_and_verify_anonymous_voting(&[1, 2, 3, 0], 4);
    assert!(ok);
}

#[test]
fn anonymous_voting_groth16_local_n5_split_vote_prove_and_verify() {
    // n=5 split-vote: [1,1,2,2,3]
    let ok = prove_and_verify_anonymous_voting(&[1, 1, 2, 2, 3], 5);
    assert!(ok);
}
