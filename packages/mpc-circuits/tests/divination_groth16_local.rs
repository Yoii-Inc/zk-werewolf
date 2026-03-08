use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::encryption::AsymmetricEncryptionScheme;
use ark_crypto_primitives::CommitmentScheme;
use ark_ec::AffineCurve;
use ark_ff::{One, UniformRand, Zero};
use ark_groth16::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
use ark_std::test_rng;
use mpc_circuits::{DivinationCircuit, DivinationPrivateInput, DivinationPublicInput};
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

fn one_hot(index: usize, len: usize) -> Vec<Fr> {
    (0..len)
        .map(|i| if i == index { Fr::one() } else { Fr::zero() })
        .collect()
}

fn build_divination_circuit(
    player_num: usize,
    seer_id: usize,
    werewolf_id: usize,
    target_id: usize,
) -> DivinationCircuit<Fr> {
    let mut rng = test_rng();

    let pedersen_param =
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::setup(&mut rng).unwrap();
    let elgamal_param =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::setup(
            &mut rng,
        )
        .unwrap();
    let (pub_key, _secret_key) =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::keygen(
            &elgamal_param,
            &mut rng,
        )
        .unwrap();

    let shared_randomness = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalRandomness::rand(&mut rng);

    let private_input = (0..player_num)
        .map(|id| DivinationPrivateInput::<Fr> {
            id,
            is_werewolf: if id == werewolf_id {
                Fr::one()
            } else {
                Fr::zero()
            },
            is_target: if id == seer_id {
                one_hot(target_id, player_num)
            } else {
                vec![Fr::zero(); player_num]
            },
            randomness: shared_randomness.clone(),
        })
        .collect::<Vec<_>>();

    DivinationCircuit::<Fr> {
        private_input,
        public_input: DivinationPublicInput::<Fr> {
            pedersen_param,
            elgamal_param,
            pub_key,
            player_num,
        },
    }
}

fn build_public_inputs(circuit: &DivinationCircuit<Fr>) -> Vec<Fr> {
    let player_num = circuit.private_input[0].is_target.len();
    let mut sum_target = vec![Fr::zero(); player_num];
    for i in 0..player_num {
        for input in &circuit.private_input {
            sum_target[i] += input.is_target[i];
        }
    }

    let mut targeted_werewolf_sum = Fr::zero();
    for input in &circuit.private_input {
        targeted_werewolf_sum += input.is_werewolf * sum_target[input.id];
    }

    let message = if targeted_werewolf_sum.is_one() {
        <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::prime_subgroup_generator()
    } else {
        <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::default()
    };

    let ciphertext =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::encrypt(
            &circuit.public_input.elgamal_param,
            &circuit.public_input.pub_key,
            &message,
            &circuit.private_input[0].randomness,
        )
        .unwrap();

    vec![
        circuit.public_input.elgamal_param.generator.x,
        circuit.public_input.elgamal_param.generator.y,
        circuit.public_input.pub_key.x,
        circuit.public_input.pub_key.y,
        ciphertext.0.x,
        ciphertext.0.y,
        ciphertext.1.x,
        ciphertext.1.y,
    ]
}

#[test]
fn divination_groth16_local_prove_and_verify() {
    // 5人: 占い師3がプレイヤー1を占う。プレイヤー1は人狼。
    let circuit = build_divination_circuit(5, 3, 1, 1);
    let public_inputs = build_public_inputs(&circuit);

    let mut rng = test_rng();
    let params = generate_random_parameters::<Bn254, _, _>(circuit.clone(), &mut rng).unwrap();
    let proof = create_random_proof(circuit, &params, &mut rng).unwrap();
    let pvk = prepare_verifying_key(&params.vk);

    let ok = verify_proof(&pvk, &proof, &public_inputs).unwrap();
    assert!(ok);
}
