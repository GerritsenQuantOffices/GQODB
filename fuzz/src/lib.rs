//! Shared helpers for the gqodb fuzz targets: checksum fix-ups for the
//! structure-aware mutator, seed builders, `arbitrary` generators for valid data
//! and a scratch-file helper. Nothing here is used by the library crates.
#![forbid(unsafe_code)]

pub mod arb;
pub mod fixup;
pub mod harness;
pub mod scratch;
pub mod seeds;
pub mod segment;
pub mod typesfuzz;

/// Defines the libFuzzer custom mutator used by the byte-oriented targets.
///
/// Plain byte mutation almost never produces an input whose CRC32 still
/// matches, so the checksum gates would hide every code path behind them. This
/// mutator mutates normally and then, for 7 of 8 inputs, recomputes every
/// checksum layer of whichever native format the (possibly mutated) magic names.
/// The remaining 1 of 8 keep the stale checksum so the rejection path is also exercised.
#[macro_export]
macro_rules! fix_checksums_mutator {
    () => {
        ::libfuzzer_sys::fuzz_mutator!(|data: &mut [u8],
                                        size: usize,
                                        max_size: usize,
                                        seed: u32| {
            let mut size = ::libfuzzer_sys::fuzzer_mutate(data, size, max_size);
            if seed % 6 == 1 {
                // Length-consistent metadata growth: byte-level mutation cannot do this because
                // it would also have to update the 4-byte metadata length.
                let extra = [16, 1024, 16 * 1024, 60_000][(seed / 6 % 4) as usize];
                size = $crate::fixup::grow_real_meta(data, size, extra, (seed / 24) as usize);
            }
            if seed % 8 != 0 {
                $crate::fixup::auto(&mut data[..size]);
            }
            size
        });
    };
}

/// For targets that parse Parquet: `gqodb-blocks` contains known upstream `parquet`/`arrow-*` panics
/// (see `parquet::open`) and turns them into errors. libFuzzer's hook would abort on such a
/// panic before the library can catch it, so ignore panics whose location is inside
/// the `parquet` and `arrow-*` dependency sources only. A panic anywhere else, or one that escapes the library's guard and
/// unwinds into the fuzz target, still aborts and is reported.
pub fn allow_contained_upstream_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.location().is_some_and(|l| {
            let f = l.file();
            f.contains("registry/src/") && (f.contains("/arrow-") || f.contains("/parquet-"))
        }) {
            if std::env::var_os("GQODB_FUZZ_TRACE").is_some() {
                eprintln!(
                    "[contained-upstream-panic] {info}\n{}",
                    std::backtrace::Backtrace::force_capture()
                );
            }
            return;
        }
        previous(info)
    }));
}
