use std::collections::BTreeMap;

use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::CommitmentScheme;
use ark_groth16::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_std::test_rng;
use mpc_algebra_wasm::{calc_shuffle_matrix, GroupingParameter, Role as GroupingRole};
use mpc_circuits::{RoleAssignmentCircuit, RoleAssignmentPrivateInput, RoleAssignmentPublicInput};
use zk_mpc::circuits::LocalOrMPC;

fn build_grouping_parameter(num_players: usize, werewolf_count: usize) -> GroupingParameter {
    assert!(num_players >= 4, "num_players must be >= 4");
    assert!(werewolf_count > 0, "werewolf_count must be > 0");
    assert!(werewolf_count < num_players, "werewolf_count must be < num_players");

    let villager_count = num_players - 1 - werewolf_count;
    assert!(villager_count >= 1, "villager_count must be >= 1");

    let mut map = BTreeMap::new();
    map.insert(GroupingRole::FortuneTeller, (1, false));
    map.insert(GroupingRole::Werewolf, (werewolf_count, werewolf_count > 1));
    map.insert(GroupingRole::Villager, (villager_count, false));
    GroupingParameter::new(map)
}

fn build_role_assignment_circuit(num_players: usize, werewolf_count: usize) -> RoleAssignmentCircuit<Fr> {
    let mut rng = test_rng();
    let grouping_parameter = build_grouping_parameter(num_players, werewolf_count);
    let tau_matrix = grouping_parameter.generate_tau_matrix::<Fr>();
    let matrix_size = tau_matrix.nrows();
    let identity = nalgebra::DMatrix::<Fr>::identity(matrix_size, matrix_size);

    let private_input = (0..num_players)
        .map(|id| RoleAssignmentPrivateInput::<Fr> {
            id,
            shuffle_matrices: identity.clone(),
            randomness: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
            player_randomness: Fr::from((id + 1) as u64),
        })
        .collect::<Vec<_>>();

    let pedersen_param =
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::setup(&mut rng).unwrap();
    let role_commitment = vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];
    let player_commitment = vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];

    RoleAssignmentCircuit::<Fr> {
        private_input,
        public_input: RoleAssignmentPublicInput::<Fr> {
            num_players,
            max_group_size: grouping_parameter.get_max_group_size(),
            pedersen_param,
            grouping_parameter,
            tau_matrix,
            role_commitment,
            player_commitment,
        },
    }
}

fn build_public_inputs(circuit: &RoleAssignmentCircuit<Fr>) -> Vec<Fr> {
    let tau = &circuit.public_input.tau_matrix;
    // IMPORTANT: nalgebra matrix iterator is column-major. This must match
    // FpVar::new_input allocation order in the circuit.
    tau.iter().copied().collect()
}

fn print_role_assignment_outputs(num_players: usize, werewolf_count: usize) {
    let grouping_parameter = build_grouping_parameter(num_players, werewolf_count);
    let matrix_size = grouping_parameter.get_num_players() + grouping_parameter.get_num_groups();
    let identity = nalgebra::DMatrix::<Fr>::identity(matrix_size, matrix_size);
    let shuffle_matrices = (0..num_players).map(|_| identity.clone()).collect::<Vec<_>>();

    println!("=== RoleAssignment local output: n{}-w{} ===", num_players, werewolf_count);
    for id in 0..num_players {
        let (role, role_val, fellow_ids) =
            calc_shuffle_matrix(&grouping_parameter, &shuffle_matrices, id).unwrap();
        println!(
            "player_id={} role={:?} role_val={} fellow_ids={:?}",
            id, role, role_val, fellow_ids
        );
    }
}

fn prove_and_verify_role_assignment_profile(num_players: usize, werewolf_count: usize) -> bool {
    let circuit = build_role_assignment_circuit(num_players, werewolf_count);
    let public_inputs = build_public_inputs(&circuit);

    let mut rng = test_rng();
    let params = generate_random_parameters::<Bn254, _, _>(circuit.clone(), &mut rng).unwrap();
    let proof = create_random_proof(circuit, &params, &mut rng).unwrap();
    let pvk = prepare_verifying_key(&params.vk);

    verify_proof(&pvk, &proof, &public_inputs).unwrap()
}

#[test]
fn role_assignment_groth16_local_profiles_prove_and_verify() {
    let profiles = [(4, 1), (5, 1), (5, 2), (6, 1), (6, 2)];
    let mut failed_profiles = Vec::new();
    for (num_players, werewolf_count) in profiles {
        if !prove_and_verify_role_assignment_profile(num_players, werewolf_count) {
            failed_profiles.push(format!("n{}-w{}", num_players, werewolf_count));
        }
    }
    assert!(
        failed_profiles.is_empty(),
        "verify_proof failed for profiles: {}",
        failed_profiles.join(", ")
    );
}

#[test]
fn role_assignment_local_output_n5w2_n6w2() {
    print_role_assignment_outputs(5, 2);
    print_role_assignment_outputs(6, 2);
}

#[test]
fn role_assignment_local_constraint_satisfaction_diagnostics() {
    let profiles = [(4, 1), (5, 1), (5, 2), (6, 1), (6, 2)];
    for (num_players, werewolf_count) in profiles {
        let circuit = build_role_assignment_circuit(num_players, werewolf_count);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        let is_satisfied = cs.is_satisfied().unwrap();
        println!(
            "constraint_satisfied n{}-w{} = {}, constraints={}",
            num_players,
            werewolf_count,
            is_satisfied,
            cs.num_constraints()
        );
        if !is_satisfied {
            println!(
                "unsatisfied n{}-w{}: {:?}",
                num_players,
                werewolf_count,
                cs.which_is_unsatisfied()
            );
        }
    }
}
