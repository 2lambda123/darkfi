/* This file is part of DarkFi (https://dark.fi)
 *
 * Copyright (C) 2020-2022 Dyne.org foundation
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use darkfi_sdk::crypto::ContractId;
use darkfi_serial::Decodable;
use log::error;
use std::io::Cursor;
use wasmer::{FunctionEnvMut, WasmPtr};

use crate::{
    runtime::vm_runtime::{ContractSection, Env},
    Result,
};

/// Internal wasm runtime API for sled trees
pub struct DbHandle {
    pub contract_id: ContractId,
    tree: sled::Tree,
}

impl DbHandle {
    pub fn new(contract_id: ContractId, tree: sled::Tree) -> Self {
        Self { contract_id, tree }
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(v) = self.tree.get(key)? {
            return Ok(Some(v.to_vec()))
        };

        Ok(None)
    }

    pub fn apply_batch(&self, batch: sled::Batch) -> Result<()> {
        Ok(self.tree.apply_batch(batch)?)
    }

    pub fn flush(&self) -> Result<()> {
        let _ = self.tree.flush()?;
        Ok(())
    }
}

/// Only deploy() can call this. Creates a new database instance for this contract.
///
/// ```
///     type DbHandle = u32;
///     db_init(db_name) -> DbHandle
/// ```
pub(crate) fn db_init(ctx: FunctionEnvMut<Env>, ptr: WasmPtr<u8>, len: u32) -> i32 {
    let env = ctx.data();
    match env.contract_section {
        ContractSection::Deploy => {
            let memory_view = env.memory_view(&ctx);
            let db = &env.blockchain.sled_db;
            let contracts = &env.blockchain.contracts;
            let contract_id = &env.contract_id;

            let Ok(mem_slice) = ptr.slice(&memory_view, len) else {
                error!(target: "wasm_runtime::db_init", "Failed to make slice from ptr");
                return -2
            };

            let mut buf = vec![0_u8; len as usize];
            if let Err(e) = mem_slice.read_slice(&mut buf) {
                error!(target: "wasm_runtime::db_init", "Failed to read from memory slice: {}", e);
                return -2
            };

            let mut buf_reader = Cursor::new(buf);

            let cid: ContractId = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_init", "Failed to decode ContractId: {}", e);
                    return -2
                }
            };

            let db_name: String = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_init", "Failed to decode db_name: {}", e);
                    return -2
                }
            };

            // TODO: Ensure we've read the entire buffer above.

            let tree_handle = match contracts.init(db, &cid, &db_name) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime:db_lookup", "Failed to init db: {}", e);
                    return -2
                }
            };

            // TODO: Make sure we don't duplicate the DbHandle in the vec.
            //       It should behave like an ordered set.
            // In `lookup()` we also create a `sled::Batch`. This is done for
            // some simplicity reasons, and also for possible future changes.
            // However, we make sure that unauthorized writes are not available
            // from other functions that interface with the databases.
            let mut db_handles = env.db_handles.borrow_mut();
            let mut db_batches = env.db_batches.borrow_mut();
            db_handles.push(DbHandle::new(cid, tree_handle));
            db_batches.push(sled::Batch::default());
            return (db_handles.len() - 1) as i32
        }
        _ => {
            error!(target: "wasm_runtime::db_init", "db_init called in unauthorized section");
            return -1
        }
    }
}

/// Everyone can call this. Lookups up a database handle from its name.
///
/// ```
///     type DbHandle = u32;
///     db_lookup(db_name) -> DbHandle
/// ```
pub(crate) fn db_lookup(ctx: FunctionEnvMut<Env>, ptr: WasmPtr<u8>, len: u32) -> i32 {
    let env = ctx.data();
    match env.contract_section {
        ContractSection::Deploy |
        ContractSection::Exec |
        ContractSection::Update |
        ContractSection::Metadata => {
            let memory_view = env.memory_view(&ctx);
            let db = &env.blockchain.sled_db;
            let contracts = &env.blockchain.contracts;

            let Ok(mem_slice) = ptr.slice(&memory_view, len) else {
                error!(target: "wasm_runtime::db_lookup", "Failed to make slice from ptr");
                return -2
            };

            let mut buf = vec![0_u8; len as usize];
            if let Err(e) = mem_slice.read_slice(&mut buf) {
                error!(target: "wasm_runtime::db_lookup", "Failed to read from memory slice: {}", e);
                return -2
            };

            let mut buf_reader = Cursor::new(buf);

            let cid: ContractId = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_lookup", "Failed to decode ContractId: {}", e);
                    return -2
                }
            };

            let db_name: String = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_lookup", "Failed to decode db_name: {}", e);
                    return -2
                }
            };

            // TODO: Ensure we've read the entire buffer above.

            let tree_handle = match contracts.lookup(db, &cid, &db_name) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime:db_lookup", "Failed to lookup db: {}", e);
                    return -2
                }
            };

            // TODO: Make sure we don't duplicate the DbHandle in the vec.
            //       It should behave like an ordered set.
            // In `lookup()` we also create a `sled::Batch`. This is done for
            // some simplicity reasons, and also for possible future changes.
            // However, we make sure that unauthorized writes are not available
            // from other functions that interface with the databases.
            let mut db_handles = env.db_handles.borrow_mut();
            let mut db_batches = env.db_batches.borrow_mut();
            db_handles.push(DbHandle::new(cid, tree_handle));
            db_batches.push(sled::Batch::default());
            return (db_handles.len() - 1) as i32
        }
        _ => {
            error!(target: "wasm_runtime::db_lookup", "db_lookup called in unauthorized section");
            return -1
        }
    }
}

/// Only update() can call this. Set a value within the transaction.
///
/// ```
///     db_set(tx_handle, key, value);
/// ```
pub(crate) fn db_set(ctx: FunctionEnvMut<Env>, ptr: WasmPtr<u8>, len: u32) -> i32 {
    let env = ctx.data();
    match env.contract_section {
        ContractSection::Deploy | ContractSection::Update => {
            let memory_view = env.memory_view(&ctx);

            let Ok(mem_slice) = ptr.slice(&memory_view, len) else {
                error!(target: "wasm_runtime::db_set", "Failed to make slice from ptr");
                return -2
            };

            let mut buf = vec![0_u8; len as usize];
            if let Err(e) = mem_slice.read_slice(&mut buf) {
                error!(target: "wasm_runtime:db_set", "Failed to read from memory slice: {}", e);
                return -2
            };

            let mut buf_reader = Cursor::new(buf);

            // FIXME: There's a type DbHandle=u32, but this should maybe be renamed
            let db_handle: u32 = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_set", "Failed to decode DbHandle: {}", e);
                    return -2
                }
            };
            let db_handle = db_handle as usize;

            let key: Vec<u8> = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_set", "Failed to decode key vec: {}", e);
                    return -2
                }
            };

            let value: Vec<u8> = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_set", "Failed to decode value vec: {}", e);
                    return -2
                }
            };

            // TODO: Ensure we've read the entire buffer above.

            let db_handles = env.db_handles.borrow();
            let mut db_batches = env.db_batches.borrow_mut();

            if db_handles.len() <= db_handle || db_batches.len() <= db_handle {
                error!(target: "wasm_runtime::db_set", "Requested DbHandle that is out of bounds");
                return -2
            }

            let handle_idx = db_handle;
            let db_handle = &db_handles[handle_idx];
            let db_batch = &mut db_batches[handle_idx];

            if db_handle.contract_id != env.contract_id {
                error!(target: "wasm_runtime::db_set", "Unauthorized to write to DbHandle");
                return -1
            }

            db_batch.insert(key, value);

            0
        }
        _ => -1,
    }
}

/// Everyone can call this. Will read a key from the key-value store.
///
/// ```
///     value = db_get(db_handle, key);
/// ```
pub(crate) fn db_get(ctx: FunctionEnvMut<Env>, ptr: WasmPtr<u8>, len: u32) -> i64 {
    let env = ctx.data();
    match env.contract_section {
        ContractSection::Exec | ContractSection::Metadata => {
            let memory_view = env.memory_view(&ctx);
            let db = &env.blockchain.sled_db;
            let contracts = &env.blockchain.contracts;

            let Ok(mem_slice) = ptr.slice(&memory_view, len) else {
                error!(target: "wasm_runtime::db_get", "Failed to make slice from ptr");
                return -2
            };

            let mut buf = vec![0_u8; len as usize];
            if let Err(e) = mem_slice.read_slice(&mut buf) {
                error!(target: "wasm_runtime::db_get", "Failed to read from memory slice: {}", e);
                return -2
            };

            let mut buf_reader = Cursor::new(buf);

            // FIXME: There's a type DbHandle=u32, but this should maybe be renamed
            let db_handle: u32 = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_get", "Failed to decode DbHandle: {}", e);
                    return -2
                }
            };
            let db_handle = db_handle as usize;

            let key: Vec<u8> = match Decodable::decode(&mut buf_reader) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_get", "Failed to decode key from vec: {}", e);
                    return -2
                }
            };

            // TODO: Ensure we've read the entire buffer above.

            let db_handles = env.db_handles.borrow();
            let db_batches = env.db_batches.borrow();

            if db_handles.len() <= db_handle || db_batches.len() <= db_handle {
                error!(target: "wasm_runtime::db_get", "Requested DbHandle that is out of bounds");
                return -2
            }

            let handle_idx = db_handle;
            let db_handle = &db_handles[handle_idx];

            let ret = match db_handle.get(&key) {
                Ok(v) => v,
                Err(e) => {
                    error!(target: "wasm_runtime::db_get", "Internal error getting from tree");
                    return -2
                }
            };

            let Some(return_data) = ret else {
                log::debug!("returned empty vec");
                return -3
            };

            // Copy Vec<u8> to the VM
            let mut objects = env.objects.borrow_mut();
            objects.push(return_data);
            (objects.len() - 1) as i64
        }
        _ => -1,
    }
}