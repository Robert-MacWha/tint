//! Various conversion utilities for working with [`ark_bn254::Fr`] and
//! other types.

use alloy_primitives::{Address, B256};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};

pub fn fr_to_u128(fr: &Fr) -> u128 {
    let bytes = fr.into_bigint().to_bytes_le();
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes[..16]);
    u128::from_le_bytes(arr)
}

pub fn fr_to_b256(fr: Fr) -> B256 {
    B256::from_slice(&fr.into_bigint().to_bytes_be())
}

pub fn b256_to_fr(bytes: B256) -> Fr {
    Fr::from_be_bytes_mod_order(bytes.as_slice())
}

pub fn fr_to_address(fr: Fr) -> Address {
    let word = fr_to_b256(fr);
    Address::from_word(word)
}

pub fn address_to_fr(address: Address) -> Fr {
    b256_to_fr(address.into_word())
}

#[cfg(test)]
mod tests {
    use std::u128;

    use super::*;

    #[test]
    fn test_fr_to_u128() {
        let fr = Fr::from(123456789u128);
        let u128_value = fr_to_u128(&fr);
        assert_eq!(u128_value, 123456789u128);
    }

    #[test]
    fn test_fr_to_u128_overflow() {
        let fr = Fr::from(u128::MAX) + Fr::from(2);
        let u128_value = fr_to_u128(&fr);
        assert_eq!(u128_value, 1);
    }

    #[test]
    fn test_b256_to_fr() {
        let bytes = B256::from_slice(&[1u8; 32]);
        let fr_value = b256_to_fr(bytes);
        let bytes_back = fr_to_b256(fr_value);
        assert_eq!(bytes, bytes_back);
    }

    #[test]
    fn test_address_to_fr() {
        let address = Address::new([3; 20]);
        let fr_value = address_to_fr(address);
        let address_value = fr_to_address(fr_value);
        assert_eq!(address, address_value);
    }

    #[test]
    fn test_address_to_fr_does_not_overflow() {
        let address = Address::new([0xff; 20]);
        let fr_value = address_to_fr(address);
        let address_value = fr_to_address(fr_value);
        assert_eq!(address, address_value);
    }
}
