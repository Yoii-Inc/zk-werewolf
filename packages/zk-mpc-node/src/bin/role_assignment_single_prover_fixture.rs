use ark_bls12_377::Fr;
use ark_serialize::CanonicalSerialize;
use ark_std::{One, UniformRand, Zero};
use libc::{close, dup, dup2};
use mpc_algebra::CommitmentScheme;
use mpc_algebra_wasm::GroupingParameter;
use mpc_circuits::{
    BuiltinCircuit, RoleAssignmentCircuit, RoleAssignmentPrivateInput, RoleAssignmentPublicInput,
};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::fd::AsRawFd;
use zk_mpc::{
    circuits::LocalOrMPC,
    marlin::LocalMarlin,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureOutput {
    proof: String,
    public_inputs: String,
}

fn main() -> anyhow::Result<()> {
    let (proof_bytes, public_input_bytes) = with_stdout_silenced(generate_fixture)?;

    let fixture = FixtureOutput {
        proof: to_hex(&proof_bytes),
        public_inputs: to_hex(&public_input_bytes),
    };
    println!("{}", serde_json::to_string(&fixture)?);

    Ok(())
}

fn generate_fixture() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let mut rng = ark_std::test_rng();

    let grouping_parameter = GroupingParameter::from_player_count(4);
    let num_players = grouping_parameter.get_num_players();
    let num_groups = grouping_parameter.get_num_groups();
    let matrix_size = num_players + num_groups;

    let tau_matrix = grouping_parameter.generate_tau_matrix::<Fr>();
    let pedersen_param = <Fr as LocalOrMPC<Fr>>::PedersenComScheme::setup(&mut rng)
        .map_err(|e| anyhow::anyhow!("pedersen setup failed: {}", e))?;

    let private_input = (0..num_players)
        .map(|i| {
            let mut shuffle_matrices = tau_matrix.clone();
            shuffle_matrices.fill(Fr::zero());
            for d in 0..matrix_size {
                shuffle_matrices[(d, d)] = Fr::one();
            }

            RoleAssignmentPrivateInput::<Fr> {
                id: i,
                shuffle_matrices,
                randomness: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::rand(&mut rng),
                player_randomness: Fr::rand(&mut rng),
            }
        })
        .collect::<Vec<_>>();

    let public_input = RoleAssignmentPublicInput::<Fr> {
        num_players,
        max_group_size: grouping_parameter.get_max_group_size(),
        pedersen_param,
        grouping_parameter,
        tau_matrix: tau_matrix.clone(),
        role_commitment: vec![<Fr as LocalOrMPC<Fr>>::PedersenCommitment::default(); num_players],
        player_commitment: vec![
            <Fr as LocalOrMPC<Fr>>::PedersenCommitment::default();
            num_players
        ],
    };

    let circuit = BuiltinCircuit::RoleAssignment(RoleAssignmentCircuit {
        private_input,
        public_input,
    });

    let mut public_inputs = Vec::with_capacity(tau_matrix.nrows() * tau_matrix.ncols());
    for i in 0..tau_matrix.nrows() {
        for j in 0..tau_matrix.ncols() {
            public_inputs.push(tau_matrix[(i, j)]);
        }
    }

    let srs = LocalMarlin::universal_setup(7000, 7000, 20000, &mut rng)
        .map_err(|e| anyhow::anyhow!("universal_setup failed: {:?}", e))?;
    let (index_pk, index_vk) = LocalMarlin::index(&srs, circuit.clone())
        .map_err(|e| anyhow::anyhow!("index failed: {:?}", e))?;
    let proof = LocalMarlin::prove(&index_pk, circuit, &mut rng)
        .map_err(|e| anyhow::anyhow!("prove failed: {:?}", e))?;
    let is_valid = LocalMarlin::verify(&index_vk, &public_inputs, &proof, &mut rng)
        .map_err(|e| anyhow::anyhow!("verify failed: {:?}", e))?;
    if !is_valid {
        anyhow::bail!("local marlin verification failed");
    }

    let mut proof_bytes = Vec::new();
    proof.serialize_uncompressed(&mut proof_bytes)?;

    let mut public_input_bytes = Vec::new();
    public_inputs.serialize_uncompressed(&mut public_input_bytes)?;

    Ok((proof_bytes, public_input_bytes))
}

fn with_stdout_silenced<T>(f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    let stdout_fd = std::io::stdout().as_raw_fd();
    let saved_stdout = unsafe { dup(stdout_fd) };
    if saved_stdout < 0 {
        anyhow::bail!("failed to dup stdout");
    }

    let devnull = OpenOptions::new().write(true).open("/dev/null")?;
    if unsafe { dup2(devnull.as_raw_fd(), stdout_fd) } < 0 {
        unsafe {
            close(saved_stdout);
        }
        anyhow::bail!("failed to redirect stdout");
    }

    let result = f();

    let _ = std::io::stdout().flush();
    if unsafe { dup2(saved_stdout, stdout_fd) } < 0 {
        unsafe {
            close(saved_stdout);
        }
        anyhow::bail!("failed to restore stdout");
    }
    unsafe {
        close(saved_stdout);
    }

    result
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(2 + bytes.len() * 2);
    out.push_str("0x");
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
