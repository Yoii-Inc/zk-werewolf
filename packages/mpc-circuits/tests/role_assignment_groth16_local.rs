use std::collections::BTreeMap;

use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::CommitmentScheme;
use ark_ff::PrimeField;
use ark_groth16::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_std::test_rng;
use mpc_algebra_wasm::{
    calc_shuffle_matrix, generate_individual_shuffle_matrix, GroupingParameter, Role as GroupingRole,
};
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
    let num_groups = grouping_parameter.get_num_groups();

    let private_input = (0..num_players)
        .map(|id| RoleAssignmentPrivateInput::<Fr> {
            id,
            shuffle_matrices: generate_individual_shuffle_matrix::<Fr, _>(num_players, num_groups, &mut rng),
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

fn fr_to_u32(value: Fr) -> u32 {
    let repr = value.into_repr();
    let limbs = repr.as_ref();
    assert!(
        limbs.iter().skip(1).all(|&limb| limb == 0),
        "Fr value is too large for u32 conversion: {:?}",
        limbs
    );
    u32::try_from(limbs[0]).expect("Fr value does not fit into u32")
}

fn role_to_role_id(role: GroupingRole) -> u32 {
    match role {
        GroupingRole::Villager => 0,
        GroupingRole::FortuneTeller => 1,
        GroupingRole::Werewolf => 2,
    }
}

fn decode_player_mask(mask: u32, num_players: usize) -> Vec<usize> {
    (0..num_players)
        .filter(|&idx| (mask & (1u32 << idx)) != 0)
        .collect::<Vec<_>>()
}

fn print_role_assignment_outputs(num_players: usize, werewolf_count: usize) {
    let mut rng = test_rng();
    let grouping_parameter = build_grouping_parameter(num_players, werewolf_count);
    let num_groups = grouping_parameter.get_num_groups();
    let shuffle_matrices = (0..num_players)
        .map(|_| generate_individual_shuffle_matrix::<Fr, _>(num_players, num_groups, &mut rng))
        .collect::<Vec<_>>();

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

#[test]
fn role_assignment_local_calculate_output_matches_calc_shuffle_matrix_profiles() {
    let profiles = [(5, 2), (6, 2), (7, 2), (7, 3), (8, 3)];

    for (num_players, werewolf_count) in profiles {
        let circuit = build_role_assignment_circuit(num_players, werewolf_count);

        let role_outputs = circuit.calculate_output();
        assert_eq!(role_outputs.len(), num_players);

        let grouping_parameter = build_grouping_parameter(num_players, werewolf_count);
        let shuffle_matrices = circuit
            .private_input
            .iter()
            .map(|input| input.shuffle_matrices.clone())
            .collect::<Vec<_>>();

        for player_id in 0..num_players {
            let (expected_role, _expected_role_val, _fellow_ids) =
                calc_shuffle_matrix(&grouping_parameter, &shuffle_matrices, player_id).unwrap();
            assert_eq!(
                fr_to_u32(role_outputs[player_id]),
                role_to_role_id(expected_role),
                "unexpected role id for player {} in n{}-w{}",
                player_id,
                num_players,
                werewolf_count
            );
        }
    }
}

#[test]
fn role_assignment_local_werewolf_mates_mask_matches_calc_shuffle_matrix_profiles() {
    let profiles = [(5, 2), (6, 2), (7, 2), (7, 3), (8, 3)];

    for (num_players, werewolf_count) in profiles {
        let circuit = build_role_assignment_circuit(num_players, werewolf_count);
        let outputs = circuit.calculate_output_with_werewolf_mates_mask();
        assert_eq!(outputs.len(), num_players);

        let grouping_parameter = build_grouping_parameter(num_players, werewolf_count);
        let shuffle_matrices = circuit
            .private_input
            .iter()
            .map(|input| input.shuffle_matrices.clone())
            .collect::<Vec<_>>();

        for player_id in 0..num_players {
            let (role, _role_val, fellow_ids) =
                calc_shuffle_matrix(&grouping_parameter, &shuffle_matrices, player_id).unwrap();

            let expected_role_id = role_to_role_id(role);
            assert_eq!(
                fr_to_u32(outputs[player_id].role_share),
                expected_role_id,
                "unexpected role id for player {} in n{}-w{}",
                player_id,
                num_players,
                werewolf_count
            );

            let expected_mask = if role == GroupingRole::Werewolf {
                fellow_ids
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|&id| id != player_id)
                    .fold(0u32, |acc, id| acc | (1u32 << id))
            } else {
                0u32
            };

            assert_eq!(
                fr_to_u32(outputs[player_id].werewolf_mates_mask_share),
                expected_mask,
                "unexpected teammate mask for player {} in n{}-w{}",
                player_id,
                num_players,
                werewolf_count
            );
        }
    }
}

#[test]
fn role_assignment_local_werewolf_players_see_other_werewolves_profiles() {
    let profiles = [(5, 2), (6, 2), (7, 3), (8, 3)];

    for (num_players, werewolf_count) in profiles {
        let circuit = build_role_assignment_circuit(num_players, werewolf_count);
        let outputs = circuit.calculate_output_with_werewolf_mates_mask();

        let werewolf_ids = outputs
            .iter()
            .enumerate()
            .filter_map(|(id, output)| {
                if fr_to_u32(output.role_share) == role_to_role_id(GroupingRole::Werewolf) {
                    Some(id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        assert_eq!(
            werewolf_ids.len(),
            werewolf_count,
            "unexpected werewolf count in n{}-w{}",
            num_players,
            werewolf_count
        );

        for (player_id, output) in outputs.iter().enumerate() {
            let mask = fr_to_u32(output.werewolf_mates_mask_share);
            let mut actual_mates = decode_player_mask(mask, num_players);
            actual_mates.sort_unstable();

            if werewolf_ids.contains(&player_id) {
                let mut expected_mates = werewolf_ids
                    .iter()
                    .copied()
                    .filter(|&id| id != player_id)
                    .collect::<Vec<_>>();
                expected_mates.sort_unstable();

                assert_eq!(
                    actual_mates,
                    expected_mates,
                    "werewolf player {} has wrong mates in n{}-w{}",
                    player_id,
                    num_players,
                    werewolf_count
                );
            } else {
                assert!(
                    actual_mates.is_empty(),
                    "non-werewolf player {} should not receive werewolf mates in n{}-w{}",
                    player_id,
                    num_players,
                    werewolf_count
                );
            }
        }
    }
}
