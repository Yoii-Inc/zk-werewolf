use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey};
use ark_serialize::CanonicalSerialize;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_algebra_wasm::GroupingParameter;
use mpc_circuits::{RoleAssignmentCircuit, RoleAssignmentPrivateInput, RoleAssignmentPublicInput};
use serde::Serialize;
use zk_mpc::circuits::LocalOrMPC;

use arkworks_solidity_verifier::SolidityVerifier;

const CIRCUIT_ID: &str = "role_assignment_max5_v1";
const FIXED_PLAYERS: usize = 5;

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
    let pk_out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../zk-mpc-node/data/groth16/{CIRCUIT_ID}.pk"));
    let verifier_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../foundry/contracts/verifiers/RoleAssignmentGroth16Verifier.sol");
    let metadata_out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../zk-mpc-node/data/groth16/{CIRCUIT_ID}.json"));

    let mut rng = test_rng();
    let circuit = build_fixed_role_assignment_circuit(&mut rng)?;
    let (pk, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed: {e:?}"))?;

    write_proving_key(&pk_out, &pk)?;

    let contract = rename_contract(&Groth16::<Bn254>::export(&vk));
    write_file(&verifier_out, &contract)?;

    let grouping = GroupingParameter::from_player_count(FIXED_PLAYERS);
    let matrix_size = grouping.get_num_players() + grouping.get_num_groups();
    let metadata = SetupMetadata {
        circuit_id: CIRCUIT_ID.to_string(),
        max_players: FIXED_PLAYERS,
        public_input_len: matrix_size * matrix_size,
        pk_path: pk_out.display().to_string(),
        verifier_path: verifier_out.display().to_string(),
    };
    write_file(&metadata_out, &serde_json::to_string_pretty(&metadata)?)?;

    println!("wrote {}", pk_out.display());
    println!("wrote {}", verifier_out.display());
    println!("wrote {}", metadata_out.display());

    Ok(())
}

fn build_fixed_role_assignment_circuit(
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<RoleAssignmentCircuit<Fr>> {
    let grouping_parameter = GroupingParameter::from_player_count(FIXED_PLAYERS);
    let tau_matrix = grouping_parameter.generate_tau_matrix::<Fr>();
    let matrix_size = tau_matrix.nrows();
    let identity = nalgebra::DMatrix::<Fr>::identity(matrix_size, matrix_size);

    let private_input = (0..FIXED_PLAYERS)
        .map(|id| RoleAssignmentPrivateInput::<Fr> {
            id,
            shuffle_matrices: identity.clone(),
            randomness: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
            player_randomness: Fr::from((id + 1) as u64),
        })
        .collect::<Vec<_>>();

    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let role_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); FIXED_PLAYERS];
    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); FIXED_PLAYERS];

    Ok(RoleAssignmentCircuit {
        private_input,
        public_input: RoleAssignmentPublicInput::<Fr> {
            num_players: FIXED_PLAYERS,
            max_group_size: grouping_parameter.get_max_group_size(),
            pedersen_param,
            grouping_parameter,
            tau_matrix,
            role_commitment,
            player_commitment,
        },
    })
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

fn rename_contract(contract_src: &str) -> String {
    let renamed = contract_src.replacen(
        "contract Verifier {",
        "contract RoleAssignmentGroth16Verifier {",
        1,
    );

    if renamed.starts_with("// SPDX-License-Identifier:") {
        renamed
    } else {
        format!("// SPDX-License-Identifier: MIT\n{renamed}")
    }
}
