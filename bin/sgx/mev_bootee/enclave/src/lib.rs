// docker image versioning: 0

#![cfg_attr(not(target_env = "sgx"), no_std)]
#![cfg_attr(target_env = "sgx", feature(rustc_private))]
// #![feature(map_first_last)]

#[cfg(not(target_env = "sgx"))]
#[macro_use]
extern crate sgxlib as std;

use app_mev_bootee::{MevBooTEE, GreedyBlockBuildingStrategy, PartialBlockBuildingMode};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::prelude::v1::*;
use std::sgx_trts;
use std::sgx_types::sgx_status_t;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref PARTIAL_BLOCK_BUILDING_MODE: Mutex<Option<PartialBlockBuildingMode>> = Mutex::new(None);
    static ref APP: MevBooTEE<GreedyBlockBuildingStrategy> = {
        let mode = match &*PARTIAL_BLOCK_BUILDING_MODE.lock().unwrap() {
            Some(m) => m.clone(),
            None => panic!("partial block building mode must be set!"),
        };
        MevBooTEE::new(mode)
    };
}

#[no_mangle]
pub unsafe extern "C" fn enclave_entrypoint(eid: u64, args: *const c_char) -> sgx_status_t {
    glog::init();
    glog::info!("Initialize Enclave!");

    let args = apps::parse_args(args);
    let mode = todo!(); // get mode from args
    *PARTIAL_BLOCK_BUILDING_MODE.lock().unwrap() = Some(mode);
    match apps::run_enclave(&APP, eid, args) {
        Ok(()) => sgx_status_t::SGX_SUCCESS,
        Err(err) => err,
    }
}

#[no_mangle]
pub unsafe extern "C" fn enclave_terminate() -> sgx_status_t {
    apps::terminate(&APP);
    sgx_status_t::SGX_SUCCESS
}

#[no_mangle]
pub extern "C" fn __assert_fail(
    __assertion: *const u8,
    __file: *const u8,
    __line: u32,
    __function: *const u8,
) -> ! {
    let assertion = unsafe { CStr::from_ptr(__assertion as *const c_char).to_str() }
        .expect("__assertion is not a valid c-string!");
    let file = unsafe { CStr::from_ptr(__file as *const c_char).to_str() }
        .expect("__file is not a valid c-string!");
    let line = unsafe { CStr::from_ptr(__line as *const c_char).to_str() }
        .expect("__line is not a valid c-string!");
    let function = unsafe { CStr::from_ptr(__function as *const c_char).to_str() }
        .expect("__function is not a valid c-string!");
    println!("{}:{}:{}:{}", file, line, function, assertion);

    use sgx_trts::trts::rsgx_abort;
    rsgx_abort()
}
