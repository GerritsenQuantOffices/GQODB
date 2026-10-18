#![no_main]
use libfuzzer_sys::fuzz_target;
gqodb_fuzz::fix_checksums_mutator!();
fuzz_target!(|data: &[u8]| gqodb_fuzz::harness::store_reader(data));
