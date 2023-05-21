//! Implementations for Poseidon2 over Goldilocks field of widths 12.

use plonky2::field::extension::quadratic::QuadraticExtension;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hashing::HashConfig;
use plonky2::plonk::config::GenericConfig;

use crate::poseidon2_hash::{Poseidon2, Poseidon2Hash, Poseidon2HashConfig};

#[rustfmt::skip]
impl Poseidon2 for GoldilocksField {
    // We only need INTERNAL_MATRIX_DIAG_M_1 here, specifying the diagonal - 1 of the internal matrix

    const INTERNAL_MATRIX_DIAG_M_1: [u64; Poseidon2HashConfig::WIDTH]  = [
        0xcf6f77ac16722af9, 0x3fd4c0d74672aebc, 0x9b72bf1c1c3d08a8, 0xe4940f84b71e4ac2,
        0x61b27b077118bc72, 0x2efd8379b8e661e2, 0x858edcf353df0341, 0x2d9c20affb5c4516,
        0x5120143f0695defb, 0x62fc898ae34a5c5b, 0xa3d9560c99123ed2, 0x98fd739d8e7fc933,
    ];

    #[cfg(all(target_arch="aarch64", target_feature="neon"))]
    #[inline(always)]
    fn sbox_layer(state: &mut [Self; 12]) {
         unsafe {
             plonky2::hash::arch::aarch64::poseidon_goldilocks_neon::sbox_layer(state);
         }
    }
}

/// Configuration using Poseidon2 over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Poseidon2GoldilocksConfig;
impl GenericConfig<2> for Poseidon2GoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type HCO = Poseidon2HashConfig;
    type HCI = Poseidon2HashConfig;
    type Hasher = Poseidon2Hash;
    type InnerHasher = Poseidon2Hash;
}

#[cfg(test)]
mod tests {
    use plonky2::field::extension::Extendable;
    use plonky2::field::goldilocks_field::GoldilocksField as F;
    use plonky2::hash::hash_types::RichField;
    use plonky2::hash::hashing::HashConfig;
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{
        AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig,
    };
    use rstest::rstest;
    use serial_test::serial;

    use crate::poseidon2_goldilock::Poseidon2GoldilocksConfig;
    use crate::poseidon2_hash::test_helpers::{
        check_test_vectors, prove_circuit_with_poseidon_hash, recursive_proof,
    };
    use crate::poseidon2_hash::{Poseidon2, Poseidon2Hash};

    const D: usize = 2;

    #[test]
    fn test_vectors() {
        // Test inputs are:
        // 1. range 0..WIDTH

        #[rustfmt::skip]
            let test_vectors12: Vec<([u64; 12], [u64; 12])> = vec![
            ([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, ],
             [0xed3dbcc4ff1e8d33, 0xfb85eac6ac91a150, 0xd41e1e237ed3e2ef, 0x5e289bf0a4c11897,
                 0x4398b20f93e3ba6b, 0x5659a48ffaf2901d, 0xe44d81e89a88f8ae, 0x08efdb285f8c3dbc,
                 0x294ab7503297850e, 0xa11c61f4870b9904, 0xa6855c112cc08968, 0x17c6d53d2fb3e8c1, ]),
        ];

        check_test_vectors::<F>(test_vectors12);
    }

    #[test]
    fn test_circuit_with_poseidon2() {
        let (cd, proof) =
            prove_circuit_with_poseidon_hash::<_, Poseidon2GoldilocksConfig, D, _, _>(
                CircuitConfig::standard_recursion_config(),
                1024,
                Poseidon2Hash,
                false,
            )
            .unwrap();

        cd.verify(proof).unwrap();
    }

    #[ignore]
    #[rstest]
    #[case::poseidon(PoseidonGoldilocksConfig{})]
    #[case::poseidon2(Poseidon2GoldilocksConfig{})]
    #[serial]
    fn compare_proof_generation_with_poseidon<
        F: RichField + Extendable<D> + Poseidon2,
        const D: usize,
        C: GenericConfig<D, F = F>,
    >(
        #[case] _c: C,
    ) where
        [(); C::HCO::WIDTH]:,
        [(); C::HCI::WIDTH]:,
    {
        let (cd, proof) = prove_circuit_with_poseidon_hash::<_, C, D, _, _>(
            CircuitConfig::standard_recursion_config(),
            4096,
            Poseidon2Hash,
            true,
        )
        .unwrap();

        cd.verify(proof).unwrap();
    }

    #[ignore]
    #[rstest]
    #[case::poseidon(PoseidonGoldilocksConfig, PoseidonHash{})]
    #[case::poseidon2(PoseidonGoldilocksConfig, Poseidon2Hash{})]
    #[serial]
    fn compare_circuits_with_poseidon<
        F: RichField + Extendable<D> + Poseidon2,
        const D: usize,
        C: GenericConfig<D, F = F>,
        HC: HashConfig,
        H: Hasher<F, HC> + AlgebraicHasher<F, HC>,
    >(
        #[case] _conf: C,
        #[case] hasher: H,
    ) where
        [(); HC::WIDTH]:,
        [(); C::HCO::WIDTH]:,
        [(); C::HCI::WIDTH]:,
    {
        let (cd, proof) = prove_circuit_with_poseidon_hash::<_, C, D, _, _>(
            CircuitConfig::standard_recursion_config(),
            4096,
            hasher,
            true,
        )
        .unwrap();

        cd.verify(proof).unwrap();
    }

    #[rstest]
    #[serial]
    fn test_recursive_circuit_with_poseidon2<
        F: RichField + Poseidon2 + Extendable<D>,
        C: GenericConfig<D, F = F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        #[values(PoseidonGoldilocksConfig{}, Poseidon2GoldilocksConfig{})] _c: C,
        #[values(PoseidonGoldilocksConfig{}, Poseidon2GoldilocksConfig{})] _inner: InnerC,
    ) where
        InnerC::Hasher: AlgebraicHasher<F, InnerC::HCO>,
        [(); C::HCO::WIDTH]:,
        [(); C::HCI::WIDTH]:,
        [(); InnerC::HCO::WIDTH]:,
        [(); InnerC::HCI::WIDTH]:,
    {
        let config = CircuitConfig::standard_recursion_config();

        let (cd, proof) = prove_circuit_with_poseidon_hash::<F, InnerC, D, _, _>(
            config,
            1024,
            Poseidon2Hash {},
            false,
        )
        .unwrap();

        println!("base proof generated");

        let (rec_cd, rec_proof) =
            recursive_proof::<F, C, InnerC, D>(proof, &cd, &cd.common.config).unwrap();

        println!("recursive proof generated");

        rec_cd.verify(rec_proof).unwrap();

        assert_eq!(rec_cd.common.degree_bits(), 12);
    }
}
