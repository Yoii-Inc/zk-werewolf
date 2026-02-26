use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, ProvingKey, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_circuits::{
    WinningJudgementCircuit, WinningJudgementPrivateInput, WinningJudgementPublicInput,
};
use zk_mpc::circuits::LocalOrMPC;

use arkworks_solidity_verifier::SolidityVerifier;

const FIXED_PLAYERS: usize = 5;

fn main() -> Result<()> {
    let contract = generate_groth16_verifier_contract()?;
    let contract = rename_contract(&contract);
    let output_path = output_path();

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&output_path, contract)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    println!("wrote {}", output_path.display());
    Ok(())
}

fn generate_groth16_verifier_contract() -> Result<String> {
    if let Some(vk) = load_vk_from_pk()? {
        return Ok(Groth16::<Bn254>::export(&vk));
    }

    let mut rng = test_rng();
    let circuit = build_fixed_winning_judgement_circuit(&mut rng)?;
    let (_, vk) = Groth16::<Bn254>::setup(circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed: {e:?}"))?;

    Ok(Groth16::<Bn254>::export(&vk))
}

fn load_vk_from_pk() -> Result<Option<VerifyingKey<Bn254>>> {
    let path = match std::env::var("WINNING_JUDGEMENT_GROTH16_PK_PATH") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => return Ok(None),
    };

    if !path.exists() {
        anyhow::bail!(
            "WINNING_JUDGEMENT_GROTH16_PK_PATH is set but file does not exist: {}",
            path.display()
        );
    }

    let bytes = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let pk = ProvingKey::<Bn254>::deserialize_uncompressed(bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("failed to deserialize proving key: {e:?}"))?;

    Ok(Some(pk.vk))
}

fn build_fixed_winning_judgement_circuit(
    rng: &mut impl ark_std::rand::RngCore,
) -> Result<WinningJudgementCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let private_input = (0..FIXED_PLAYERS)
        .map(|id| WinningJudgementPrivateInput::<Fr> {
            id,
            am_werewolf: if id == 0 { Fr::from(1u32) } else { Fr::from(0u32) },
            player_randomness: Fr::from((id + 7) as u64),
        })
        .collect::<Vec<_>>();

    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); FIXED_PLAYERS];

    Ok(WinningJudgementCircuit {
        private_input,
        public_input: WinningJudgementPublicInput::<Fr> {
            pedersen_param,
            player_commitment,
        },
    })
}

fn rename_contract(contract_src: &str) -> String {
    let renamed = contract_src.replacen(
        "contract Verifier {",
        "contract WinningJudgementGroth16Verifier {",
        1,
    );
    let renamed = renamed.replace("Pairing", "WinningJudgementPairing");

    if renamed.starts_with("// SPDX-License-Identifier:") {
        renamed
    } else {
        format!("// SPDX-License-Identifier: MIT\n{renamed}")
    }
}

fn output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../foundry/contracts/verifiers/WinningJudgementGroth16Verifier.sol")
}
