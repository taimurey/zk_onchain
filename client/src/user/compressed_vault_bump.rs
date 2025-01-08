use light_sdk::merkle_context::AddressMerkleContext;

use light_utils::{hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be};
use solana_sdk::pubkey::Pubkey;
use zk_onchain::state::USER_VAULT;

pub fn derive_user_vault_with_bump(
    authority: Pubkey,
    address_merkle_context: AddressMerkleContext,
) -> ([u8; 32], u8) {
    // First derive the address seed using hashv_to_bn254_field_size_be
    let address_seed = derive_address_seed(
        &[USER_VAULT.as_bytes(), authority.as_ref()],
        &zk_onchain::ID,
    );

    // Then derive the final address and get the bump
    let merkle_tree_pubkey = address_merkle_context.address_merkle_tree_pubkey.to_bytes();
    let final_input = [merkle_tree_pubkey, address_seed].concat();
    let (final_address, bump) = hash_to_bn254_field_size_be(final_input.as_slice()).unwrap();

    (final_address, bump)
}

pub fn derive_address_seed(seeds: &[&[u8]], program_id: &Pubkey) -> [u8; 32] {
    let mut inputs = Vec::with_capacity(seeds.len() + 1);
    let program_id = program_id.to_bytes();
    inputs.push(program_id.as_slice());
    inputs.extend(seeds);
    hashv_to_bn254_field_size_be(inputs.as_slice())
}
