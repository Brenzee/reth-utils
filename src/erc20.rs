use std::collections::HashMap;

use alloy::{
    sol,
    sol_types::{SolCall, SolValue},
};
use eyre::eyre;
use reth::providers::StateProvider;
use reth_revm::{
    database::StateProviderDatabase,
    db::{CacheDB, StateBuilder},
};
use revm::{
    context::result::ExecutionResult,
    primitives::{
        address, keccak256, map::foldhash::fast::RandomState, Address, FixedBytes, TxKind, B256,
        U256,
    },
    state::Account,
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
pub type RevmDb = CacheDB<StateProviderDatabase<Box<dyn StateProvider>>>;

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

    let state = config.get_latest_state();
    let mut db = CacheDB::new(StateProviderDatabase::new(state));

    let (_, touched_state) = get_erc20_balance(&mut db, token, user);

    let min_slot = U256::from(1) << U256::from(128);
    let touched_slots: Vec<Slot> = touched_state
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

        let (balance, _) = get_erc20_balance(&mut cache_db, token, user);

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
}

fn get_erc20_balance(
    cache_db: &mut RevmDb,
    token: Address,
    user: Address,
) -> (U256, HashMap<Address, Account, RandomState>) {
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

    match ref_tx.result {
        ExecutionResult::Success { output, .. } => (
            U256::abi_decode(output.data(), false).unwrap_or(U256::ZERO),
            ref_tx.state,
        ),
        _ => (U256::ZERO, ref_tx.state),
    }
}

fn get_mapping_slot(user_bytes32: B256, storage_slot: B256) -> Option<U256> {
    for i in 0..20 {
        let hashed_slot = keccak256((user_bytes32, U256::from(i)).abi_encode());
        if hashed_slot == storage_slot {
            return Some(U256::from(i));
        }
    }

    None
}
