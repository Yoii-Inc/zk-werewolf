use std::{path::PathBuf, process::Command};

use ark_bn254::{Bn254, Fr};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Groth16, ProvingKey};
use ark_serialize::CanonicalDeserialize;
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_std::test_rng;
use mpc_algebra::CommitmentScheme;
use mpc_circuits::{
    AnonymousVotingCircuit, AnonymousVotingPrivateInput, AnonymousVotingPublicInput,
};
use serde::Serialize;
use zk_mpc::circuits::LocalOrMPC;

const FIXED_PLAYERS: usize = 5;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureOutput {
    ax: String,
    ay: String,
    bx: Vec<String>,
    by: Vec<String>,
    cx: String,
    cy: String,
    public_inputs: Vec<String>,
    offchain_verified: bool,
}

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|arg| arg == "--emit-json") {
        let fixture = generate_fixture()?;
        println!("{}", serde_json::to_string(&fixture)?);
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let output = Command::new(exe).arg("--emit-json").output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("fixture generation failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('{') && trimmed.ends_with('}')
        })
        .ok_or_else(|| anyhow::anyhow!("no JSON payload found in fixture output"))?;
    println!("{}", json_line.trim());
    Ok(())
}

fn generate_fixture() -> anyhow::Result<FixtureOutput> {
    let mut rng = test_rng();
    let circuit = build_fixed_anonymous_voting_circuit(&mut rng)?;
    let public_inputs = vec![circuit.calculate_output()];

    let pk = load_or_generate_proving_key(circuit.clone(), &mut rng)?;
    let vk = pk.vk.clone();
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("Groth16 prove failed: {e:?}"))?;
    let ok = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .map_err(|e| anyhow::anyhow!("Groth16 verify failed: {e:?}"))?;

    Ok(FixtureOutput {
        ax: field_to_hex(proof.a.x),
        ay: field_to_hex(proof.a.y),
        bx: vec![field_to_hex(proof.b.x.c0), field_to_hex(proof.b.x.c1)],
        by: vec![field_to_hex(proof.b.y.c0), field_to_hex(proof.b.y.c1)],
        cx: field_to_hex(proof.c.x),
        cy: field_to_hex(proof.c.y),
        public_inputs: public_inputs.iter().copied().map(field_to_hex).collect(),
        offchain_verified: ok,
    })
}

fn load_or_generate_proving_key(
    circuit: AnonymousVotingCircuit<Fr>,
    rng: &mut (impl ark_std::rand::RngCore + ark_std::rand::CryptoRng),
) -> anyhow::Result<ProvingKey<Bn254>> {
    if let Some(path) = resolve_pk_path() {
        let bytes = std::fs::read(&path)
            .map_err(|e| anyhow::anyhow!("failed to read proving key {}: {}", path.display(), e))?;
        let pk = ProvingKey::<Bn254>::deserialize_uncompressed(bytes.as_slice())
            .map_err(|e| anyhow::anyhow!("failed to deserialize proving key: {:?}", e))?;
        return Ok(pk);
    }

    let (pk, _) = Groth16::<Bn254>::setup(circuit, rng)
        .map_err(|e| anyhow::anyhow!("Groth16 setup failed: {e:?}"))?;
    Ok(pk)
}

fn resolve_pk_path() -> Option<PathBuf> {
    let explicit = std::env::var("ANONYMOUS_VOTING_GROTH16_PK_PATH").ok();
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.exists() {
                return Some(path);
            }
        }
    }

    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../zk-mpc-node/data/groth16/anonymous_voting_max5_v1.pk");
    if default_path.exists() {
        return Some(default_path);
    }

    None
}

fn build_fixed_anonymous_voting_circuit(
    rng: &mut impl ark_std::rand::RngCore,
) -> anyhow::Result<AnonymousVotingCircuit<Fr>> {
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {e:?}"))?;

    let private_input = (0..FIXED_PLAYERS)
        .map(|id| {
            let mut is_target_id = vec![Fr::from(0u32); FIXED_PLAYERS];
            is_target_id[1] = Fr::from(1u32);

            AnonymousVotingPrivateInput::<Fr> {
                id,
                is_target_id,
                player_randomness: Fr::from((id + 1) as u64),
            }
        })
        .collect::<Vec<_>>();

    let player_commitment =
        vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); FIXED_PLAYERS];

    Ok(AnonymousVotingCircuit {
        private_input,
        public_input: AnonymousVotingPublicInput::<Fr> {
            pedersen_param,
            player_commitment,
            player_num: FIXED_PLAYERS,
        },
    })
}

fn field_to_hex<F: PrimeField>(value: F) -> String {
    let mut le = value.into_repr().to_bytes_le();
    le.resize(32, 0);

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(66);
    out.push_str("0x");
    for b in le.iter().rev() {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
