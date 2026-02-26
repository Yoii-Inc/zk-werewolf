use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::encryption::AsymmetricEncryptionScheme;
use ark_groth16::{Groth16, ProvingKey};
use ark_serialize::CanonicalSerialize;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_circuits::{KeyPublicizeCircuit, KeyPublicizePrivateInput, KeyPublicizePublicInput};
use serde::Serialize;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

use arkworks_solidity_verifier::SolidityVerifier;

const CIRCUIT_ID: &str = "key_publicize_max5_v1";
const FIXED_PLAYERS: usize = 5;
const PUBLIC_INPUT_LEN: usize = 0;

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
    let pk_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../zk-mpc-node/data/groth16/{CIRCUIT_ID}.pk"));
    let verifier_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../foundry/contracts/verifiers/KeyPublicizeGroth16Verifier.sol");
    let metadata_out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../zk-mpc-node/data/groth16/{CIRCUIT_ID}.json"));

    let mut rng = test_rng();
    let circuit = build_fixed_key_publicize_circuit(&mut rng)?;
    let (pk, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed: {e:?}"))?;

    write_proving_key(&pk_out, &pk)?;

    let contract = rename_contract(&Groth16::<Bn254>::export(&vk));
    write_file(&verifier_out, &contract)?;

    let metadata = SetupMetadata {
        circuit_id: CIRCUIT_ID.to_string(),
        max_players: FIXED_PLAYERS,
        public_input_len: PUBLIC_INPUT_LEN,
        pk_path: pk_out.display().to_string(),
        verifier_path: verifier_out.display().to_string(),
    };
    write_file(&metadata_out, &serde_json::to_string_pretty(&metadata)?)?;

    println!("wrote {}", pk_out.display());
    println!("wrote {}", verifier_out.display());
    println!("wrote {}", metadata_out.display());
    Ok(())
}

fn build_fixed_key_publicize_circuit(
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

    let private_input = (0..FIXED_PLAYERS)
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
        "contract KeyPublicizeGroth16Verifier {",
        1,
    );
    let renamed = renamed.replace("Pairing", "KeyPublicizePairing");

    if renamed.starts_with("// SPDX-License-Identifier:") {
        renamed
    } else {
        format!("// SPDX-License-Identifier: MIT\n{renamed}")
    }
}
