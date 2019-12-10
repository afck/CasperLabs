use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::convert::From;

use crate::{
    bytesrepr::{self, FromBytes, ToBytes},
    contract_api::{self, runtime, ContractRef, Error, TURef},
    ext_ffi,
    key::{Key, UREF_SIZE},
    unwrap_or_revert::UnwrapOrRevert,
    uref::{AccessRights, URef},
    value::{CLTyped, CLValue},
};

/// Reads value under `turef` in the global state.
pub fn read<T: CLTyped + FromBytes>(turef: TURef<T>) -> Result<Option<T>, bytesrepr::Error> {
    let key: Key = turef.into();
    // Note: _bytes is necessary to keep the Vec<u8> in scope. If _bytes is dropped then key_ptr
    // becomes invalid.
    let (key_ptr, key_size, _bytes) = contract_api::to_ptr(key);
    let value_size = unsafe { ext_ffi::read_value(key_ptr, key_size) };
    get_read(value_size)
}

/// Reads the value under `key` in the context-local partition of global state.
pub fn read_local<K: ToBytes, V: CLTyped + FromBytes>(
    key: &K,
) -> Result<Option<V>, bytesrepr::Error> {
    let key_bytes = key.to_bytes()?;
    let key_bytes_ptr = key_bytes.as_ptr();
    let key_bytes_size = key_bytes.len();
    let value_size = unsafe { ext_ffi::read_value_local(key_bytes_ptr, key_bytes_size) };
    get_read(value_size)
}

/// Retrieves a value from the host buffer which has previously been populated via a call to
/// `ext_ffi::read_value` or `ext_ffi::read_value_local`.
fn get_read<T: CLTyped + FromBytes>(
    reported_value_size: i64,
) -> Result<Option<T>, bytesrepr::Error> {
    if reported_value_size < 0 {
        return Ok(None);
    }

    let value_size = reported_value_size as usize;

    let value_ptr = contract_api::alloc_bytes(value_size);
    let value_bytes = unsafe {
        ext_ffi::get_read(value_ptr);
        Vec::from_raw_parts(value_ptr, value_size, value_size)
    };
    Ok(Some(bytesrepr::deserialize(value_bytes)?))
}

/// Writes `value` under `turef` in the global state.
pub fn write<T: CLTyped + ToBytes>(turef: TURef<T>, value: T) {
    let key = Key::from(turef);
    let (key_ptr, key_size, _bytes1) = contract_api::to_ptr(key);

    let cl_value = CLValue::from_t(value).unwrap_or_revert();
    let (cl_value_ptr, cl_value_size, _bytes2) = contract_api::to_ptr(cl_value);

    unsafe {
        ext_ffi::write(key_ptr, key_size, cl_value_ptr, cl_value_size);
    }
}

/// Writes `value` under `key` in the context-local partition of global state.
pub fn write_local<K: ToBytes, V: CLTyped + ToBytes>(key: K, value: V) {
    let key_bytes = key.to_bytes().unwrap_or_revert();
    let key_bytes_ptr = key_bytes.as_ptr();
    let key_bytes_size = key_bytes.len();

    let cl_value = CLValue::from_t(value).unwrap_or_revert();
    let (cl_value_ptr, cl_value_size, _bytes) = contract_api::to_ptr(cl_value);

    unsafe {
        ext_ffi::write_local(key_bytes_ptr, key_bytes_size, cl_value_ptr, cl_value_size);
    }
}

/// Adds `value` to the one currently under `turef` in the global state.
pub fn add<T: CLTyped + ToBytes>(turef: TURef<T>, value: T) {
    let key = Key::from(turef);
    let (key_ptr, key_size, _bytes1) = contract_api::to_ptr(key);

    let cl_value = CLValue::from_t(value).unwrap_or_revert();
    let (cl_value_ptr, cl_value_size, _bytes2) = contract_api::to_ptr(cl_value);

    unsafe {
        // Could panic if `value` cannot be added to the given value in memory.
        ext_ffi::add(key_ptr, key_size, cl_value_ptr, cl_value_size);
    }
}

/// Stores the serialized bytes of an exported function under a URef generated by the host.
pub fn store_function(name: &str, named_keys: BTreeMap<String, Key>) -> ContractRef {
    let (fn_ptr, fn_size, _bytes1) = contract_api::str_ref_to_ptr(name);
    let (keys_ptr, keys_size, _bytes2) = contract_api::to_ptr(named_keys);
    let mut addr = [0u8; 32];
    unsafe {
        ext_ffi::store_function(fn_ptr, fn_size, keys_ptr, keys_size, addr.as_mut_ptr());
    }
    ContractRef::URef(URef::new(addr, AccessRights::READ_ADD_WRITE))
}

/// Stores the serialized bytes of an exported function at an immutable address generated by the
/// host.
pub fn store_function_at_hash(name: &str, named_keys: BTreeMap<String, Key>) -> ContractRef {
    let (fn_ptr, fn_size, _bytes1) = contract_api::str_ref_to_ptr(name);
    let (keys_ptr, keys_size, _bytes2) = contract_api::to_ptr(named_keys);
    let mut addr = [0u8; 32];
    unsafe {
        ext_ffi::store_function_at_hash(fn_ptr, fn_size, keys_ptr, keys_size, addr.as_mut_ptr());
    }
    ContractRef::Hash(addr)
}

/// Returns a new unforgable pointer, where value is initialized to `init`
pub fn new_turef<T: CLTyped + ToBytes>(init: T) -> TURef<T> {
    let key_ptr = contract_api::alloc_bytes(UREF_SIZE);
    let cl_value = CLValue::from_t(init).unwrap_or_revert();
    let (cl_value_ptr, cl_value_size, _cl_value_bytes) = contract_api::to_ptr(cl_value);
    let bytes = unsafe {
        ext_ffi::new_uref(key_ptr, cl_value_ptr, cl_value_size); // URef has `READ_ADD_WRITE` access
        Vec::from_raw_parts(key_ptr, UREF_SIZE, UREF_SIZE)
    };
    let key: Key = bytesrepr::deserialize(bytes).unwrap_or_revert();
    if let Key::URef(uref) = key {
        TURef::from_uref(uref).unwrap_or_revert()
    } else {
        runtime::revert(Error::UnexpectedKeyVariant);
    }
}
