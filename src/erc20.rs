use std::collections::HashMap;

use alloy::{
    sol,
    sol_types::{SolCall, SolValue},
};
use eyre::eyre;
use reth_revm::{
    database::StateProviderDatabase,
    db::{CacheDB, StateBuilder},
};
use revm::{
    context::result::ExecutionResult,
    primitives::{address, keccak256, Address, FixedBytes, TxKind, B256, U256},
    Context, ExecuteEvm, MainBuilder, MainContext,
};
use serde::Serialize;
use ERC20::balanceOfCall;

use crate::config::Config;

sol! {
    #[sol(rpc, abi)]
    contract ERC20 {
        function balanceOf(address owner) external view returns (uint256 balance);
    }
}

pub type MappingIndexCache = HashMap<Address, U256>;

#[derive(Serialize, Debug)]
pub struct Erc20BalanceSlot {
    pub address: Address,
    pub slot: FixedBytes<32>,
    pub mapping_slot: Option<U256>,
}

type Slot = (Address, U256);

pub fn get_erc20_balance_slot(
    config: &Config,
    token: Address,
    user: Address,
    mapping_slot: Option<&MappingIndexCache>,
) -> eyre::Result<Erc20BalanceSlot> {
    if let Some(cache) = mapping_slot {
        if let Some(mapping_index) = cache.get(&token) {
            let user_bytes32 = user.into_word();
            let slot = keccak256((user_bytes32, *mapping_index).abi_encode());

            return Ok(Erc20BalanceSlot {
                address: token,
                slot,
                mapping_slot: Some(*mapping_index),
            });
        }
    }

    let balanceof_calldata = ERC20::balanceOfCall { owner: user }.abi_encode();
    let state = config.get_latest_state();
    let db = CacheDB::new(StateProviderDatabase::new(state));
    let mut state = StateBuilder::new_with_database(db).build();

    let mut evm = Context::mainnet()
        .with_db(&mut state)
        .modify_tx_chained(|tx| {
            tx.caller = address!("0000000000000000000000000000000000000000");
            tx.kind = TxKind::Call(token);
            tx.data = balanceof_calldata.into();
        })
        .build_mainnet();

    let ref_tx = evm.replay().unwrap();

    let min_slot = U256::from(1) << U256::from(128);
    let touched_slots: Vec<Slot> = ref_tx
        .state
        .iter()
        .flat_map(|(address, account)| {
            account
                .storage
                .iter()
                .filter_map(move |(slot, _value)| match *slot >= min_slot {
                    true => Some((*address, *slot)),
                    false => None,
                })
        })
        .collect();

    let user_bytes32 = user.into_word();

    if touched_slots.len() == 1 {
        let storage_slot = FixedBytes::from(touched_slots[0].1.to_be_bytes());
        let mapping_slot = get_mapping_slot(user_bytes32, storage_slot);
        return Ok(Erc20BalanceSlot {
            address: touched_slots[0].0,
            slot: FixedBytes::from_slice(touched_slots[0].1.as_le_slice()),
            mapping_slot,
        });
    }

    let mut best_slot: (Option<Slot>, U256) = (None, U256::ZERO);
    let fake_balance = U256::from(2)
        .pow(U256::from(96))
        .checked_sub(U256::ONE)
        .unwrap();

    for (contract, slot) in &touched_slots {
        let state = config.get_latest_state();
        let mut cache_db = CacheDB::new(StateProviderDatabase::new(state));
        cache_db
            .insert_account_storage(*contract, *slot, fake_balance)
            .unwrap();

        let state = StateBuilder::new_with_database(cache_db).build();

        let encoded = balanceOfCall { owner: user }.abi_encode();

        let mut evm = Context::mainnet()
            .with_db(state)
            .modify_tx_chained(|tx| {
                tx.caller = address!("0000000000000000000000000000000000000001");
                tx.kind = TxKind::Call(token);
                tx.data = encoded.into();
            })
            .build_mainnet();

        let ref_tx = evm.replay().unwrap();
        let result = ref_tx.result;

        let balance = match result {
            ExecutionResult::Success { output, .. } => {
                U256::abi_decode(output.data(), false).unwrap_or(U256::ZERO)
            }
            _ => continue,
        };

        if balance > best_slot.1 {
            best_slot = (Some((*contract, *slot)), balance)
        }
    }

    match best_slot.0 {
        Some((address, slot)) => {
            let storage_slot = FixedBytes::from_slice(slot.as_le_slice());
            let mapping_slot = get_mapping_slot(user_bytes32, storage_slot);

            return Ok(Erc20BalanceSlot {
                address,
                slot: storage_slot,
                mapping_slot,
            });
        }
        None => {
            // The loop finished without finding a slot that returned the fake_balance
            Err(eyre!(
                "Could not definitively identify the ERC20 balance slot for user {:?} at token {:?}. Touched slots: {:?}",
                user,
                token,
                touched_slots // Include touched slots for debugging
            ))
        }
    }

    //let storage_slot = FixedBytes::from_slice(best_slot.1.as_le_slice());
    //let mapping_slot = get_mapping_slot(user_bytes32, storage_slot);
    //
    //return Ok(Erc20BalanceSlot {
    //    address: best_slot.0.unwrap().0,
    //    slot: storage_slot,
    //    mapping_slot,
    //});
}

//fn get_balance(

fn get_mapping_slot(user_bytes32: B256, storage_slot: B256) -> Option<U256> {
    for i in 0..20 {
        let hashed_slot = keccak256((user_bytes32, U256::from(i)).abi_encode());
        if hashed_slot == storage_slot {
            return Some(U256::from(i));
        }
    }

    None
}
