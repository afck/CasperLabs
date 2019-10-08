#![no_std]

#[macro_use]
extern crate alloc;
extern crate contract_ffi;

use contract_ffi::contract_api::{self, Error};
use contract_ffi::execution::Phase;
use contract_ffi::value::account::PurseId;
use contract_ffi::value::U512;

const GET_PAYMENT_PURSE: &str = "get_payment_purse";

fn standard_payment(amount: U512) {
    let main_purse = contract_api::main_purse();

    let pos_pointer = contract_api::get_pos();

    let payment_purse: PurseId =
        contract_api::call_contract(pos_pointer, &(GET_PAYMENT_PURSE,), &vec![]);

    if contract_api::transfer_from_purse_to_purse(main_purse, payment_purse, amount).is_err() {
        contract_api::revert(Error::Transfer);
    }
}

#[no_mangle]
pub extern "C" fn call() {
    let known_phase: Phase = match contract_api::get_arg(0) {
        Some(Ok(data)) => data,
        Some(Err(_)) => contract_api::revert(Error::InvalidArgument),
        None => contract_api::revert(Error::MissingArgument),
    };
    let get_phase = contract_api::get_phase();
    assert_eq!(
        get_phase, known_phase,
        "get_phase did not return known_phase"
    );

    standard_payment(U512::from(10_000_000));
}
