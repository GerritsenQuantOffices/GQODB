#![no_main]
use gqodb_lang::{Session, parse, plan, render_all, to_canonical};
use libfuzzer_sys::fuzz_target;

// Properties: parsing never panics; a script that parses prints to text that parses again and prints identically;
// planning never panics, whatever the parsed command is; every diagnostic, from the parser or the planner, renders
// without a panic (it cuts the source line on character boundaries).
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    if text.len() > 4096 {
        return;
    }
    match parse(text) {
        Ok(script) => {
            let canonical = to_canonical(&script);
            let again = parse(&canonical).unwrap_or_else(|e| panic!("canonical text must parse: {canonical:?}: {e:?}"));
            assert_eq!(to_canonical(&again), canonical);
            for command in &script.commands {
                if let Err(diags) = plan(command, &Session::default()) {
                    for d in &diags {
                        let _ = d.render(text);
                    }
                    let _ = render_all(&diags, text);
                }
            }
        }
        Err(diags) => {
            for d in &diags {
                let shown = d.render(text);
                assert!(shown.starts_with("error["));
            }
            let _ = render_all(&diags, text);
        }
    }
});
