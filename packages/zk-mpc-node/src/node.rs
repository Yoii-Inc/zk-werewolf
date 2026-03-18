use crate::crypto::KeyManager;
use crate::models::ProofRequest;
use crate::proof::ProofManager;
use crate::server::ApiClient;
use crate::{EncryptedShare, ProofOutput, ProofOutputType, UserPublicKey};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{generate_random_parameters, prepare_verifying_key, verify_proof, ProvingKey};
use ark_serialize::CanonicalDeserialize;
use ark_std::test_rng;
use mpc_algebra::{AdditivePairingShare, MpcPairingEngine, Reveal};
use mpc_algebra_wasm::{CircuitEncryptedInputIdentifier, CircuitProfile};
use mpc_circuits::CircuitFactory;
use mpc_net::multi::MPCNetConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::zip;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite};
use zk_mpc::groth16::create_random_proof;

type LocalProvingKey = ProvingKey<ark_bn254::Bn254>;
type MPCProvingKey =
    ProvingKey<MpcPairingEngine<ark_bn254::Bn254, AdditivePairingShare<ark_bn254::Bn254>>>;

#[derive(Serialize)]
struct RoleSharePayload<'a> {
    schema_version: &'static str,
    role_share: &'a str,
    role_share_encoding: &'static str,
    werewolf_mates_mask_share: &'a str,
    werewolf_mates_mask_share_encoding: &'static str,
}

#[derive(Deserialize)]
struct RoleOutputV2 {
    role_share: String,
    #[serde(default)]
    werewolf_mates_mask_share: Option<String>,
}

#[derive(Clone)]
struct Groth16Setup {
    local_proving_key: Arc<LocalProvingKey>,
}

impl Groth16Setup {
    fn from_pk_path(path: &PathBuf, label: &str) -> Result<Self, std::io::Error> {
        let total_started = Instant::now();
        println!(
            "[node:init][groth16] start loading {label} PK from {}",
            path.display()
        );

        let read_started = Instant::now();
        let bytes = std::fs::read(path)?;
        println!(
            "[node:init][groth16] read {} bytes for {label} PK from {} in {} ms",
            bytes.len(),
            path.display(),
            read_started.elapsed().as_millis()
        );

        let deserialize_started = Instant::now();
        let local_proving_key = LocalProvingKey::deserialize_uncompressed(bytes.as_slice())
            .map_err(|e| {
                std::io::Error::other(format!(
                    "failed to deserialize {label} proving key {}: {:?}",
                    path.display(),
                    e
                ))
            })?;
        println!(
            "[node:init][groth16] finished deserializing {label} PK from {} in {} ms (total {} ms)",
            path.display(),
            deserialize_started.elapsed().as_millis(),
            total_started.elapsed().as_millis()
        );

        Ok(Self {
            local_proving_key: Arc::new(local_proving_key),
        })
    }

    fn prepared_verifying_key(&self) -> ark_groth16::PreparedVerifyingKey<ark_bn254::Bn254> {
        prepare_verifying_key(&self.local_proving_key.vk)
    }

    fn mpc_proving_key(&self) -> MPCProvingKey {
        ProvingKey::from_public((*self.local_proving_key).clone())
    }
}

#[derive(Clone, Default)]
struct CircuitGroth16Setups {
    by_profile: HashMap<CircuitProfile, Groth16Setup>,
    role_assignment: Option<Groth16Setup>,
    divination: Option<Groth16Setup>,
    anonymous_voting: Option<Groth16Setup>,
    winning_judgement: Option<Groth16Setup>,
    key_publicize: Option<Groth16Setup>,
}

impl CircuitGroth16Setups {
    fn load() -> Result<Self, std::io::Error> {
        let started = Instant::now();
        println!("[node:init][groth16] loading all Groth16 setups...");

        let mut by_profile = HashMap::new();
        load_profile_setups_from_data_dir(&mut by_profile)?;

        let setup = Self {
            by_profile,
            role_assignment: load_setup_with_fallback(
                "ROLE_ASSIGNMENT_GROTH16_PK_PATH",
                "role_assignment_max5_v1.pk",
                "RoleAssignment",
            )?,
            divination: load_setup_with_fallback(
                "DIVINATION_GROTH16_PK_PATH",
                "divination_max5_v1.pk",
                "Divination",
            )?,
            anonymous_voting: load_setup_with_fallback(
                "ANONYMOUS_VOTING_GROTH16_PK_PATH",
                "anonymous_voting_max5_v1.pk",
                "AnonymousVoting",
            )?,
            winning_judgement: load_setup_with_fallback(
                "WINNING_JUDGEMENT_GROTH16_PK_PATH",
                "winning_judgement_max5_v1.pk",
                "WinningJudgement",
            )?,
            key_publicize: load_setup_with_fallback(
                "KEY_PUBLICIZE_GROTH16_PK_PATH",
                "key_publicize_max5_v1.pk",
                "KeyPublicize",
            )?,
        };

        let fallback_loaded = [
            setup.role_assignment.is_some(),
            setup.divination.is_some(),
            setup.anonymous_voting.is_some(),
            setup.winning_judgement.is_some(),
            setup.key_publicize.is_some(),
        ]
        .into_iter()
        .filter(|loaded| *loaded)
        .count();

        println!(
            "[node:init][groth16] completed loading Groth16 setups in {} ms (profile_setups={}, fallback_setups={})",
            started.elapsed().as_millis(),
            setup.by_profile.len(),
            fallback_loaded
        );

        Ok(setup)
    }

    fn for_circuit(&self, circuit_type: &CircuitEncryptedInputIdentifier) -> Option<&Groth16Setup> {
        if let Some(profile) = circuit_type.circuit_profile() {
            if let Some(setup) = self.by_profile.get(&profile) {
                return Some(setup);
            }
            // プロファイルが判別できるのに一致する setup がない場合、
            // max5 等へのフォールバックを行うと不正な鍵で証明してしまうため禁止する。
            return None;
        }

        match circuit_type {
            CircuitEncryptedInputIdentifier::RoleAssignment(_) => self.role_assignment.as_ref(),
            CircuitEncryptedInputIdentifier::Divination(_) => self.divination.as_ref(),
            CircuitEncryptedInputIdentifier::AnonymousVoting(_) => self.anonymous_voting.as_ref(),
            CircuitEncryptedInputIdentifier::WinningJudge(_) => self.winning_judgement.as_ref(),
            CircuitEncryptedInputIdentifier::KeyPublicize(_) => self.key_publicize.as_ref(),
        }
    }
}

pub struct Node<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static> {
    pub id: u32,
    pub net: Arc<MPCNetConnection<IO>>,
    pub proof_manager: Arc<ProofManager>,
    pub key_manager: Arc<KeyManager>,
    pub api_client: Arc<ApiClient>,
    groth16_setups: CircuitGroth16Setups,
}

impl<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static> Node<IO> {
    pub async fn new(
        id: u32,
        net: MPCNetConnection<IO>,
        proof_manager: Arc<ProofManager>,
        key_manager: Arc<KeyManager>,
        server_url: String,
    ) -> Self {
        let init_started = Instant::now();
        println!("[node:init] start node initialization: id={id}");

        let key_load_started = Instant::now();
        // 環境変数から秘密鍵と公開鍵を取得（優先）、なければファイルから読込
        if let Ok(private_key_base64) = std::env::var("MPC_PRIVATE_KEY") {
            println!("[node:init] loading node keypair from environment variables");
            // 本番環境：環境変数から取得
            let public_key_env_name = format!("MPC_NODE_{}_PUBLIC_KEY", id);
            if let Ok(public_key_base64) = std::env::var(&public_key_env_name) {
                // Base64デコード
                let private_key_bytes = base64::decode(&private_key_base64)
                    .expect("Failed to decode MPC_PRIVATE_KEY from base64");
                let public_key_bytes = base64::decode(&public_key_base64).unwrap_or_else(|_| {
                    panic!("Failed to decode {} from base64", public_key_env_name)
                });

                key_manager
                    .set_keys_from_base64_bytes(private_key_bytes, public_key_bytes)
                    .await
                    .expect("Failed to set keys from environment variables");
                println!(
                    "[node:init] loaded node keypair from env in {} ms",
                    key_load_started.elapsed().as_millis()
                );
            } else {
                panic!(
                    "Environment variable {} not found. Please set both MPC_PRIVATE_KEY and {}",
                    public_key_env_name, public_key_env_name
                );
            }
        } else {
            println!("[node:init] MPC_PRIVATE_KEY is not set. Loading node keypair from file...");
            // 開発環境：ファイルから読込
            key_manager
                .load_keypair(id)
                .await
                .expect("Failed to load keypair from file");
            println!(
                "[node:init] loaded node keypair from file in {} ms",
                key_load_started.elapsed().as_millis()
            );
        }

        let api_client = Arc::new(ApiClient::new(server_url.clone()));
        println!("[node:init] created API client for server URL: {}", server_url);

        let groth16_started = Instant::now();
        println!("[node:init] start loading Groth16 PK setups...");
        let groth16_setups = CircuitGroth16Setups::load().unwrap_or_else(|e| {
            panic!("Failed to load Groth16 proving key(s): {}", e);
        });
        println!(
            "[node:init] finished loading Groth16 PK setups in {} ms",
            groth16_started.elapsed().as_millis()
        );

        let node = Self {
            id,
            net: Arc::new(net),
            proof_manager,
            key_manager,
            api_client: api_client.clone(),
            groth16_setups,
        };

        // 生成した公開鍵をサーバーに登録
        let register_started = Instant::now();
        println!(
            "[node:init] start register_public_key for node {}...",
            node.id
        );
        node.register_public_key()
            .await
            .expect("Failed to register public key with server");
        println!(
            "[node:init] finished register_public_key for node {} in {} ms",
            node.id,
            register_started.elapsed().as_millis()
        );

        println!(
            "[node:init] node initialization completed: id={}, total={} ms",
            node.id,
            init_started.elapsed().as_millis()
        );

        node
    }

    // 公開鍵を登録するメソッドを追加
    pub async fn register_public_key(&self) -> Result<(), Box<dyn std::error::Error>> {
        let started = Instant::now();
        println!(
            "[node:init] register_public_key: fetching local public key for node {}",
            self.id
        );
        let public_key = self.key_manager.get_public_key().await?;
        println!(
            "[node:init] register_public_key: fetched local public key for node {} (length={})",
            self.id,
            public_key.len()
        );
        println!(
            "[node:init] register_public_key: sending request to backend for node {}",
            self.id
        );
        self.api_client
            .register_public_key(self.id, public_key)
            .await?;
        println!(
            "[node:init] register_public_key: backend registration succeeded for node {} in {} ms",
            self.id,
            started.elapsed().as_millis()
        );
        Ok(())
    }

    pub async fn generate_proof(
        &self,
        request: ProofRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let pm = self.proof_manager.clone();
        if let Some(profile) = request.circuit_type.circuit_profile() {
            if !profile.is_supported_onchain_profile() {
                println!(
                    "Warning: unsupported on-chain profile requested: {} (proof generation continues off-chain)",
                    circuit_profile_label(profile)
                );
            }
        }

        // Setup circuit
        let local_circuit = CircuitFactory::create_local_circuit(&request.circuit_type);

        let secret_key = self
            .key_manager
            .get_secret_key()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        let mpc_circuit = CircuitFactory::create_mpc_circuit(
            &request.circuit_type,
            &self.id.to_string(),
            &secret_key,
        );

        let inputs = CircuitFactory::create_verify_inputs(&mpc_circuit);

        let rng = &mut test_rng();
        let (pvk, mpc_params): (_, MPCProvingKey) = if let Some(setup) =
            self.groth16_setups.for_circuit(&request.circuit_type)
        {
            (setup.prepared_verifying_key(), setup.mpc_proving_key())
        } else {
            let params = generate_random_parameters::<ark_bn254::Bn254, _, _>(local_circuit, rng)
                .map_err(|e| -> Box<dyn std::error::Error + Send> {
                Box::new(std::io::Error::other(format!(
                    "Failed to generate Groth16 parameters: {:?}",
                    e
                )))
            })?;
            let pvk = prepare_verifying_key(&params.vk);
            (pvk, ProvingKey::from_public(params))
        };

        let mpc_proof = create_random_proof::<
            MpcPairingEngine<ark_bn254::Bn254, AdditivePairingShare<ark_bn254::Bn254>>,
            _,
            _,
        >(mpc_circuit.clone(), &mpc_params, rng)
        .map_err(|e| -> Box<dyn std::error::Error + Send> {
            Box::new(std::io::Error::other(format!(
                "Failed to generate collaborative Groth16 proof: {:?}",
                e
            )))
        })?;
        let publicized_proof = mpc_proof.reveal().await;
        let is_valid = verify_proof(&pvk, &publicized_proof, &inputs).map_err(
            |e| -> Box<dyn std::error::Error + Send> {
                Box::new(std::io::Error::other(format!(
                    "Failed to verify Groth16 proof: {:?}",
                    e
                )))
            },
        )?;

        let outputs = if is_valid {
            let proof_bytes = abi_encode_groth16_proof(&publicized_proof);
            let public_input_len = expected_public_input_len(&request.circuit_type);
            let public_input_bytes = Some(
                abi_encode_fixed_uint256_inputs(&inputs, public_input_len).map_err(
                    |e| -> Box<dyn std::error::Error + Send> {
                        Box::new(std::io::Error::other(format!(
                            "Failed to encode public inputs: {}",
                            e
                        )))
                    },
                )?,
            );

            let proof_outputs = CircuitFactory::get_circuit_outputs(&mpc_circuit);
            let proof_output = match &request.output_type {
                ProofOutputType::Public => ProofOutput {
                    output_type: request.output_type.clone(),
                    value: Some(proof_outputs),
                    proof: Some(proof_bytes.clone()),
                    public_inputs: public_input_bytes.clone(),
                    shares: None,
                },
                ProofOutputType::PrivateToPublic(pubkeys) => {
                    // TODO: 出力をシェアに分割して暗号化
                    let shares = self
                        .split_and_encrypt_output(&proof_outputs, pubkeys)
                        .await?;
                    ProofOutput {
                        output_type: request.output_type.clone(),
                        value: None,
                        proof: Some(proof_bytes.clone()),
                        public_inputs: public_input_bytes.clone(),
                        shares: Some(shares),
                    }
                }
                ProofOutputType::PrivateToPrivate(pubkey) => {
                    // TODO: 出力を直接暗号化
                    let encrypted = self.encrypt_output(&proof_outputs, pubkey).await?;
                    ProofOutput {
                        output_type: request.output_type.clone(),
                        value: Some(encrypted),
                        proof: Some(proof_bytes),
                        public_inputs: public_input_bytes,
                        shares: None,
                    }
                }
            };
            Some(proof_output)
        } else {
            None
        };

        println!(
            "output is {:?}",
            CircuitFactory::get_circuit_outputs(&mpc_circuit)
        );

        if is_valid {
            pm.update_proof_status_with_output(
                &request.proof_id,
                "completed",
                Some("Proof generated successfully".to_string()),
                outputs,
            )
            .await;
        } else {
            pm.update_proof_status(
                &request.proof_id,
                "failed",
                Some("Proof verification failed".to_string()),
            )
            .await;
        }

        Ok(())
    }

    async fn split_and_encrypt_output(
        &self,
        output: &[u8],
        pubkeys: &[UserPublicKey],
    ) -> Result<Vec<EncryptedShare>, Box<dyn std::error::Error + Send>> {
        // outputをJSONとしてパース（RoleAssignmentの場合は share 配列）
        let output_str = std::str::from_utf8(output)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        let parsed_role_outputs_v2 = serde_json::from_str::<Vec<RoleOutputV2>>(output_str).ok();
        let parsed_role_shares_v1 = serde_json::from_str::<Vec<String>>(output_str).ok();

        // 各シェアを暗号化
        let mut encrypted_shares = Vec::new();
        if let Some(role_outputs) = parsed_role_outputs_v2 {
            if role_outputs.len() < pubkeys.len() {
                return Err(Box::new(std::io::Error::other(format!(
                    "role share length mismatch: got {}, expected at least {}",
                    role_outputs.len(),
                    pubkeys.len()
                ))));
            }
            for (role_output, pubkey) in zip(role_outputs.iter(), pubkeys.iter()) {
                let share_payload = RoleSharePayload {
                    schema_version: "role_assignment_share_v2",
                    role_share: &role_output.role_share,
                    role_share_encoding: "bn254_fr_decimal_string",
                    werewolf_mates_mask_share: role_output
                        .werewolf_mates_mask_share
                        .as_deref()
                        .unwrap_or("0"),
                    werewolf_mates_mask_share_encoding: "player_index_bitmask_lsb0",
                };
                let share_bytes = serde_json::to_vec(&share_payload)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
                let encrypted = self
                    .key_manager
                    .encrypt_share(&share_bytes, &pubkey.public_key)
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
                encrypted_shares.push(EncryptedShare {
                    node_id: self.id,
                    user_id: pubkey.user_id.clone(),
                    encrypted_data: encrypted,
                });
            }
            return Ok(encrypted_shares);
        }

        if let Some(role_shares) = parsed_role_shares_v1 {
            if role_shares.len() < pubkeys.len() {
                return Err(Box::new(std::io::Error::other(format!(
                    "role share length mismatch: got {}, expected at least {}",
                    role_shares.len(),
                    pubkeys.len()
                ))));
            }
            for (role_share, pubkey) in zip(role_shares.iter(), pubkeys.iter()) {
                let share_payload = RoleSharePayload {
                    schema_version: "role_assignment_share_v2",
                    role_share,
                    role_share_encoding: "bn254_fr_decimal_string",
                    werewolf_mates_mask_share: "0",
                    werewolf_mates_mask_share_encoding: "player_index_bitmask_lsb0",
                };
                let share_bytes = serde_json::to_vec(&share_payload)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
                let encrypted = self
                    .key_manager
                    .encrypt_share(&share_bytes, &pubkey.public_key)
                    .await
                    .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
                encrypted_shares.push(EncryptedShare {
                    node_id: self.id,
                    user_id: pubkey.user_id.clone(),
                    encrypted_data: encrypted,
                });
            }
            return Ok(encrypted_shares);
        }

        // その他の場合：全データを各プレイヤーに送る（従来の動作）
        for pubkey in pubkeys.iter() {
            let encrypted = self
                .key_manager
                .encrypt_share(output, &pubkey.public_key)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;
            encrypted_shares.push(EncryptedShare {
                node_id: self.id,
                user_id: pubkey.user_id.clone(),
                encrypted_data: encrypted,
            });
        }

        Ok(encrypted_shares)
    }

    async fn encrypt_output(
        &self,
        output: &[u8],
        pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send>> {
        self.key_manager
            .encrypt_share(output, pubkey)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)
    }

    pub async fn get_public_key(&self) -> Result<String, crate::crypto::CryptoError> {
        self.key_manager.get_public_key().await
    }

    pub async fn encrypt_share(
        &self,
        share: &[u8],
        recipient_public_key: &str,
    ) -> Result<Vec<u8>, crate::crypto::CryptoError> {
        self.key_manager
            .encrypt_share(share, recipient_public_key)
            .await
    }

    pub async fn decrypt_share(
        &self,
        encrypted_share: &[u8],
    ) -> Result<Vec<u8>, crate::crypto::CryptoError> {
        self.key_manager.decrypt_share(encrypted_share).await
    }
}

fn pk_path_from_env_or_default(env_var: &str, default_pk_file: &str) -> Option<PathBuf> {
    match std::env::var(env_var) {
        Ok(value) if !value.trim().is_empty() => {
            let path = PathBuf::from(value);
            println!(
                "[node:init][groth16] using {} from env: {}",
                env_var,
                path.display()
            );
            Some(path)
        }
        Ok(_) => {
            println!(
                "[node:init][groth16] {} is set but empty. Trying default {} in data dir",
                env_var, default_pk_file
            );
            let default_path = groth16_data_dir().join(default_pk_file);
            if default_path.exists() {
                println!(
                    "[node:init][groth16] using default PK for {}: {}",
                    env_var,
                    default_path.display()
                );
                Some(default_path)
            } else {
                println!(
                    "[node:init][groth16] default PK for {} is missing: {}",
                    env_var,
                    default_path.display()
                );
                None
            }
        }
        _ => {
            let default_path = groth16_data_dir().join(default_pk_file);
            if default_path.exists() {
                println!(
                    "[node:init][groth16] {} is not set. Using default PK: {}",
                    env_var,
                    default_path.display()
                );
                Some(default_path)
            } else {
                println!(
                    "[node:init][groth16] {} is not set and default PK is missing: {}",
                    env_var,
                    default_path.display()
                );
                None
            }
        }
    }
}

fn groth16_data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("GROTH16_DATA_DIR") {
        let path = PathBuf::from(value);
        if path.exists() {
            return path;
        }
    }

    let cargo_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/groth16");
    if cargo_dir.exists() {
        return cargo_dir;
    }

    let app_dir = PathBuf::from("/app/data/groth16");
    if app_dir.exists() {
        return app_dir;
    }

    cargo_dir
}

fn load_profile_setups_from_data_dir(
    setups: &mut HashMap<CircuitProfile, Groth16Setup>,
) -> Result<(), std::io::Error> {
    let started = Instant::now();
    let data_dir = groth16_data_dir();
    println!(
        "[node:init][groth16] scanning data dir for profile PKs: {}",
        data_dir.display()
    );
    if !data_dir.exists() {
        println!(
            "[node:init][groth16] data dir does not exist. skipping profile PK scan: {}",
            data_dir.display()
        );
        return Ok(());
    }

    let mut pk_paths = Vec::new();
    for entry in std::fs::read_dir(&data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pk") {
            continue;
        }
        pk_paths.push(path);
    }
    pk_paths.sort();
    println!(
        "[node:init][groth16] found {} PK files under {}",
        pk_paths.len(),
        data_dir.display()
    );

    for path in pk_paths {
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<invalid-utf8>");

        let Some(profile) = parse_profile_from_pk_filename(file_name) else {
            println!(
                "[node:init][groth16] skipping non-profile PK filename: {}",
                path.display()
            );
            continue;
        };

        let label = format!("{} profile", circuit_profile_label(profile));
        println!(
            "[node:init][groth16] loading profile PK {} from {}",
            label,
            path.display()
        );
        let setup = Groth16Setup::from_pk_path(&path, &label)?;
        setups.insert(profile, setup);
        println!(
            "Loaded {} Groth16 proving key from {}.",
            circuit_profile_label(profile),
            path.display()
        );
    }

    println!(
        "[node:init][groth16] completed profile PK scan/load in {} ms (loaded_profiles={})",
        started.elapsed().as_millis(),
        setups.len()
    );

    Ok(())
}

fn parse_profile_from_pk_filename(file_name: &str) -> Option<CircuitProfile> {
    let stem = file_name.strip_suffix(".pk")?;

    if let Some(rest) = stem.strip_prefix("role_assignment_n") {
        let (players, tail) = rest.split_once("_w")?;
        let (werewolves, _) = tail.split_once("_v")?;
        return Some(CircuitProfile::RoleAssignment {
            player_count: players.parse().ok()?,
            werewolf_count: werewolves.parse().ok()?,
        });
    }
    if let Some(rest) = stem.strip_prefix("role_assignment_max") {
        let (players, _) = rest.split_once("_v")?;
        let player_count: usize = players.parse().ok()?;
        let werewolf_count = default_werewolf_count_for_player_count(player_count);
        return Some(CircuitProfile::RoleAssignment {
            player_count,
            werewolf_count,
        });
    }

    parse_single_count_profile(stem, "divination")
        .map(|player_count| CircuitProfile::Divination { player_count })
        .or_else(|| {
            parse_single_count_profile(stem, "anonymous_voting")
                .map(|player_count| CircuitProfile::AnonymousVoting { player_count })
        })
        .or_else(|| {
            parse_single_count_profile(stem, "winning_judgement")
                .map(|player_count| CircuitProfile::WinningJudge { player_count })
        })
        .or_else(|| {
            parse_single_count_profile(stem, "key_publicize")
                .map(|player_count| CircuitProfile::KeyPublicize { player_count })
        })
}

fn parse_single_count_profile(stem: &str, prefix: &str) -> Option<usize> {
    if let Some(rest) = stem.strip_prefix(&format!("{}_n", prefix)) {
        let (players, _) = rest.split_once("_v")?;
        return players.parse().ok();
    }
    if let Some(rest) = stem.strip_prefix(&format!("{}_max", prefix)) {
        let (players, _) = rest.split_once("_v")?;
        return players.parse().ok();
    }
    None
}

fn default_werewolf_count_for_player_count(player_count: usize) -> usize {
    if player_count <= 6 {
        1
    } else if player_count <= 9 {
        2
    } else {
        3
    }
}

fn circuit_profile_label(profile: CircuitProfile) -> String {
    match profile {
        CircuitProfile::RoleAssignment {
            player_count,
            werewolf_count,
        } => format!("role_assignment_n{}_w{}", player_count, werewolf_count),
        CircuitProfile::Divination { player_count } => format!("divination_n{}", player_count),
        CircuitProfile::AnonymousVoting { player_count } => {
            format!("anonymous_voting_n{}", player_count)
        }
        CircuitProfile::WinningJudge { player_count } => {
            format!("winning_judgement_n{}", player_count)
        }
        CircuitProfile::KeyPublicize { player_count } => format!("key_publicize_n{}", player_count),
    }
}

fn load_setup_with_fallback(
    env_var: &str,
    default_pk_file: &str,
    label: &str,
) -> Result<Option<Groth16Setup>, std::io::Error> {
    let started = Instant::now();
    let Some(path) = pk_path_from_env_or_default(env_var, default_pk_file) else {
        println!(
            "{env_var} is not set and default key is missing. Falling back to runtime Groth16 setup for {label}."
        );
        return Ok(None);
    };

    println!(
        "[node:init][groth16] loading {label} setup from {}",
        path.display()
    );
    let setup = Groth16Setup::from_pk_path(&path, label)?;
    println!(
        "Loaded {label} Groth16 proving key from {} in {} ms.",
        path.display(),
        started.elapsed().as_millis()
    );
    Ok(Some(setup))
}

fn abi_encode_groth16_proof(proof: &ark_groth16::Proof<ark_bn254::Bn254>) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 * 32);
    out.extend_from_slice(&field_to_word(proof.a.x));
    out.extend_from_slice(&field_to_word(proof.a.y));
    out.extend_from_slice(&field_to_word(proof.b.x.c0));
    out.extend_from_slice(&field_to_word(proof.b.x.c1));
    out.extend_from_slice(&field_to_word(proof.b.y.c0));
    out.extend_from_slice(&field_to_word(proof.b.y.c1));
    out.extend_from_slice(&field_to_word(proof.c.x));
    out.extend_from_slice(&field_to_word(proof.c.y));
    out
}

fn expected_public_input_len(circuit_type: &CircuitEncryptedInputIdentifier) -> usize {
    match circuit_type {
        CircuitEncryptedInputIdentifier::RoleAssignment(items) => {
            let Some(first) = items.first() else {
                return 0;
            };
            let n = first.public_input.grouping_parameter.get_num_players();
            let m = first.public_input.grouping_parameter.get_num_groups();
            let matrix_size = n + m;
            matrix_size * matrix_size
        }
        CircuitEncryptedInputIdentifier::Divination(_) => 8,
        CircuitEncryptedInputIdentifier::AnonymousVoting(_) => 1,
        CircuitEncryptedInputIdentifier::WinningJudge(_) => 2,
        CircuitEncryptedInputIdentifier::KeyPublicize(_) => 0,
    }
}

fn abi_encode_fixed_uint256_inputs<F: PrimeField>(
    inputs: &[F],
    expected_len: usize,
) -> Result<Vec<u8>, String> {
    if inputs.len() != expected_len {
        return Err(format!(
            "expected {} public inputs, got {}",
            expected_len,
            inputs.len()
        ));
    }

    let mut out = Vec::with_capacity(expected_len * 32);
    for input in inputs {
        out.extend_from_slice(&field_to_word(*input));
    }
    Ok(out)
}

fn field_to_word<F: PrimeField>(value: F) -> [u8; 32] {
    let mut le = value.into_repr().to_bytes_le();
    le.resize(32, 0);
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = le[31 - i];
    }
    out
}
