use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::marker::PhantomData;

use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::gates::gate::Gate;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::SPONGE_WIDTH;
use crate::poseidon2_hash as poseidon2;
use crate::poseidon2_hash::Poseidon2;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluates a full Poseidon2 permutation with 12 state elements.
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests.
#[derive(Debug, Default)]
pub struct Poseidon2Gate<F: RichField + Extendable<D>, const D: usize>(PhantomData<F>);

implement_poseidon2_gate!(Poseidon2Gate, Poseidon2, Poseidon2Generator, SPONGE_WIDTH);

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use super::Poseidon2Gate;
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use crate::poseidon2_hash::Poseidon2;
    use plonky2::iop::target::Target;
    use plonky2::iop::wire::Wire;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::GenericConfig;
    use crate::poseidon2_goldilock::Poseidon2GoldilocksConfig;

    #[test]
    fn wire_indices() {
        type F = GoldilocksField;
        type Gate = Poseidon2Gate<F, 4>;

        assert_eq!(Gate::wire_input(0), 0);
        assert_eq!(Gate::wire_input(11), 11);
        assert_eq!(Gate::wire_output(0), 12);
        assert_eq!(Gate::wire_output(11), 23);
        assert_eq!(Gate::WIRE_SWAP, 24);
        assert_eq!(Gate::wire_delta(0), 25);
        assert_eq!(Gate::wire_delta(3), 28);
        assert_eq!(Gate::wire_full_sbox_0(1, 0), 29);
        assert_eq!(Gate::wire_full_sbox_0(3, 0), 53);
        assert_eq!(Gate::wire_full_sbox_0(3, 11), 64);
        assert_eq!(Gate::wire_partial_sbox(0), 65);
        assert_eq!(Gate::wire_partial_sbox(21), 86);
        assert_eq!(Gate::wire_full_sbox_1(0, 0), 87);
        assert_eq!(Gate::wire_full_sbox_1(3, 0), 123);
        assert_eq!(Gate::wire_full_sbox_1(3, 11), 134);
    }

    #[test]
    fn generated_output() {
        const D: usize = 2;
        type C = Poseidon2GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::new(config);
        type Gate = Poseidon2Gate<F, D>;
        let gate = Gate::new();
        let row = builder.add_gate(gate, vec![]);
        for i in 0..SPONGE_WIDTH {
            builder.register_public_input(Target::wire(row, Gate::wire_output(i)));
        }
        let circuit = builder.build_prover::<C>();

        let permutation_inputs = (0..SPONGE_WIDTH)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();

        let mut inputs = PartialWitness::new();
        inputs.set_wire(
            Wire {
                row,
                column: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..SPONGE_WIDTH {
            inputs.set_wire(
                Wire {
                    row,
                    column: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let proof = circuit.prove(inputs).unwrap();

        let expected_outputs: [F; SPONGE_WIDTH] =
            F::poseidon2(permutation_inputs.try_into().unwrap());
        expected_outputs.iter().zip(proof.public_inputs.iter())
            .for_each(|(expected_out, out)|
                assert_eq!(expected_out, out)
            );
    }

    #[test]
    fn low_degree() {
        type F = GoldilocksField;
        let gate = Poseidon2Gate::<F, 4>::new();
        test_low_degree(gate)
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = Poseidon2GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = Poseidon2Gate::<F, 2>::new();
        test_eval_fns::<F, C, _, D>(gate)
    }
}
