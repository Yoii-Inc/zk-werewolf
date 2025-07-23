use crate::*;

use ark_bls12_377::Fr;
use ark_ff::PrimeField;
use ark_ff::Zero;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::select::CondSelectGadget;
use ark_r1cs_std::ToBitsGadget;
use ark_relations::r1cs::ConstraintSystemRef;
use ark_relations::{lc, r1cs::ConstraintSynthesizer};
use ark_std::test_rng;
use mpc_algebra::malicious_majority as mm;
use mpc_algebra::mpc_fields::MpcFieldVar;
use mpc_algebra::BooleanWire;
use mpc_algebra::LessThan;
use mpc_algebra::MpcCondSelectGadget;
use mpc_algebra::MpcEqGadget;
use mpc_algebra::MpcFpVar;
use mpc_algebra::MpcToBitsGadget;
use mpc_algebra::Reveal;
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
        let player_num = self.private_input.len(); // Assuming all players join generation
        let alive_player_num = player_num; // Assuming all players are alive for simplicity

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
        let player_num = self.private_input.len(); // Assuming all players join generation
        let alive_player_num = player_num; // Assuming all players are alive for simplicity

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

impl<F: PrimeField + LocalOrMPC<F> + ElGamalLocalOrMPC<F>> ConstraintSynthesizer<F>
    for WinningJudgementCircuit<F>
{
    fn generate_constraints(
        self,
        cs: ark_relations::r1cs::ConstraintSystemRef<F>,
    ) -> Result<(), ark_relations::r1cs::SynthesisError> {
        // Implement constraint generation logic here
        Ok(())
    }
}
