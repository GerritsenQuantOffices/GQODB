#![no_main]
use libfuzzer_sys::fuzz_target;
gqodb_fuzz::fix_checksums_mutator!();
fuzz_target!(init: gqodb_fuzz::allow_contained_upstream_panics(), |data: &[u8]| gqodb_fuzz::harness::parquet_decode(data));
