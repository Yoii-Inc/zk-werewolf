use ark_bn254::Fr;
use ark_crypto_primitives::CommitmentScheme;
use ark_ff::{BigInteger, One, PrimeField, Zero};
use ark_std::test_rng;
use mpc_circuits::{
    AnonymousVotingCircuit, AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
};
use zk_mpc::circuits::LocalOrMPC;

fn one_hot(index: usize, len: usize) -> Vec<Fr> {
    (0..len)
        .map(|i| if i == index { Fr::one() } else { Fr::zero() })
        .collect()
}

fn one_hot_u8(index: usize, len: usize) -> Vec<u8> {
    (0..len).map(|i| if i == index { 1 } else { 0 }).collect()
}

fn fr_to_usize(value: Fr) -> usize {
    value.into_repr().to_bytes_le()[0] as usize
}

fn expected_voting_winner(targets: &[usize], candidate_count: usize) -> usize {
    let mut votes = vec![0usize; candidate_count];
    for &target in targets {
        votes[target] += 1;
    }

    let mut winner = 0usize;
    let mut max_votes = 0usize;
    for (idx, &count) in votes.iter().enumerate() {
        if count > max_votes {
            winner = idx;
            max_votes = count;
        }
    }
    winner
}

fn build_voting_circuit(targets: &[usize], candidate_count: usize) -> AnonymousVotingCircuit<Fr> {
    let mut rng = test_rng();
    let pedersen_param =
        <<Fr as LocalOrMPC<Fr>>::PedersenComScheme as CommitmentScheme>::setup(&mut rng).unwrap();

    let private_input = targets
        .iter()
        .enumerate()
        .map(|(id, target)| AnonymousVotingPrivateInput {
            id,
            is_target_id: one_hot(*target, candidate_count),
            player_randomness: Fr::from(id as u32 + 1),
        })
        .collect::<Vec<_>>();

    AnonymousVotingCircuit {
        private_input,
        public_input: AnonymousVotingPublicInput {
            pedersen_param,
            player_commitment: vec![
                <Fr as LocalOrMPC<Fr>>::PedersenCommitment::default();
                candidate_count
            ],
            player_num: candidate_count,
        },
    }
}

fn circuit_like_divination_decision(
    alive_player_ids: &[usize],
    werewolf_flags_by_alive_order: &[u8],
    is_target_matrix: &[Vec<u8>],
) -> bool {
    let player_num = is_target_matrix.first().map(|row| row.len()).unwrap_or(0);
    let mut sum_target = vec![0u32; player_num];
    for row in is_target_matrix
        .iter()
        .take(werewolf_flags_by_alive_order.len())
    {
        for (player_id, target_flag) in row.iter().enumerate().take(player_num) {
            sum_target[player_id] += *target_flag as u32;
        }
    }

    let mut sum = 0u32;
    for (alive_idx, player_id) in alive_player_ids.iter().enumerate() {
        sum += sum_target[*player_id] * werewolf_flags_by_alive_order[alive_idx] as u32;
    }
    sum == 1
}

fn expected_divination_decision(
    werewolf_flags_by_player_id: &[u8],
    is_target_matrix: &[Vec<u8>],
) -> bool {
    (0..werewolf_flags_by_player_id.len()).any(|player_id| {
        let targeted = is_target_matrix
            .iter()
            .any(|row| row.get(player_id).copied().unwrap_or_default() == 1);
        targeted && werewolf_flags_by_player_id[player_id] == 1
    })
}

#[test]
fn anonymous_voting_local_winner_with_scattered_votes() {
    // 5人全員が投票し、投票先が散らばるケース
    // votes: [2, 1, 2, 4, 2] => winner = 2
    let circuit = build_voting_circuit(&[2, 1, 2, 4, 2], 5);
    let winner = fr_to_usize(circuit.calculate_output());
    assert_eq!(winner, 2);
}

#[test]
fn anonymous_voting_local_winner_specific_pattern_30123() {
    // 5人: votes = [3, 0, 1, 2, 3] => winner = 3
    let circuit = build_voting_circuit(&[3, 0, 1, 2, 3], 5);
    let winner = fr_to_usize(circuit.calculate_output());
    assert_eq!(winner, 3);
}

#[test]
fn anonymous_voting_multiple_player_counts_and_patterns() {
    for player_count in 4usize..=9usize {
        let patterns: Vec<Vec<usize>> = vec![
            vec![0; player_count],
            (0..player_count).map(|i| (i + 1) % player_count).collect(),
            (0..player_count).map(|i| (i * 2 + 1) % player_count).collect(),
            (0..player_count)
                .map(|i| if i % 3 == 0 { player_count - 1 } else { i % player_count })
                .collect(),
        ];

        for targets in patterns {
            let circuit = build_voting_circuit(&targets, player_count);
            let winner = fr_to_usize(circuit.calculate_output());
            let expected = expected_voting_winner(&targets, player_count);
            assert_eq!(
                winner, expected,
                "Voting mismatch for player_count={}, targets={:?}",
                player_count, targets
            );
        }
    }
}

#[test]
fn anonymous_voting_exhaustive_small_player_count() {
    // 3人は 3^3=27 通りを総当たり
    let n = 3usize;
    for a in 0..n {
        for b in 0..n {
            for c in 0..n {
                let targets = vec![a, b, c];
                let circuit = build_voting_circuit(&targets, n);
                let winner = fr_to_usize(circuit.calculate_output());
                let expected = expected_voting_winner(&targets, n);
                assert_eq!(winner, expected, "targets={:?}", targets);
            }
        }
    }
}

#[test]
fn anonymous_voting_handles_candidate_ids_beyond_alive_count() {
    // alive=3 だが候補者IDは 0..4 を想定（5人ゲーム）。
    // 修正後ロジックでは候補者列長(player_num)で集計するため、id=4 が正しく勝つ。
    let circuit = build_voting_circuit(&[4, 4, 1], 5);
    let winner = fr_to_usize(circuit.calculate_output());
    assert_eq!(winner, 4);
}

#[test]
fn divination_local_expected_and_current_match_when_indices_are_contiguous() {
    // alive player ids == [0,1,2] を想定したケース
    // werewolf = id 2, target = id 2
    let werewolf_by_alive = vec![0, 0, 1];
    let werewolf_by_player = vec![0, 0, 1];
    let targets = vec![one_hot_u8(2, 3), vec![0u8, 0, 0], vec![0u8, 0, 0]];

    let alive_player_ids = vec![0usize, 1, 2];
    let current = circuit_like_divination_decision(&alive_player_ids, &werewolf_by_alive, &targets);
    let expected = expected_divination_decision(&werewolf_by_player, &targets);

    assert_eq!(current, expected);
    assert!(expected);
}

#[test]
fn divination_contiguous_ids_multiple_sizes_and_targets() {
    // alive ids が連番のときは、現行ロジックと期待ロジックが一致することを確認
    for alive_count in 3usize..=8usize {
        for werewolf_idx in 0..alive_count {
            for target_idx in 0..alive_count {
                let mut werewolf_by_alive = vec![0u8; alive_count];
                werewolf_by_alive[werewolf_idx] = 1;
                let werewolf_by_player = werewolf_by_alive.clone();

                let mut targets = vec![vec![0u8; alive_count]; alive_count];
                // 先頭プレイヤーが target_idx を占うパターンに固定
                targets[0] = one_hot_u8(target_idx, alive_count);

                let alive_player_ids = (0..alive_count).collect::<Vec<_>>();
                let current = circuit_like_divination_decision(&alive_player_ids, &werewolf_by_alive, &targets);
                let expected = expected_divination_decision(&werewolf_by_player, &targets);

                assert_eq!(
                    current, expected,
                    "alive_count={}, werewolf_idx={}, target_idx={}",
                    alive_count, werewolf_idx, target_idx
                );
            }
        }
    }
}

#[test]
fn divination_sparse_ids_match_after_index_fix() {
    // 5人ゲームで alive ids == [0,2,4] の想定。
    // werewolf は player_id=4。target も player_id=4。
    // 期待結果は true だが、現行の「alive_count 範囲のみ参照」ロジックでは false になり得る。
    let werewolf_by_alive_order = vec![0, 0, 1]; // alive ids [0,2,4] の順
    let werewolf_by_player_id = vec![0, 0, 0, 0, 1];
    let targets = vec![
        one_hot_u8(4, 5),
        vec![0u8, 0, 0, 0, 0],
        vec![0u8, 0, 0, 0, 0],
    ];

    let alive_player_ids = vec![0usize, 2, 4];
    let current = circuit_like_divination_decision(&alive_player_ids, &werewolf_by_alive_order, &targets);
    let expected = expected_divination_decision(&werewolf_by_player_id, &targets);

    assert!(expected);
    assert!(current);
    assert_eq!(current, expected);
}

#[test]
fn divination_target_jump_over_dead_players_matches_after_fix() {
    // 5人ゲームで alive ids == [0,3,4] の想定。
    // player_id 1,2 は死亡。生存者0が player_id=3 を占う。
    // player_id=3 が人狼なら期待値は true。
    // ただし現行ロジックは alive_count 範囲(0..2列)しか見ないため false になり得る。
    let werewolf_by_alive_order = vec![0, 1, 0]; // alive ids [0,3,4] の順
    let werewolf_by_player_id = vec![0, 0, 0, 1, 0];
    let targets = vec![
        one_hot_u8(3, 5),
        vec![0u8, 0, 0, 0, 0],
        vec![0u8, 0, 0, 0, 0],
    ];

    let alive_player_ids = vec![0usize, 3, 4];
    let current = circuit_like_divination_decision(&alive_player_ids, &werewolf_by_alive_order, &targets);
    let expected = expected_divination_decision(&werewolf_by_player_id, &targets);

    assert!(expected);
    assert!(current);
    assert_eq!(current, expected);
}

#[test]
fn divination_sparse_ids_multiple_patterns_match_after_fix() {
    // 6人ゲームで alive ids が疎になる複数パターンでも一致することを確認
    let scenarios = vec![
        // alive ids, werewolf player id, target player id
        (vec![0usize, 2, 5], 5usize, 5usize),
        (vec![1usize, 3, 4], 3usize, 3usize),
        (vec![0usize, 4, 5], 4usize, 4usize),
    ];

    for (alive_ids, werewolf_player_id, target_player_id) in scenarios {
        let total_players = 6usize;
        let mut werewolf_by_alive_order = vec![0u8; alive_ids.len()];
        let mut werewolf_by_player_id = vec![0u8; total_players];
        werewolf_by_player_id[werewolf_player_id] = 1;

        for (alive_idx, player_id) in alive_ids.iter().enumerate() {
            if *player_id == werewolf_player_id {
                werewolf_by_alive_order[alive_idx] = 1;
            }
        }

        let mut targets = vec![vec![0u8; total_players]; alive_ids.len()];
        targets[0] = one_hot_u8(target_player_id, total_players);

        let current = circuit_like_divination_decision(&alive_ids, &werewolf_by_alive_order, &targets);
        let expected = expected_divination_decision(&werewolf_by_player_id, &targets);

        assert!(
            expected,
            "Scenario assumption invalid: alive_ids={:?}, werewolf={}, target={}",
            alive_ids, werewolf_player_id, target_player_id
        );
        assert_eq!(
            current, expected,
            "alive_ids={:?}, werewolf={}, target={}",
            alive_ids, werewolf_player_id, target_player_id
        );
    }
}
