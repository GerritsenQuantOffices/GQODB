#![no_main]
use gqodb_lang::{Session, parse, plan, to_canonical};
use libfuzzer_sys::fuzz_target;

// Properties: parsing never panics; a script that parses prints to text that parses again and prints identically;
// planning never panics, whatever the parsed command is.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if text.len() > 4096 {
        return;
    }
    if let Ok(script) = parse(text) {
        let canonical = to_canonical(&script);
        let again = parse(&canonical).unwrap_or_else(|e| panic!("canonical text must parse: {canonical:?}: {e:?}"));
        assert_eq!(to_canonical(&again), canonical);
        for command in &script.commands {
            let _ = plan(command, &Session::default());
        }
    }
});
