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

const DEFAULT_CIRCUIT_ID: &str = "role_assignment_max5_v1";
const DEFAULT_MAX_PLAYERS: usize = 5;

#[derive(Debug)]
struct Args {
    circuit_id: String,
    max_players: usize,
    pk_out: PathBuf,
    verifier_out: PathBuf,
    metadata_out: PathBuf,
}

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
    let args = parse_args()?;

    let mut rng = test_rng();
    let circuit = build_role_assignment_circuit(args.max_players, &mut rng)?;
    let (pk, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed: {e:?}"))?;

    write_proving_key(&args.pk_out, &pk)?;

    let contract = rename_contract(&Groth16::<Bn254>::export(&vk));
    write_file(&args.verifier_out, &contract)?;

    let grouping = GroupingParameter::from_player_count(args.max_players);
    let matrix_size = grouping.get_num_players() + grouping.get_num_groups();
    let metadata = SetupMetadata {
        circuit_id: args.circuit_id,
        max_players: args.max_players,
        public_input_len: matrix_size * matrix_size,
        pk_path: args.pk_out.display().to_string(),
        verifier_path: args.verifier_out.display().to_string(),
    };
    write_file(
        &args.metadata_out,
        &serde_json::to_string_pretty(&metadata)?,
    )?;

    println!("wrote {}", args.pk_out.display());
    println!("wrote {}", args.verifier_out.display());
    println!("wrote {}", args.metadata_out.display());

    Ok(())
}

fn parse_args() -> Result<Args> {
    let mut circuit_id = DEFAULT_CIRCUIT_ID.to_string();
    let mut max_players = DEFAULT_MAX_PLAYERS;

    let mut pk_out: Option<PathBuf> = None;
    let mut verifier_out: Option<PathBuf> = None;
    let mut metadata_out: Option<PathBuf> = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--circuit-id" => {
                circuit_id = it.next().context("--circuit-id requires a value")?;
            }
            "--max-players" => {
                let v = it.next().context("--max-players requires a value")?;
                max_players = v
                    .parse::<usize>()
                    .with_context(|| format!("invalid --max-players value: {v}"))?;
            }
            "--pk-out" => {
                pk_out = Some(PathBuf::from(
                    it.next().context("--pk-out requires a value")?,
                ));
            }
            "--verifier-out" => {
                verifier_out = Some(PathBuf::from(
                    it.next().context("--verifier-out requires a value")?,
                ));
            }
            "--metadata-out" => {
                metadata_out = Some(PathBuf::from(
                    it.next().context("--metadata-out requires a value")?,
                ));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_pk = root
        .join("../zk-mpc-node/data/groth16")
        .join(format!("{circuit_id}.pk"));
    let default_metadata = root
        .join("../zk-mpc-node/data/groth16")
        .join(format!("{circuit_id}.json"));
    let default_verifier =
        root.join("../foundry/contracts/verifiers/RoleAssignmentGroth16Verifier.sol");

    Ok(Args {
        circuit_id,
        max_players,
        pk_out: pk_out.unwrap_or(default_pk),
        verifier_out: verifier_out.unwrap_or(default_verifier),
        metadata_out: metadata_out.unwrap_or(default_metadata),
    })
}

fn print_help() {
    println!("role_assignment_groth16_setup");
    println!("  --circuit-id <id>        (default: {DEFAULT_CIRCUIT_ID})");
    println!("  --max-players <n>        (default: {DEFAULT_MAX_PLAYERS})");
    println!("  --pk-out <path>          (default: ../zk-mpc-node/data/groth16/<circuit-id>.pk)");
    println!(
        "  --verifier-out <path>    (default: ../foundry/contracts/verifiers/RoleAssignmentGroth16Verifier.sol)"
    );
    println!("  --metadata-out <path>    (default: ../zk-mpc-node/data/groth16/<circuit-id>.json)");
}

fn build_role_assignment_circuit(
    max_players: usize,
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<RoleAssignmentCircuit<Fr>> {
    let grouping_parameter = GroupingParameter::from_player_count(max_players);
    let tau_matrix = grouping_parameter.generate_tau_matrix::<Fr>();
    let matrix_size = tau_matrix.nrows();
    let identity = nalgebra::DMatrix::<Fr>::identity(matrix_size, matrix_size);

    let private_input = (0..max_players)
        .map(|id| RoleAssignmentPrivateInput::<Fr> {
            id,
            shuffle_matrices: identity.clone(),
            randomness: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
            player_randomness: Fr::from((id + 1) as u64),
        })
        .collect::<Vec<_>>();

    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let role_commitment = vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); max_players];
    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); max_players];

    Ok(RoleAssignmentCircuit {
        private_input,
        public_input: RoleAssignmentPublicInput::<Fr> {
            num_players: max_players,
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
