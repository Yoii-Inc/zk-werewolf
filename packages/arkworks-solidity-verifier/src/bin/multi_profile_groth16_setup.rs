use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::encryption::AsymmetricEncryptionScheme;
use ark_groth16::{Groth16, ProvingKey};
use ark_serialize::CanonicalSerialize;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_std::{test_rng, UniformRand};
use mpc_algebra::CommitmentScheme;
use mpc_algebra_wasm::{GroupingParameter, Role as GroupingRole};
use mpc_circuits::{
    AnonymousVotingCircuit, AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
    DivinationCircuit, DivinationPrivateInput, DivinationPublicInput, KeyPublicizeCircuit,
    KeyPublicizePrivateInput, KeyPublicizePublicInput, RoleAssignmentCircuit,
    RoleAssignmentPrivateInput, RoleAssignmentPublicInput, WinningJudgementCircuit,
    WinningJudgementPrivateInput, WinningJudgementPublicInput,
};
use serde::Serialize;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use arkworks_solidity_verifier::SolidityVerifier;

const DIVINATION_PLAYER_COUNTS: [usize; 7] = [3, 4, 5, 6, 7, 8, 9];
const ANONYMOUS_VOTING_PLAYER_COUNTS: [usize; 7] = [3, 4, 5, 6, 7, 8, 9];
const WINNING_JUDGEMENT_PLAYER_COUNTS: [usize; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
const KEY_PUBLICIZE_PLAYER_COUNTS: [usize; 6] = [4, 5, 6, 7, 8, 9];
const ROLE_ASSIGNMENT_PROFILES: [(usize, usize); 7] = [
    (4, 1),
    (5, 1),
    (5, 2),
    (6, 1),
    (6, 2),
    // Temporarily disabled: generated verifier exceeds EVM max contract size (24,576 bytes).
    // (7, 1),
    (7, 2),
    (7, 3),
    // Temporarily disabled: generated verifier exceeds EVM max contract size (24,576 bytes).
    // (8, 1),
    // (8, 2),
    // (8, 3),
    // (9, 1),
    // (9, 2),
    // (9, 3),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupMetadata {
    circuit_id: String,
    max_players: usize,
    public_input_len: usize,
    pk_path: String,
    verifier_path: String,
}

fn main() -> Result<()> {
    for (num_players, werewolf_count) in ROLE_ASSIGNMENT_PROFILES {
        generate_role_assignment_profile(num_players, werewolf_count)?;
    }

    for num_players in DIVINATION_PLAYER_COUNTS {
        generate_divination_profile(num_players)?;
    }
    for num_players in ANONYMOUS_VOTING_PLAYER_COUNTS {
        generate_anonymous_voting_profile(num_players)?;
    }
    for num_players in WINNING_JUDGEMENT_PLAYER_COUNTS {
        generate_winning_judgement_profile(num_players)?;
    }
    for num_players in KEY_PUBLICIZE_PLAYER_COUNTS {
        generate_key_publicize_profile(num_players)?;
    }

    println!("Generated Groth16 setups for all configured profiles.");
    Ok(())
}

fn generate_role_assignment_profile(num_players: usize, werewolf_count: usize) -> Result<()> {
    let circuit_id = format!("role_assignment_n{num_players}_w{werewolf_count}_v1");
    let contract_name = format!("RoleAssignmentN{num_players}W{werewolf_count}Groth16Verifier");
    let mut rng = test_rng();
    let circuit = build_role_assignment_circuit(num_players, werewolf_count, &mut rng)?;
    let public_input_len = role_assignment_public_input_len(num_players, werewolf_count);

    generate_and_write(
        circuit_id,
        contract_name,
        num_players,
        public_input_len,
        circuit,
        &mut rng,
    )
}

fn generate_divination_profile(num_players: usize) -> Result<()> {
    let circuit_id = format!("divination_n{num_players}_v1");
    let contract_name = format!("DivinationN{num_players}Groth16Verifier");
    let mut rng = test_rng();
    let circuit = build_divination_circuit(num_players, &mut rng)?;
    generate_and_write(circuit_id, contract_name, num_players, 8, circuit, &mut rng)
}

fn generate_anonymous_voting_profile(num_players: usize) -> Result<()> {
    let circuit_id = format!("anonymous_voting_n{num_players}_v1");
    let contract_name = format!("AnonymousVotingN{num_players}Groth16Verifier");
    let mut rng = test_rng();
    let circuit = build_anonymous_voting_circuit(num_players, &mut rng)?;
    generate_and_write(circuit_id, contract_name, num_players, 1, circuit, &mut rng)
}

fn generate_winning_judgement_profile(num_players: usize) -> Result<()> {
    let circuit_id = format!("winning_judgement_n{num_players}_v1");
    let contract_name = format!("WinningJudgementN{num_players}Groth16Verifier");
    let mut rng = test_rng();
    let circuit = build_winning_judgement_circuit(num_players, &mut rng)?;
    generate_and_write(circuit_id, contract_name, num_players, 2, circuit, &mut rng)
}

fn generate_key_publicize_profile(num_players: usize) -> Result<()> {
    let circuit_id = format!("key_publicize_n{num_players}_v1");
    let contract_name = format!("KeyPublicizeN{num_players}Groth16Verifier");
    let mut rng = test_rng();
    let circuit = build_key_publicize_circuit(num_players, &mut rng)?;
    generate_and_write(circuit_id, contract_name, num_players, 0, circuit, &mut rng)
}

fn generate_and_write<C>(
    circuit_id: String,
    contract_name: String,
    max_players: usize,
    public_input_len: usize,
    circuit: C,
    rng: &mut (impl ark_std::rand::RngCore + ark_std::rand::CryptoRng),
) -> Result<()>
where
    C: ark_relations::r1cs::ConstraintSynthesizer<Fr>,
{
    let pk_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../zk-mpc-node/data/groth16/{circuit_id}.pk"));
    let verifier_out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../foundry/contracts/verifiers/generated/{contract_name}.sol"
    ));
    let metadata_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../zk-mpc-node/data/groth16/{circuit_id}.json"));

    let (pk, vk) = Groth16::<Bn254>::setup(circuit, rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed for {circuit_id}: {e:?}"))?;

    write_proving_key(&pk_out, &pk)?;

    let contract_src = Groth16::<Bn254>::export(&vk);
    let renamed_contract = rename_generated_contract(&contract_src, &contract_name);
    write_file(&verifier_out, &renamed_contract)?;

    let metadata = SetupMetadata {
        circuit_id: circuit_id.clone(),
        max_players,
        public_input_len,
        pk_path: pk_out.display().to_string(),
        verifier_path: verifier_out.display().to_string(),
    };
    write_file(&metadata_out, &serde_json::to_string_pretty(&metadata)?)?;

    println!("wrote {}", pk_out.display());
    println!("wrote {}", verifier_out.display());
    println!("wrote {}", metadata_out.display());
    Ok(())
}

fn build_role_assignment_circuit(
    num_players: usize,
    werewolf_count: usize,
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<RoleAssignmentCircuit<Fr>> {
    if num_players < 4 || num_players > 9 {
        bail!("RoleAssignment supports 4..9 players only");
    }
    if werewolf_count == 0 || werewolf_count >= num_players {
        bail!("Invalid werewolf count {werewolf_count} for {num_players} players");
    }
    let villager_count = num_players
        .checked_sub(1 + werewolf_count)
        .context("invalid (num_players, werewolf_count) profile")?;

    let mut map = BTreeMap::new();
    map.insert(GroupingRole::FortuneTeller, (1, false));
    map.insert(GroupingRole::Werewolf, (werewolf_count, werewolf_count > 1));
    map.insert(GroupingRole::Villager, (villager_count, false));
    let grouping_parameter = GroupingParameter::new(map);
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

    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let role_commitment = vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];
    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];

    Ok(RoleAssignmentCircuit {
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
    })
}

fn build_divination_circuit(
    num_players: usize,
    rng: &mut (impl ark_std::rand::RngCore + ark_std::rand::CryptoRng),
) -> Result<DivinationCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let elgamal_param =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::setup(rng)
            .map_err(|e| anyhow::anyhow!("elgamal setup failed: {e:?}"))?;
    let (pub_key, _secret_key) =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::keygen(
            &elgamal_param,
            rng,
        )
        .map_err(|e| anyhow::anyhow!("elgamal keygen failed: {e:?}"))?;

    let randomness = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalRandomness::rand(rng);
    let private_input = (0..num_players)
        .map(|id| {
            let mut is_target = vec![Fr::from(0u32); num_players];
            if id == 0 {
                is_target[0] = Fr::from(1u32);
            }

            DivinationPrivateInput::<Fr> {
                id,
                is_werewolf: if id == 0 {
                    Fr::from(1u32)
                } else {
                    Fr::from(0u32)
                },
                is_target,
                randomness: randomness.clone(),
            }
        })
        .collect::<Vec<_>>();

    Ok(DivinationCircuit {
        private_input,
        public_input: DivinationPublicInput::<Fr> {
            pedersen_param,
            elgamal_param,
            pub_key,
            player_num: num_players,
        },
    })
}

fn build_anonymous_voting_circuit(
    num_players: usize,
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<AnonymousVotingCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let private_input = (0..num_players)
        .map(|id| {
            let mut is_target_id = vec![Fr::from(0u32); num_players];
            is_target_id[1] = Fr::from(1u32);

            AnonymousVotingPrivateInput::<Fr> {
                id,
                is_target_id,
                player_randomness: Fr::from((id + 1) as u64),
            }
        })
        .collect::<Vec<_>>();

    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];

    Ok(AnonymousVotingCircuit {
        private_input,
        public_input: AnonymousVotingPublicInput::<Fr> {
            pedersen_param,
            player_commitment,
            player_num: num_players,
        },
    })
}

fn build_winning_judgement_circuit(
    num_players: usize,
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<WinningJudgementCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let private_input = (0..num_players)
        .map(|id| WinningJudgementPrivateInput::<Fr> {
            id,
            am_werewolf: if id == 0 {
                Fr::from(1u32)
            } else {
                Fr::from(0u32)
            },
            player_randomness: Fr::from((id + 7) as u64),
        })
        .collect::<Vec<_>>();

    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players];

    Ok(WinningJudgementCircuit {
        private_input,
        public_input: WinningJudgementPublicInput::<Fr> {
            pedersen_param,
            player_commitment,
        },
    })
}

fn build_key_publicize_circuit(
    num_players: usize,
    rng: &mut (impl ark_std::rand::RngCore + ark_std::rand::CryptoRng),
) -> Result<KeyPublicizeCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let elgamal_param =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::setup(rng)
            .map_err(|e| anyhow::anyhow!("elgamal setup failed: {e:?}"))?;
    let (pub_key, _secret_key) =
        <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme as AsymmetricEncryptionScheme>::keygen(
            &elgamal_param,
            rng,
        )
        .map_err(|e| anyhow::anyhow!("elgamal keygen failed: {e:?}"))?;

    let private_input = (0..num_players)
        .map(|id| KeyPublicizePrivateInput::<Fr> {
            id,
            pub_key_or_dummy_x: if id == 0 { pub_key.x } else { Fr::from(0u32) },
            pub_key_or_dummy_y: if id == 0 { pub_key.y } else { Fr::from(0u32) },
            is_fortune_teller: if id == 0 {
                Fr::from(1u32)
            } else {
                Fr::from(0u32)
            },
        })
        .collect::<Vec<_>>();

    Ok(KeyPublicizeCircuit {
        private_input,
        public_input: KeyPublicizePublicInput::<Fr> { pedersen_param },
    })
}

fn role_assignment_public_input_len(num_players: usize, werewolf_count: usize) -> usize {
    let matrix_size = 2 * num_players - werewolf_count + 1;
    matrix_size * matrix_size
}

fn write_proving_key(path: &PathBuf, pk: &ProvingKey<Bn254>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut bytes = Vec::new();
    pk.serialize_uncompressed(&mut bytes)
        .map_err(|e| anyhow::anyhow!("failed to serialize proving key: {e:?}"))?;
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn write_file(path: &PathBuf, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn rename_generated_contract(contract_src: &str, contract_name: &str) -> String {
    let pairing_name = format!("{contract_name}Pairing");
    let renamed = contract_src.replacen(
        "contract Verifier {",
        &format!("contract {contract_name} {{"),
        1,
    );
    let renamed = renamed.replace("Pairing", &pairing_name);

    if renamed.starts_with("// SPDX-License-Identifier:") {
        renamed
    } else {
        format!("// SPDX-License-Identifier: MIT\n{renamed}")
    }
}
