use crate::*;

use ark_bls12_377::Fr;
use ark_crypto_primitives::encryption::AsymmetricEncryptionScheme;
use ark_ec::AffineCurve;
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::select::CondSelectGadget;
use ark_r1cs_std::ToBitsGadget;
use ark_relations::r1cs::ConstraintSystemRef;
use ark_relations::{lc, r1cs::ConstraintSynthesizer};
use ark_std::test_rng;
use ark_std::{One, Zero};
use mpc_algebra::malicious_majority as mm;
use mpc_algebra::mpc_fields::MpcFieldVar;
use mpc_algebra::BooleanWire;
use mpc_algebra::EqualityZero;
use mpc_algebra::LessThan;
use mpc_algebra::MpcCondSelectGadget;
use mpc_algebra::MpcEqGadget;
use mpc_algebra::MpcFpVar;
use mpc_algebra::MpcToBitsGadget;
use mpc_algebra::Reveal;
use zk_mpc::circuits::serialize::werewolf;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> MpcCircuit<F>
    for AnonymousVotingCircuit<F>
{
    type Private = AnonymousVotingPrivateInput<F>;
    type Public = AnonymousVotingPublicInput<F>;

    fn combine_inputs(individuals: Vec<Self::Private>, public: Self::Public) -> Self {
        AnonymousVotingCircuit {
            private_input: individuals,
            public_input: public,
        }
    }

    fn validate(&self) -> Result<(), anyhow::Error> {
        // Implement validation logic here
        Ok(())
    }
}

impl AnonymousVotingCircuit<Fr> {
    pub fn calculate_output(&self) -> Fr {
        let player_num = self.private_input.len(); // Assuming all players join generation
        let alive_player_num = player_num; // Assuming all players are alive for simplicity

        let mut num_voted = vec![Fr::zero(); player_num];

        for i in 0..player_num {
            for j in 0..alive_player_num {
                num_voted[i] += self.private_input[j].is_target_id[i];
            }
        }

        let mut most_voted_id = Fr::zero();
        let mut max_votes = Fr::zero();

        for i in 0..player_num {
            if num_voted[i] > max_votes {
                most_voted_id = Fr::from(i as u32);
                max_votes = num_voted[i];
            }
        }
        most_voted_id
    }
}

impl AnonymousVotingCircuit<mm::MpcField<Fr>> {
    pub fn calculate_output(&self) -> mm::MpcField<Fr> {
        let player_num = self.private_input.len(); // Assuming all players join generation
        let alive_player_num = player_num; // Assuming all players are alive for simplicity

        let mut num_voted = vec![mm::MpcField::<Fr>::zero(); player_num];

        for i in 0..player_num {
            for j in 0..alive_player_num {
                num_voted[i] += self.private_input[j].is_target_id[i];
            }
        }

        let mut most_voted_id = mm::MpcField::<Fr>::zero();
        let mut max_votes = mm::MpcField::<Fr>::zero();

        for i in 0..player_num {
            max_votes +=
                (num_voted[i] - max_votes) * max_votes.sync_is_smaller_than(&num_voted[i]).field();

            most_voted_id += (mm::MpcField::<Fr>::from(i as u32) - most_voted_id)
                * max_votes.sync_is_smaller_than(&num_voted[i]).field();
        }
        most_voted_id
    }
}

impl ConstraintSynthesizer<Fr> for AnonymousVotingCircuit<Fr> {
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<Fr>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // Implement constraint generation logic here
        // initialize
        let player_num = self.private_input[0].is_target_id.len();
        let alive_player_num = self.private_input.len();

        // check player commitment
        // for i in 0..player_num {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        let is_target_id_var = self
            .private_input
            .iter()
            .map(|input| {
                input
                    .is_target_id
                    .iter()
                    .map(|b| FpVar::new_witness(cs.clone(), || Ok(*b)))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        let is_most_voted_id_var = FpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;

        // calculate
        let mut num_voted_var = Vec::new();

        for i in 0..player_num {
            let mut each_num_voted = <FpVar<Fr> as Zero>::zero();

            for j in 0..alive_player_num {
                each_num_voted += is_target_id_var[j][i].clone();
            }

            num_voted_var.push(each_num_voted);
        }

        let constant = (0..4)
            .map(|i| FpVar::Constant(Fr::from(i as i32)))
            .collect::<Vec<_>>();

        let mut calced_is_most_voted_id = FpVar::new_witness(cs.clone(), || Ok(Fr::zero()))?;

        for i in 0..player_num {
            let a_now = FpVar::conditionally_select_power_of_two_vector(
                &calced_is_most_voted_id.to_bits_le().unwrap()[..2],
                &constant,
            )?;

            let res = FpVar::is_cmp(
                //&num_voted_var[calced_is_most_voted_id],
                &a_now,
                &num_voted_var[i],
                std::cmp::Ordering::Greater,
                true,
            )?;

            let false_value = FpVar::new_witness(cs.clone(), || Ok(Fr::from(i as i32)))?;

            calced_is_most_voted_id =
                FpVar::conditionally_select(&res, &calced_is_most_voted_id, &false_value)?;
        }

        // enforce equal
        is_most_voted_id_var.enforce_equal(&calced_is_most_voted_id);

        println!("total number of constraints: {}", cs.num_constraints());
        Ok(())
    }
}

impl ConstraintSynthesizer<mm::MpcField<Fr>> for AnonymousVotingCircuit<mm::MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<mm::MpcField<Fr>>,
    ) -> ark_relations::r1cs::Result<()> {
        // initialize
        let player_num = self.private_input[0].is_target_id.len();
        let alive_player_num = self.private_input.len();

        // check player commitment
        // for i in 0..player_num {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open:
        //             <mm::MpcField<Fr> as LocalOrMPC<mm::MpcField<Fr>>>::PedersenRandomness::default(
        //             ),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        let is_target_id_var = self
            .private_input
            .iter()
            .map(|input| {
                input
                    .is_target_id
                    .iter()
                    .map(|b| MpcFpVar::new_witness(cs.clone(), || Ok(*b)))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        let is_most_voted_id_var = MpcFpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;

        // calculate
        let mut num_voted_var = Vec::new();

        for i in 0..player_num {
            let mut each_num_voted = <MpcFpVar<mm::MpcField<Fr>> as MpcFieldVar<
                mm::MpcField<Fr>,
                mm::MpcField<Fr>,
            >>::zero();

            for j in 0..alive_player_num {
                each_num_voted += is_target_id_var[j][i].clone();
            }

            num_voted_var.push(each_num_voted);
        }

        let constant = (0..4)
            .map(|i| {
                MpcFpVar::Constant(mm::MpcField::<Fr>::king_share(
                    Fr::from(i as i32),
                    &mut test_rng(),
                ))
            })
            .collect::<Vec<_>>();

        let mut calced_is_most_voted_id = MpcFpVar::new_witness(cs.clone(), || {
            Ok(mm::MpcField::<Fr>::king_share(Fr::zero(), &mut test_rng()))
        })?;

        for i in 0..player_num {
            let a_now = MpcFpVar::conditionally_select_power_of_two_vector(
                &calced_is_most_voted_id.to_bits_le().unwrap()[..2],
                &constant,
            )?;

            let res = MpcFpVar::is_cmp(
                //&num_voted_var[calced_is_most_voted_id],
                &a_now,
                &num_voted_var[i],
                std::cmp::Ordering::Greater,
                true,
            )?;

            let false_value = MpcFpVar::new_witness(cs.clone(), || {
                Ok(mm::MpcField::<Fr>::king_share(
                    Fr::from(i as i32),
                    &mut test_rng(),
                ))
            })?;

            calced_is_most_voted_id =
                MpcFpVar::conditionally_select(&res, &calced_is_most_voted_id, &false_value)?;
        }

        // enforce equal
        is_most_voted_id_var.enforce_equal(&calced_is_most_voted_id);

        println!("total number of constraints: {}", cs.num_constraints());

        Ok(())
    }
}

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> ConstraintSynthesizer<F>
    for KeyPublicizeCircuit<F>
{
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<F>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // Implement constraint generation logic here
        Ok(())
    }
}

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> ConstraintSynthesizer<F>
    for DivinationCircuit<F>
{
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<F>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // Implement constraint generation logic here
        Ok(())
    }
}

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> ConstraintSynthesizer<F>
    for RoleAssignmentCircuit<F>
{
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<F>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // Implement constraint generation logic here
        Ok(())
    }
}

impl WinningJudgementCircuit<Fr> {
    pub fn calculate_output(&self) -> Fr {
        let werewolf_count = self
            .private_input
            .iter()
            .filter(|input| input.am_werewolf.is_one())
            .count();

        let villagers_count = self
            .private_input
            .iter()
            .filter(|input| input.am_werewolf.is_zero())
            .count();

        let game_state = if werewolf_count == 0 {
            Fr::from(2u32)
        } else if werewolf_count >= villagers_count {
            Fr::from(1u32)
        } else {
            Fr::from(3u32)
        };
        game_state
    }
}

impl WinningJudgementCircuit<mm::MpcField<Fr>> {
    pub fn calculate_output(&self) -> mm::MpcField<Fr> {
        let alive_player_num = self.private_input.len();

        let werewolf_count = self
            .private_input
            .iter()
            .fold(mm::MpcField::<Fr>::zero(), |acc, input| {
                acc + input.am_werewolf
            });

        let villagers_count = mm::MpcField::<Fr>::from(alive_player_num as u32) - werewolf_count;

        let no_werewolf = werewolf_count.sync_is_zero_shared();

        let game_state = no_werewolf.field() * mm::MpcField::<Fr>::from(2_u32)
            + (!no_werewolf).field()
                * (werewolf_count
                    .sync_is_smaller_than(&villagers_count)
                    .field()
                    * mm::MpcField::<Fr>::from(3_u32)
                    + (mm::MpcField::<Fr>::one()
                        - (werewolf_count.sync_is_smaller_than(&villagers_count)).field())
                        * mm::MpcField::<Fr>::from(1_u32));
        game_state
    }
}

impl DivinationCircuit<Fr> {
    pub fn calculate_output(&self) -> Fr {
        todo!()
    }
}

impl DivinationCircuit<mm::MpcField<Fr>> {
    pub fn calculate_output(
        &self,
    ) -> <mm::MpcField<Fr> as ElGamalLocalOrMPC<mm::MpcField<Fr>>>::ElGamalCiphertext {
        let is_target_vec = self
            .private_input
            .iter()
            .map(|input| input.is_target.clone())
            .collect::<Vec<_>>();
        let is_werewolf_vec = self
            .private_input
            .iter()
            .map(|input| input.is_werewolf)
            .collect::<Vec<_>>();

        let mut sum = mm::MpcField::<Fr>::default();
        for (t, w) in is_target_vec.iter().zip(is_werewolf_vec.iter()) {
            sum += t.iter().fold(mm::MpcField::<Fr>::zero(), |acc, x| acc + x) * w;
        }

        let pub_key = self.public_input.pub_key;

        let base = <mm::MpcField<Fr> as ElGamalLocalOrMPC<mm::MpcField<Fr>>>::ElGamalPlaintext::prime_subgroup_generator();

        // TODO: implement correctly. (without reveal)
        let message = if sum.sync_reveal().is_one() {
            base
        } else {
            <mm::MpcField<Fr> as ElGamalLocalOrMPC<mm::MpcField<Fr>>>::ElGamalPlaintext::default()
        };

        let ciphertext =
            <mm::MpcField<Fr> as ElGamalLocalOrMPC<mm::MpcField<Fr>>>::ElGamalScheme::encrypt(
                &self.public_input.elgamal_param,
                &pub_key,
                &message,
                &self.private_input[0].randomness,
            )
            .unwrap();
        ciphertext
    }
}

impl KeyPublicizeCircuit<mm::MpcField<Fr>> {
    pub fn calculate_output(&self) -> mm::MpcField<Fr> {
        todo!()
    }
}

impl RoleAssignmentCircuit<mm::MpcField<Fr>> {
    pub fn calculate_output(&self) -> mm::MpcField<Fr> {
        // let num_players = circuit.num_players;
        // let max_group_size = circuit.max_group_size;
        // let pedersen_param = circuit.pedersen_param.clone();
        // let tau_matrix = circuit.tau_matrix.clone();
        // let role_commitment = circuit.role_commitment.clone();
        // let player_commitment = circuit.player_commitment.clone();
        // let shuffle_matrices = circuit.shuffle_matrices.clone();
        // let randomness = circuit.randomness.clone();
        // let player_randomness = circuit.player_randomness.clone();
        // let mut buffer = Vec::new();
        // // CanonicalSerialize::serialize(&num_players, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&max_group_size, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&pedersen_param, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&tau_matrix, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&role_commitment, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&player_commitment, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&shuffle_matrices, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&randomness, &mut buffer).unwrap();
        // // CanonicalSerialize::serialize(&player_randomness, &mut buffer).unwrap();
        // buffer
        todo!()
    }
}

impl ConstraintSynthesizer<Fr> for WinningJudgementCircuit<Fr> {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> ark_relations::r1cs::Result<()> {
        // check player commitment
        // let player_num = self.player_randomness.len();
        let alive_player_num = self.private_input.len();
        // for i in 0..alive_player_num {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        // initialize
        let num_alive_var = FpVar::new_input(cs.clone(), || Ok(Fr::from(alive_player_num as u32)))?;

        let am_werewolf_var = self
            .private_input
            .iter()
            .map(|input| input.am_werewolf)
            .collect::<Vec<_>>();

        let game_state_var = FpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;

        // calculate
        let num_werewolf_var =
            am_werewolf_var
                .iter()
                .fold(<FpVar<Fr> as Zero>::zero(), |mut acc, x| {
                    acc += *x;
                    acc
                });

        let num_citizen_var = num_alive_var - &num_werewolf_var;

        let calced_game_state_var = FpVar::conditionally_select(
            &FieldVar::is_zero(&num_werewolf_var)?,
            &FpVar::constant(Fr::from(2)), // villager win
            &FpVar::conditionally_select(
                &num_werewolf_var.is_cmp(&num_citizen_var, std::cmp::Ordering::Less, false)?,
                &FpVar::constant(Fr::from(3)), // game continues
                &FpVar::constant(Fr::from(1)), // werewolf win
            )?,
        )?;

        // // check commitment
        // for am_werewolf_with_commit in self.am_werewolf.iter() {
        //     let am_werewolf_com_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: am_werewolf_with_commit.input,
        //         open: am_werewolf_with_commit.randomness.clone(),
        //         commit: am_werewolf_with_commit.commitment.clone(),
        //     };

        //     am_werewolf_com_circuit.generate_constraints(cs.clone())?;
        // }

        // enforce equal
        game_state_var.enforce_equal(&calced_game_state_var)?;

        println!("total number of constraints: {}", cs.num_constraints());

        Ok(())
    }
}

impl ConstraintSynthesizer<mm::MpcField<Fr>> for WinningJudgementCircuit<mm::MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<mm::MpcField<Fr>>,
    ) -> ark_relations::r1cs::Result<()> {
        // check player commitment
        // let player_num = self.player_randomness.len();
        let alive_player_num = self.private_input.len();
        // for i in 0..alive_player_num {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        // initialize
        let num_alive_var = MpcFpVar::new_input(cs.clone(), || {
            Ok(mm::MpcField::<Fr>::from(alive_player_num as u32))
        })?;

        let am_werewolf_var = self
            .private_input
            .iter()
            .map(|input| input.am_werewolf)
            .collect::<Vec<_>>();

        // let game_state_var = MpcFpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;
        // let game_state_var =
        //     MpcFpVar::new_input(cs.clone(), || Ok(mm::MpcField::<Fr>::from(0_u32)))?;

        // calculate
        let num_werewolf_var = am_werewolf_var.iter().fold(
            <MpcFpVar<mm::MpcField<Fr>> as Zero>::zero(),
            |mut acc, x| {
                acc += *x;
                acc
            },
        );

        let num_citizen_var = num_alive_var - &num_werewolf_var;

        let calced_game_state_var = MpcFpVar::conditionally_select(
            &MpcFieldVar::is_zero(&num_werewolf_var)?,
            &MpcFpVar::constant(mm::MpcField::<Fr>::from(2_u32)), // villager win
            &MpcFpVar::conditionally_select(
                &num_werewolf_var.is_cmp(&num_citizen_var, std::cmp::Ordering::Less, false)?,
                &MpcFpVar::constant(mm::MpcField::<Fr>::from(3_u32)), // game continues
                &MpcFpVar::constant(mm::MpcField::<Fr>::from(1_u32)), // werewolf win
            )?,
        )?;

        // // check commitment
        // for am_werewolf_with_commit in self.am_werewolf.iter() {
        //     let am_werewolf_com_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: am_werewolf_with_commit.input,
        //         open: am_werewolf_with_commit.randomness.clone(),
        //         commit: am_werewolf_with_commit.commitment.clone(),
        //     };

        //     am_werewolf_com_circuit.generate_constraints(cs.clone())?;
        // }

        // enforce equal
        // game_state_var.enforce_equal(&calced_game_state_var)?;

        println!("total number of constraints: {}", cs.num_constraints());

        Ok(())
    }
}
