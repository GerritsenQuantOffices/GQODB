#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| gqodb_fuzz::harness::store_structured(data));
