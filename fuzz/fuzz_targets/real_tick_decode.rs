#![no_main]
use gqodb_blocks::real::Mode;
use libfuzzer_sys::fuzz_target;
gqodb_fuzz::fix_checksums_mutator!();
fuzz_target!(|data: &[u8]| gqodb_fuzz::harness::real_decode(Mode::Gqodb, data));
