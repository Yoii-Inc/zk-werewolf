use crate::*;

use ark_bn254::Fr;
use ark_crypto_primitives::encryption::AsymmetricEncryptionScheme;
use ark_ec::{group, AffineCurve};
use ark_ff::{BigInteger, PrimeField, SquareRootField};
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::groups::CurveVar;
use ark_r1cs_std::prelude::Boolean;
use ark_r1cs_std::select::CondSelectGadget;
use ark_r1cs_std::{R1CSVar, ToBitsGadget};
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError};
use ark_relations::{lc, r1cs::ConstraintSynthesizer};
use ark_std::{rand, test_rng};
use ark_std::{One, Zero};
use mpc_algebra::groups::MpcCurveVar;
use mpc_algebra::mpc_fields::MpcFieldVar;
use mpc_algebra::LessThan;
use mpc_algebra::MpcBoolean;
use mpc_algebra::MpcCondSelectGadget;
use mpc_algebra::MpcEqGadget;
use mpc_algebra::MpcFpVar;
use mpc_algebra::MpcToBitsGadget;
use mpc_algebra::Reveal;
use mpc_algebra::{BitDecomposition, BooleanWire};
use mpc_algebra::{EqualityZero, ModulusConversion};
use mpc_algebra_wasm::{calc_shuffle_matrix, Role};
use nalgebra as na;
use zk_mpc::circuits::serialize::werewolf;
use zk_mpc::circuits::{ElGamalLocalOrMPC, LocalOrMPC};
use zk_mpc::field::*;

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

impl AnonymousVotingCircuit<MpcField<Fr>> {
    pub fn calculate_output(&self) -> MpcField<Fr> {
        let player_num = self.private_input.len(); // Assuming all players join generation
        let alive_player_num = player_num; // Assuming all players are alive for simplicity

        let mut num_voted = vec![MpcField::<Fr>::zero(); player_num];

        for i in 0..player_num {
            for j in 0..alive_player_num {
                num_voted[i] += self.private_input[j].is_target_id[i];
            }
        }

        let mut most_voted_id = MpcField::<Fr>::zero();
        let mut max_votes = MpcField::<Fr>::zero();

        for i in 0..player_num {
            max_votes +=
                (num_voted[i] - max_votes) * max_votes.sync_is_smaller_than(&num_voted[i]).field();

            most_voted_id += (MpcField::<Fr>::from(i as u32) - most_voted_id)
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

        // TODO: check player commitment
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

        println!(
            "[AnonymousVotingCircuit(Local)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[AnonymousVotingCircuit(Local)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[AnonymousVotingCircuit(Local)] total number of constraints: {}",
            cs.num_constraints()
        );
        Ok(())
    }
}

impl ConstraintSynthesizer<MpcField<Fr>> for AnonymousVotingCircuit<MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<MpcField<Fr>>,
    ) -> ark_relations::r1cs::Result<()> {
        // initialize
        let player_num = self.private_input[0].is_target_id.len();
        let alive_player_num = self.private_input.len();

        // TODO: check player commitment
        // for i in 0..player_num {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open:
        //             <MpcField<Fr> as LocalOrMPC<MpcField<Fr>>>::PedersenRandomness::default(
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
            let mut each_num_voted =
                <MpcFpVar<MpcField<Fr>> as MpcFieldVar<MpcField<Fr>, MpcField<Fr>>>::zero();

            for j in 0..alive_player_num {
                each_num_voted += is_target_id_var[j][i].clone();
            }

            num_voted_var.push(each_num_voted);
        }

        let constant = (0..4)
            .map(|i| {
                MpcFpVar::Constant(MpcField::<Fr>::king_share(
                    Fr::from(i as i32),
                    &mut test_rng(),
                ))
            })
            .collect::<Vec<_>>();

        let mut calced_is_most_voted_id = MpcFpVar::new_witness(cs.clone(), || {
            Ok(MpcField::<Fr>::king_share(Fr::zero(), &mut test_rng()))
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
                Ok(MpcField::<Fr>::king_share(
                    Fr::from(i as i32),
                    &mut test_rng(),
                ))
            })?;

            calced_is_most_voted_id =
                MpcFpVar::conditionally_select(&res, &calced_is_most_voted_id, &false_value)?;
        }

        // enforce equal
        is_most_voted_id_var.enforce_equal(&calced_is_most_voted_id);

        println!(
            "[AnonymousVotingCircuit(MPC)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[AnonymousVotingCircuit(MPC)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[AnonymousVotingCircuit(MPC)] total number of constraints: {}",
            cs.num_constraints()
        );

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
        let pk_x = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_x)
            .collect::<Vec<_>>();

        let pk_y = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_y)
            .collect::<Vec<_>>();

        let is_fortune_teller = self
            .private_input
            .iter()
            .map(|input| input.is_fortune_teller)
            .collect::<Vec<_>>();

        let x_var = pk_x
            .iter()
            .map(|x| FpVar::<F>::new_witness(ark_relations::ns!(cs, "gadget_randomness"), || Ok(x)))
            .collect::<Result<Vec<_>, _>>()?;

        let y_var = pk_y
            .iter()
            .map(|y| FpVar::<F>::new_witness(ark_relations::ns!(cs, "gadget_randomness"), || Ok(y)))
            .collect::<Result<Vec<_>, _>>()?;

        let is_ft_var = is_fortune_teller
            .iter()
            .map(|b| FpVar::<F>::new_witness(ark_relations::ns!(cs, "gadget_randomness"), || Ok(b)))
            .collect::<Result<Vec<_>, _>>()?;

        // is_fortune_teller = 0 or 1
        for b in is_ft_var.iter() {
            let is_zero = ark_r1cs_std::prelude::FieldVar::<F, F>::is_zero(b)?;
            let is_one = ark_r1cs_std::prelude::FieldVar::<F, F>::is_one(b)?;
            let is_bool = is_zero.or(&is_one)?;
            is_bool.enforce_equal(&Boolean::constant(true))?;
        }

        let _sum_x_var =
            x_var
                .iter()
                .enumerate()
                .fold(<FpVar<F> as Zero>::zero(), |mut acc, (i, x)| {
                    acc = acc + x * &is_ft_var[i];
                    acc
                });

        let _sum_y_var =
            y_var
                .iter()
                .enumerate()
                .fold(<FpVar<F> as Zero>::zero(), |mut acc, (i, y)| {
                    acc = acc + y * &is_ft_var[i];
                    acc
                });

        // TODO: Add verify commitments
        // self.verify_commitments(cs.clone())?;

        println!(
            "[KeyPublicizeCircuit] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[KeyPublicizeCircuit] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[KeyPublicizeCircuit] total number of constraints: {}",
            cs.num_constraints()
        );

        Ok(())
    }
}

impl ConstraintSynthesizer<Fr> for DivinationCircuit<Fr> {
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<Fr>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        let is_werewolf_bit = self
            .private_input
            .iter()
            .map(|input| Boolean::new_witness(cs.clone(), || Ok(input.is_werewolf.is_one())))
            .collect::<Result<Vec<_>, _>>()?;

        let is_target_bit = self
            .private_input
            .iter()
            .map(|input| {
                input
                    .is_target
                    .iter()
                    .map(|b| Boolean::new_witness(cs.clone(), || Ok(b.is_one())))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        let is_target_sum_bit = (0..is_target_bit[0].len())
            .map(|j| {
                Boolean::kary_or(
                    &is_target_bit
                        .iter() // i を動かす
                        .map(|row| row[j].clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let is_wt = is_werewolf_bit
            .iter()
            .zip(is_target_sum_bit.iter())
            .map(|(x, y)| x.and(y))
            .collect::<Result<Vec<_>, _>>()?;

        let is_target_werewolf_bit = Boolean::kary_or(is_wt.as_slice())?;

        let one_point = <Fr as ElGamalLocalOrMPC<Fr>>::EdwardsVar::new_witness(
            ark_relations::ns!(cs, "gadget_randomness"),
            || Ok(<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::prime_subgroup_generator()),
        )?;

        let zero_point = <Fr as ElGamalLocalOrMPC<Fr>>::EdwardsVar::new_witness(
            ark_relations::ns!(cs, "gadget_randomness"),
            || Ok(<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::default()),
        )?;

        let is_target_werewolf = is_target_werewolf_bit.select(&one_point, &zero_point)?;

        // elgamal encryption

        let param_var = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalParamVar::new_input(
            ark_relations::ns!(cs, "gadget_parameters"),
            || Ok(self.public_input.elgamal_param.clone()),
        )?;

        // let randomness_sum_var = randomness_var.iter().fold(
        //     <<Fr as ElGamalLocalOrMPC<Fr>>::ElGamalRandomnessVar>::default(),
        //     |mut acc, x| {
        //         acc += x;
        //         acc
        //     },
        // );

        let randomness_bits_var = self.private_input[0]
            .randomness
            .0
            .into_repr()
            .to_bits_le()
            .iter()
            .map(|b| Boolean::new_witness(cs.clone(), || Ok(*b)))
            .collect::<Result<Vec<_>, _>>()?;

        // allocate public key
        let pub_key_var = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPublicKeyVar::new_input(
            ark_relations::ns!(cs, "gadget_public_key"),
            || Ok(self.public_input.pub_key),
        )?;

        // allocate the output
        let enc_result_var = {
            // flatten randomness to little-endian bit vector
            let randomness = randomness_bits_var;

            // compute s = randomness*pk
            let s = Fr::get_public_key(&pub_key_var)
                .clone()
                .scalar_mul_le(randomness.iter())?;

            // compute c1 = randomness*generator
            let c1 = Fr::get_generator(&param_var)
                .clone()
                .scalar_mul_le(randomness.iter())?;

            // compute c2 = m + s
            let c2 = is_target_werewolf.clone() + s;

            <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalCiphertextVar::new(c1, c2)
        };

        // compare
        let enc_result_var2 = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalCiphertextVar::new_input(
            ark_relations::ns!(cs, "gadget_commitment"),
            || {
                let sum_target = (0..self.private_input.len())
                    .map(|i| {
                        let mut tmp = Fr::from(0);
                        for j in 0..self.private_input.len() {
                            tmp += self.private_input[j].is_target[i];
                        }
                        tmp
                    })
                    .collect::<Vec<_>>();
                let is_werewolf: Fr = self
                    .private_input
                    .iter()
                    .map(|input| input.is_werewolf)
                    .zip(sum_target.iter())
                    .map(|(x, y)| x * y)
                    .sum();

                let message = match is_werewolf.is_one() {
                    true => {
                        <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::prime_subgroup_generator()
                    }
                    false => <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalPlaintext::default(),
                };
                let enc_result = <Fr as ElGamalLocalOrMPC<Fr>>::ElGamalScheme::encrypt(
                    &self.public_input.elgamal_param,
                    &self.public_input.pub_key,
                    &message,
                    &self.private_input[0].randomness,
                )
                .unwrap();
                Ok(enc_result)
            },
        )?;

        enc_result_var.enforce_equal(&enc_result_var2)?;

        // TODO: Add verify commitments
        // self.verify_commitments(cs.clone())?;

        println!(
            "[DivinationCircuit(Local)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[DivinationCircuit(Local)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[DivinationCircuit(Local)] total number of constraints: {}",
            cs.num_constraints()
        );

        Ok(())
    }
}

impl ConstraintSynthesizer<MpcField<Fr>> for DivinationCircuit<MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<MpcField<Fr>>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // TODO: Implement constraint generation logic here

        let is_werewolf_bit = MpcBoolean::new_witness_vec(
            cs.clone(),
            &self
                .private_input
                .iter()
                .map(|input| input.is_werewolf)
                .collect::<Vec<_>>(),
        )?;

        let is_target_bit = self
            .private_input
            .iter()
            .map(|input| MpcBoolean::new_witness_vec(cs.clone(), &input.is_target))
            .collect::<Result<Vec<_>, _>>()?;

        let is_target_sum_bit = (0..is_target_bit[0].len())
            .map(|j| {
                MpcBoolean::kary_or(
                    &is_target_bit
                        .iter() // i を動かす
                        .map(|row| row[j].clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        let is_wt = is_werewolf_bit
            .iter()
            .zip(is_target_sum_bit.iter())
            .map(|(x, y)| x.and(y))
            .collect::<Result<Vec<_>, _>>()?;

        let is_target_werewolf_bit = MpcBoolean::kary_or(is_wt.as_slice())?;

        let one_point = <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::EdwardsVar::new_witness(
            ark_relations::ns!(cs, "gadget_randomness"),
            || {
                Ok(<MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::prime_subgroup_generator())
            },
        )?;

        let zero_point =
            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::EdwardsVar::new_witness(
                ark_relations::ns!(cs, "gadget_randomness"),
                || {
                    Ok(<MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::default())
                },
            )?;

        let is_target_werewolf =
            MpcField::<Fr>::select(&is_target_werewolf_bit, &one_point, &zero_point)?;

        // elgamal encryption

        let param_var =
            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalParamVar::new_input(
                ark_relations::ns!(cs, "gadget_parameters"),
                || Ok(self.public_input.elgamal_param.clone()),
            )?;

        let mut randomness_bits_mpc = self.private_input[0]
            .randomness
            .0
            .sync_bit_decomposition()
            .iter()
            .map(|b| b.field().sync_modulus_conversion())
            .collect::<Vec<_>>();

        // Pad with zeros to match Fr bit size (to_bits_le() returns fixed length)
        randomness_bits_mpc.resize(256, MpcField::<Fr>::zero());

        let randomness_bits_var = MpcBoolean::new_witness_vec(cs.clone(), &randomness_bits_mpc)?;

        // allocate public key
        let pub_key_var =
            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPublicKeyVar::new_input(
                ark_relations::ns!(cs, "gadget_public_key"),
                || Ok(self.public_input.pub_key),
            )?;

        // allocate the output
        let enc_result_var = {
            // flatten randomness to little-endian bit vector
            let randomness = randomness_bits_var;

            // compute s = randomness*pk
            let s = MpcField::<Fr>::get_public_key(&pub_key_var)
                .clone()
                .scalar_mul_le(randomness.iter())?;

            // compute c1 = randomness*generator
            let c1 = MpcField::<Fr>::get_generator(&param_var)
                .clone()
                .scalar_mul_le(randomness.iter())?;

            // compute c2 = m + s
            let c2 = is_target_werewolf.clone() + s;

            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalCiphertextVar::new(c1, c2)
        };

        // compare
        let enc_result_var2 =
            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalCiphertextVar::new_input(
                ark_relations::ns!(cs, "gadget_commitment"),
                || {
                    let sum_target = (0..self.private_input.len())
                        .map(|i| {
                            let mut tmp = MpcField::<Fr>::from(0u32);
                            for j in 0..self.private_input.len() {
                                tmp += self.private_input[j].is_target[i];
                            }
                            tmp
                        })
                        .collect::<Vec<_>>();
                    let is_werewolf: MpcField<Fr> = self
                        .private_input
                        .iter()
                        .map(|input| input.is_werewolf)
                        .zip(sum_target.iter())
                        .map(|(x, y)| x * y)
                        .sum();

                    let message = match is_werewolf.is_one() {
                    true => {
                        <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::prime_subgroup_generator()
                    }
                    false => <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::default(),
                };
                    let enc_result =
                        <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalScheme::encrypt(
                            &self.public_input.elgamal_param,
                            &self.public_input.pub_key,
                            &message,
                            &self.private_input[0].randomness,
                        )
                        .unwrap();
                    Ok(enc_result)
                },
            )?;

        enc_result_var.enforce_equal(&enc_result_var2)?;

        // TODO: Add verify commitments
        // self.verify_commitments(cs.clone())?;

        println!(
            "[DivinationCircuit(MPC)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[DivinationCircuit(MPC)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[DivinationCircuit(MPC)] total number of constraints: {}",
            cs.num_constraints()
        );

        Ok(())
    }
}

impl ConstraintSynthesizer<Fr> for RoleAssignmentCircuit<Fr> {
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<Fr>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // TODO: Implement constraint generation logic here
        // check player commitment
        // assert_eq!(
        //     self.public_input.num_players,
        //     self.private_input.player_randomness.len()
        // );
        // assert_eq!(
        //     self.public_input.num_players,
        //     self.public_input.player_commitment.len()
        // );

        // TODO: check player commitment
        // for i in 0..self.public_input.num_players {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        // initialize
        let tau_matrix_var = na::DMatrix::from_iterator(
            self.public_input.tau_matrix.nrows(),
            self.public_input.tau_matrix.ncols(),
            self.public_input.tau_matrix.iter().map(|b| {
                FpVar::new_input(cs.clone(), || Ok(b))
                    .expect("tau matrix var is not allocated correctly")
            }),
        );

        let shuffle_matrix_var = self
            .private_input
            .iter()
            .map(|input| {
                na::DMatrix::from_iterator(
                    input.shuffle_matrices.nrows(),
                    input.shuffle_matrices.ncols(),
                    input.shuffle_matrices.iter().map(|b| {
                        FpVar::new_witness(cs.clone(), || Ok(b))
                            .expect("shuffle matrix var is not allocated correctly")
                    }),
                )
            })
            .collect::<Vec<_>>();

        let inverse_shuffle_matrix_var = self
            .private_input
            .iter()
            .rev()
            .map(|input| {
                na::DMatrix::from_iterator(
                    input.shuffle_matrices.nrows(),
                    input.shuffle_matrices.ncols(),
                    input.shuffle_matrices.transpose().iter().map(|b| {
                        FpVar::new_witness(cs.clone(), || Ok(b))
                            .expect("shuffle matrix var is not allocated correctly")
                    }),
                )
            })
            .collect::<Vec<_>>();

        // each shuffle matrix is a permutation matrix and sub matrix is a identity matrix
        // Warning: This should be enforced, but we skip it to reduce the constraint count.
        // shuffle_matrix_var.iter().for_each(|matrix| {
        //     enforce_permutation_matrix(matrix, self.public_input.num_players).unwrap()
        // });

        // calculate
        // M = Product of shuffle_matrix
        let matrix_M_var = shuffle_matrix_var
            .clone()
            .iter()
            .skip(1)
            .fold(shuffle_matrix_var[0].clone(), |acc, x| acc * x);

        let inverse_matrix_M_var = inverse_shuffle_matrix_var
            .clone()
            .iter()
            .skip(1)
            .fold(inverse_shuffle_matrix_var[0].clone(), |acc, x| acc * x);

        // rho = M^-1 * tau * M
        let rho_var = inverse_matrix_M_var * &tau_matrix_var * &matrix_M_var;

        let mut rho_sequence_var = Vec::with_capacity(self.public_input.num_players);
        let mut current_rho = rho_var.clone();
        for _ in 0..self.public_input.num_players {
            rho_sequence_var.push(current_rho.clone());
            current_rho *= rho_var.clone(); // rho^(i+1) = rho^i * rho
        }

        // input_result is consistent with the calculated result
        let length = self.public_input.tau_matrix.nrows();

        // 1. gen one-hot vector
        let unit_vecs = (0..self.public_input.num_players)
            .map(|i| test_one_hot_vector(length, i, cs.clone()))
            .collect::<Vec<_>>();

        // 2. calculate rho^i * unit_vec_j to value
        let calced_vec = unit_vecs
            .iter()
            .map(|unit_vec_j| {
                rho_sequence_var
                    .iter()
                    .map(|rho| {
                        let res_index = rho * unit_vec_j.clone();
                        // Warning: This should be enforced, but we skip it to reduce the constraint count.
                        test_index_to_value(res_index, false).unwrap()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Warning: This should be enforced, but we skip it to reduce the constraint count.
        let calced_role = calced_vec
            .iter()
            .map(|val| test_max(val, self.public_input.num_players, false).unwrap())
            .collect::<Vec<_>>();

        // // commitment
        // for i in 0..self.num_players {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.public_input.pedersen_param.clone()),
        //         input: calced_role[i].value().unwrap_or_default(),
        //         open: self.private_input[i].randomness.clone(),
        //         commit: self.public_input.role_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        println!(
            "[RoleAssignmentCircuit(Local)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[RoleAssignmentCircuit(Local)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[RoleAssignmentCircuit(Local)] total number of constraints: {}",
            cs.num_constraints()
        );

        Ok(())
    }
}

impl ConstraintSynthesizer<MpcField<Fr>> for RoleAssignmentCircuit<MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<MpcField<Fr>>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // TODO: Implement constraint generation logic here
        // check player commitment
        // assert_eq!(
        //     self.public_input.num_players,
        //     self.private_input.player_randomness.len()
        // );
        // assert_eq!(
        //     self.public_input.num_players,
        //     self.public_input.player_commitment.len()
        // );

        // TODO: check player commitment
        // for i in 0..self.public_input.num_players {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.pedersen_param.clone()),
        //         input: self.player_randomness[i],
        //         open: <Fr as LocalOrMPC<Fr>>::PedersenRandomness::default(),
        //         commit: self.player_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        // initialize
        let tau_matrix_var = na::DMatrix::from_iterator(
            self.public_input.tau_matrix.nrows(),
            self.public_input.tau_matrix.ncols(),
            self.public_input.tau_matrix.iter().map(|b| {
                MpcFpVar::new_input(cs.clone(), || Ok(b))
                    .expect("tau matrix var is not allocated correctly")
            }),
        );

        let shuffle_matrix_var = self
            .private_input
            .iter()
            .map(|input| {
                na::DMatrix::from_iterator(
                    input.shuffle_matrices.nrows(),
                    input.shuffle_matrices.ncols(),
                    input.shuffle_matrices.iter().map(|b| {
                        MpcFpVar::new_witness(cs.clone(), || Ok(b))
                            .expect("shuffle matrix var is not allocated correctly")
                    }),
                )
            })
            .collect::<Vec<_>>();

        let inverse_shuffle_matrix_var = self
            .private_input
            .iter()
            .rev()
            .map(|input| {
                na::DMatrix::from_iterator(
                    input.shuffle_matrices.nrows(),
                    input.shuffle_matrices.ncols(),
                    input.shuffle_matrices.transpose().iter().map(|b| {
                        MpcFpVar::new_witness(cs.clone(), || Ok(b))
                            .expect("shuffle matrix var is not allocated correctly")
                    }),
                )
            })
            .collect::<Vec<_>>();

        // each shuffle matrix is a permutation matrix and sub matrix is a identity matrix
        // Warning: This should be enforced, but we skip it to reduce the constraint count.
        // shuffle_matrix_var.iter().for_each(|matrix| {
        //     enforce_permutation_matrix_mpc(matrix, self.public_input.num_players).unwrap()
        // });

        // calculate
        // M = Product of shuffle_matrix
        let matrix_M_var = shuffle_matrix_var
            .clone()
            .iter()
            .skip(1)
            .fold(shuffle_matrix_var[0].clone(), |acc, x| acc * x);

        let inverse_matrix_M_var = inverse_shuffle_matrix_var
            .clone()
            .iter()
            .skip(1)
            .fold(inverse_shuffle_matrix_var[0].clone(), |acc, x| acc * x);

        // rho = M^-1 * tau * M
        let rho_var = inverse_matrix_M_var * &tau_matrix_var * &matrix_M_var;

        let mut rho_sequence_var = Vec::with_capacity(self.public_input.num_players);
        let mut current_rho = rho_var.clone();
        for _ in 0..self.public_input.num_players {
            rho_sequence_var.push(current_rho.clone());
            current_rho *= rho_var.clone(); // rho^(i+1) = rho^i * rho
        }

        // input_result is consistent with the calculated result
        let length = self.public_input.tau_matrix.nrows();

        // 1. gen one-hot vector
        let unit_vecs = (0..self.public_input.num_players)
            .map(|i| test_one_hot_vector_mpc(length, i, cs.clone()))
            .collect::<Vec<_>>();

        // 2. calculate rho^i * unit_vec_j to value
        let calced_vec = unit_vecs
            .iter()
            .map(|unit_vec_j| {
                rho_sequence_var
                    .iter()
                    .map(|rho| {
                        let res_index = rho * unit_vec_j.clone();
                        // Warning: This should be enforced, but we skip it to reduce the constraint count.
                        test_index_to_value_mpc(res_index, false).unwrap()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Warning: This should be enforced, but we skip it to reduce the constraint count.
        let calced_role = calced_vec
            .iter()
            .map(|val| test_max_mpc(val, self.public_input.num_players, false).unwrap())
            .collect::<Vec<_>>();

        // // commitment
        // for i in 0..self.num_players {
        //     let pedersen_circuit = PedersenComCircuit {
        //         param: Some(self.public_input.pedersen_param.clone()),
        //         input: calced_role[i].value().unwrap_or_default(),
        //         open: self.private_input[i].randomness.clone(),
        //         commit: self.public_input.role_commitment[i],
        //     };
        //     pedersen_circuit.generate_constraints(cs.clone())?;
        // }

        println!(
            "[RoleAssignmentCircuit(MPC)] instance vars: {}",
            cs.num_instance_variables()
        );
        println!(
            "[RoleAssignmentCircuit(MPC)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[RoleAssignmentCircuit(MPC)] total number of constraints: {}",
            cs.num_constraints()
        );

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

        // game_state
        if werewolf_count == 0 {
            Fr::from(2u32)
        } else if werewolf_count >= villagers_count {
            Fr::from(1u32)
        } else {
            Fr::from(3u32)
        }
    }
}

impl WinningJudgementCircuit<MpcField<Fr>> {
    pub fn calculate_output(&self) -> MpcField<Fr> {
        let alive_player_num = self.private_input.len();

        let werewolf_count = self
            .private_input
            .iter()
            .fold(MpcField::<Fr>::zero(), |acc, input| acc + input.am_werewolf);

        let villagers_count = MpcField::<Fr>::from(alive_player_num as u32) - werewolf_count;

        let no_werewolf = werewolf_count.sync_is_zero_shared();

        // game_state
        no_werewolf.field() * MpcField::<Fr>::from(2_u32)
            + (!no_werewolf).field()
                * ((werewolf_count + MpcField::<Fr>::one())
                    .sync_is_smaller_than(&villagers_count)
                    .field()
                    * MpcField::<Fr>::from(3_u32)
                    + (MpcField::<Fr>::one()
                        - ((werewolf_count + MpcField::<Fr>::one())
                            .sync_is_smaller_than(&villagers_count))
                        .field())
                        * MpcField::<Fr>::from(1_u32))
    }
}

impl DivinationCircuit<Fr> {
    pub fn calculate_output(&self) -> Fr {
        todo!()
    }
}

impl DivinationCircuit<MpcField<Fr>> {
    pub fn calculate_output(
        &self,
    ) -> <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalCiphertext {
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

        let mut sum = MpcField::<Fr>::default();

        for i in 0..self.private_input.len() {
            let mut tmp = MpcField::<Fr>::default();
            for j in 0..self.private_input.len() {
                tmp += self.private_input[j].is_target[i];
            }
            sum += tmp * self.private_input[i].is_werewolf;
        }

        let pub_key = self.public_input.pub_key;

        let base = <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::prime_subgroup_generator();

        // TODO: implement correctly. (without reveal)
        let message = if sum.sync_reveal().is_one() {
            base
        } else {
            <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalPlaintext::default()
        };

        let ciphertext = <MpcField<Fr> as ElGamalLocalOrMPC<MpcField<Fr>>>::ElGamalScheme::encrypt(
            &self.public_input.elgamal_param,
            &pub_key,
            &message,
            &self.private_input[0].randomness,
        )
        .unwrap();
        ciphertext
    }
}

impl KeyPublicizeCircuit<Fr> {
    fn calculate_output(&self) -> (Fr, Fr) {
        let pub_key_x = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_x)
            .fold(Fr::zero(), |acc, x| acc + x);

        let pub_key_y = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_y)
            .fold(Fr::zero(), |acc, y| acc + y);

        (pub_key_x, pub_key_y)
    }
}

impl KeyPublicizeCircuit<MpcField<Fr>> {
    pub fn calculate_output(&self) -> (MpcField<Fr>, MpcField<Fr>) {
        let pub_key_x = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_x)
            .fold(MpcField::<Fr>::zero(), |acc, x| acc + x);

        let pub_key_y = self
            .private_input
            .iter()
            .map(|input| input.pub_key_or_dummy_y)
            .fold(MpcField::<Fr>::zero(), |acc, y| acc + y);

        (pub_key_x, pub_key_y)
    }
}

impl RoleAssignmentCircuit<MpcField<Fr>> {
    // Vec<MpcField<Fr>> は各プレイヤーの役職IDを表す。0が村人、1が占い師、2が人狼など。
    pub fn calculate_output(&self) -> Vec<MpcField<Fr>> {
        let num_players = self.private_input.len();

        let grouping_parameter = &self.public_input.grouping_parameter;

        let shuffle_matrix = self
            .private_input
            .iter()
            .map(|input| input.shuffle_matrices.clone())
            .collect::<Vec<_>>();

        let revealed_shuffle_matrix = shuffle_matrix
            .iter()
            .map(|row| row.map(|x| x.sync_reveal()))
            .collect::<Vec<_>>();

        let mut output_vec = Vec::new();

        for id in 0..num_players {
            let (role, role_val, player_ids) =
                calc_shuffle_matrix(grouping_parameter, &revealed_shuffle_matrix, id).unwrap();

            match role {
                Role::Villager => {
                    output_vec.push(MpcField::<Fr>::from(0u32));
                }
                Role::FortuneTeller => {
                    output_vec.push(MpcField::<Fr>::from(1u32));
                }
                Role::Werewolf => {
                    output_vec.push(MpcField::<Fr>::from(2u32));
                }
            }
        }

        output_vec
    }
}

impl ConstraintSynthesizer<Fr> for WinningJudgementCircuit<Fr> {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> ark_relations::r1cs::Result<()> {
        // TODO: check player commitment
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
            .map(|input| FpVar::new_witness(cs.clone(), || Ok(input.am_werewolf)))
            .collect::<Result<Vec<_>, _>>()?;

        let game_state_var = FpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;

        // calculate
        let num_werewolf_var =
            am_werewolf_var
                .iter()
                .fold(<FpVar<Fr> as Zero>::zero(), |mut acc, x| {
                    acc += x;
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

        // // TODO: check commitment
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

        println!(
            "[WinningJudgementCircuit(Local)] instance vars: {}",
            cs.num_instance_variables()
        ); // 1(定数) + 公開入力の数
        println!(
            "[WinningJudgementCircuit(Local)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[WinningJudgementCircuit(Local)] total number of constraints: {}",
            cs.num_constraints()
        );
        Ok(())
    }
}

impl ConstraintSynthesizer<MpcField<Fr>> for WinningJudgementCircuit<MpcField<Fr>> {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<MpcField<Fr>>,
    ) -> ark_relations::r1cs::Result<()> {
        // TODO: check player commitment
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
            Ok(MpcField::<Fr>::from(alive_player_num as u32))
        })?;

        let am_werewolf_var = self
            .private_input
            .iter()
            .map(|input| MpcFpVar::new_witness(cs.clone(), || Ok(input.am_werewolf)))
            .collect::<Result<Vec<_>, _>>()?;

        let game_state_var = MpcFpVar::new_input(cs.clone(), || Ok(self.calculate_output()))?;
        // let game_state_var =
        //     MpcFpVar::new_input(cs.clone(), || Ok(MpcField::<Fr>::from(0_u32)))?;

        // calculate
        let num_werewolf_var =
            am_werewolf_var
                .iter()
                .fold(<MpcFpVar<MpcField<Fr>> as Zero>::zero(), |mut acc, x| {
                    acc += x;
                    acc
                });

        let num_citizen_var = num_alive_var - &num_werewolf_var;

        let calced_game_state_var = MpcFpVar::conditionally_select(
            &MpcFieldVar::is_zero(&num_werewolf_var)?,
            &MpcFpVar::constant(MpcField::<Fr>::from(2_u32)), // villager win
            &MpcFpVar::conditionally_select(
                &num_werewolf_var.is_cmp(&num_citizen_var, std::cmp::Ordering::Less, false)?,
                &MpcFpVar::constant(MpcField::<Fr>::from(3_u32)), // game continues
                &MpcFpVar::constant(MpcField::<Fr>::from(1_u32)), // werewolf win
            )?,
        )?;

        // // TODO: check commitment
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

        println!(
            "[WinningJudgementCircuit(MPC)] instance vars: {}",
            cs.num_instance_variables()
        ); // 1(定数) + 公開入力の数
        println!(
            "[WinningJudgementCircuit(MPC)] witness vars: {}",
            cs.num_witness_variables()
        );
        println!(
            "[WinningJudgementCircuit(MPC)] total number of constraints: {}",
            cs.num_constraints()
        );

        Ok(())
    }
}

// return maximum value in the vector a, index runs from 0 to use_index_len
fn test_max<F: PrimeField>(
    a: &[FpVar<F>],
    use_index_len: usize,
    should_enforce: bool,
) -> Result<FpVar<F>, SynthesisError> {
    let cs = a[0].cs().clone();
    let max_var = FpVar::new_witness(cs, || {
        let max = a.iter().map(|x| x.value().unwrap()).max().unwrap();
        Ok(max)
    })?;

    if should_enforce {
        // each element must be less than half of the modulus
        for i in 0..use_index_len {
            a[i].enforce_cmp(&max_var, core::cmp::Ordering::Less, true)?;
        }
    }

    Ok(max_var)
}

fn test_max_mpc<F: PrimeField + SquareRootField + BitDecomposition + EqualityZero>(
    a: &[MpcFpVar<F>],
    use_index_len: usize,
    should_enforce: bool,
) -> Result<MpcFpVar<F>, SynthesisError> {
    let cs = a[0].cs().clone();
    let max_var = MpcFpVar::new_witness(cs, || {
        let max = a.iter().map(|x| x.value().unwrap()).max().unwrap();
        Ok(max)
    })?;

    if should_enforce {
        for i in 0..use_index_len {
            a[i].enforce_cmp(&max_var, core::cmp::Ordering::Less, true)?;
        }
    }

    Ok(max_var)
}

fn test_index_to_value<F: PrimeField>(
    a: na::DVector<FpVar<F>>,
    should_enforce: bool,
) -> Result<FpVar<F>, SynthesisError> {
    let cs = a[0].cs().clone();
    let value_var = FpVar::new_witness(cs.clone(), || {
        let res = a
            .iter()
            .position(|x| x.value().unwrap().is_one())
            .expect("This index vector is not a one-hot vector");
        Ok(F::from(res as u64))
    })?;

    if should_enforce {
        let stair_vector = na::DVector::from(
            (0..a.len())
                .map(|i| FpVar::new_constant(cs.clone(), F::from(i as u64)).unwrap())
                .collect::<Vec<_>>(),
        );
        let ip = a.dot(&stair_vector);

        ip.enforce_equal(&value_var)?;
    }
    Ok(value_var)
}

fn test_index_to_value_mpc<F: PrimeField + Reveal>(
    a: na::DVector<MpcFpVar<F>>,
    should_enforce: bool,
) -> Result<MpcFpVar<F>, SynthesisError>
where
    <F as Reveal>::Base: Zero,
    <F as Reveal>::Base: One,
    <F as Reveal>::Base: PartialEq,
{
    let cs = a[0].cs().clone();
    let value_var = MpcFpVar::new_witness(cs.clone(), || {
        let res = a
            .iter()
            .position(|x| {
                let value = x.value().unwrap().sync_reveal();
                value.is_one()
            })
            .expect("This index vector is not a one-hot vector");
        Ok(F::from(res as u64))
    })?;

    if should_enforce {
        let stair_vector = na::DVector::from(
            (0..a.len())
                .map(|i| MpcFpVar::new_constant(cs.clone(), F::from(i as u64)).unwrap())
                .collect::<Vec<_>>(),
        );
        let ip = a.dot(&stair_vector);

        ip.enforce_equal(&value_var)?;
    }
    Ok(value_var)
}

fn test_one_hot_vector<F: PrimeField>(
    length: usize,
    index: usize,
    cs: ConstraintSystemRef<F>,
) -> na::DVector<FpVar<F>> {
    assert!(index < length);
    let mut res = na::DVector::<FpVar<F>>::zeros(length);
    for i in 0..length {
        if i == index {
            res[i] = FpVar::new_constant(cs.clone(), F::one()).unwrap();
        } else {
            res[i] = FpVar::new_constant(cs.clone(), F::zero()).unwrap();
        }
    }
    res
}

fn test_one_hot_vector_mpc<F: PrimeField + Reveal>(
    length: usize,
    index: usize,
    cs: ConstraintSystemRef<F>,
) -> na::DVector<MpcFpVar<F>>
where
    <F as Reveal>::Base: Zero,
{
    assert!(index < length);
    let mut res = na::DVector::<MpcFpVar<F>>::zeros(length);
    for i in 0..length {
        if i == index {
            res[i] = MpcFpVar::new_constant(cs.clone(), F::one()).unwrap();
        } else {
            res[i] = MpcFpVar::new_constant(cs.clone(), F::zero()).unwrap();
        }
    }
    res
}

fn enforce_permutation_matrix<F: PrimeField>(
    matrix: &na::DMatrix<FpVar<F>>,
    n: usize,
) -> Result<(), SynthesisError> {
    let size = matrix.nrows();
    // (0,0) ~ (n-1,n-1) is arbitrary permutation matrix

    for i in 0..n {
        let mut i_th_row_sum = <FpVar<F> as Zero>::zero();
        let mut i_th_column_sum = <FpVar<F> as Zero>::zero();

        for j in 0..n {
            // all check 0 or 1 -> row sum and column sum is 1
            let val = &matrix[(i, j)];

            val.is_eq(&<FpVar<F> as Zero>::zero())
                .unwrap()
                .or(&val.is_eq(&<FpVar<F> as One>::one()).unwrap())
                .unwrap()
                .enforce_equal(&Boolean::TRUE)?;

            // row column is ambiguos
            i_th_row_sum += val;
            i_th_column_sum += &matrix[(j, i)];
        }

        i_th_row_sum.enforce_equal(&<FpVar<F> as One>::one())?;
        i_th_column_sum.enforce_equal(&<FpVar<F> as One>::one())?;
    }

    for i in 0..size {
        for j in 0..size {
            if i >= n || j >= n {
                // (n~n+m-1, n~n+m-1) is identity matrix
                if i == j {
                    let val = &matrix[(i, j)];
                    val.enforce_equal(&<FpVar<F> as One>::one())?;
                } else {
                    // other is 0
                    let val = &matrix[(i, j)];
                    val.enforce_equal(&<FpVar<F> as Zero>::zero())?;
                }
            }
        }
    }

    Ok(())
}

fn enforce_permutation_matrix_mpc<
    F: PrimeField + Reveal + ark_ff::SquareRootField + mpc_algebra::EqualityZero,
>(
    matrix: &na::DMatrix<MpcFpVar<F>>,
    n: usize,
) -> Result<(), SynthesisError>
where
    <F as Reveal>::Base: Zero,
{
    let size = matrix.nrows();
    // (0,0) ~ (n-1,n-1) is arbitrary permutation matrix

    for i in 0..n {
        let mut i_th_row_sum = <MpcFpVar<F> as Zero>::zero();
        let mut i_th_column_sum = <MpcFpVar<F> as Zero>::zero();

        for j in 0..n {
            // all check 0 or 1 -> row sum and column sum is 1
            let val = &matrix[(i, j)];

            val.is_zero()
                .unwrap()
                .or(&(val - <MpcFpVar<F> as One>::one()).is_zero().unwrap())
                .unwrap()
                .enforce_equal(&MpcBoolean::TRUE)?;

            // row column is ambiguos
            i_th_row_sum += val;
            i_th_column_sum += &matrix[(j, i)];
        }

        i_th_row_sum.enforce_equal(&<MpcFpVar<F> as One>::one())?;
        i_th_column_sum.enforce_equal(&<MpcFpVar<F> as One>::one())?;
    }

    for i in 0..size {
        for j in 0..size {
            if i >= n || j >= n {
                // (n~n+m-1, n~n+m-1) is identity matrix
                if i == j {
                    let val = &matrix[(i, j)];
                    val.enforce_equal(&<MpcFpVar<F> as One>::one())?;
                } else {
                    // other is 0
                    let val = &matrix[(i, j)];
                    val.enforce_equal(&<MpcFpVar<F> as Zero>::zero())?;
                }
            }
        }
    }

    Ok(())
}
