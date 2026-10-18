//! Printing a parsed command and parsing the print gives the same command, for many generated commands.
//! A small deterministic generator (no extra crate): the same seed always produces the same cases.
use gqodb_lang::{parse, to_canonical};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() as usize) % items.len()]
    }
    fn chance(&mut self, one_in: u64) -> bool {
        self.next() % one_in == 0
    }
}

const STREAMS: &[&str] = &["trades", "quotes", "book", "TRADES", "Book"];
const SYMBOLS: &[&str] = &[
    "EURUSD",
    "BTC-USDT",
    "BRK.B",
    "'1INCHUSDT'",
    "\"EUR/USD\"",
    "'a b'",
    "x:y/z",
    "'in'",
    "eurusd",
];
const VENUES: &[&str] = &[
    "bybit",
    "'on'",
    "'as'",
    "okx_spot",
    "\"my venue\"",
    "'123'",
    "binance-perp",
];
const RANGES: &[&str] = &[
    "2026-03-01..2026-03-02",
    "2026-03-01..",
    "..2026-03-02T12:30:00.123456789Z",
    "..",
    "100..200",
    "0..9223372036854775807",
];
const CLOCKS: &[&str] = &[
    "available",
    "received",
    "local",
    "wall",
    "source",
    "broker",
    "sequence",
    "BROKER",
];
const PATHS: &[&str] = &[
    "./data",
    "/mnt/x/y.gqodb.tick",
    "'my data'",
    "\"it's\"",
    "data",
    "'a;b'",
];
const COLUMNS: &[&str] = &[
    "price", "quantity", "side", "bid", "ask", "level", "action", "sequence",
];

fn generate(rng: &mut Rng) -> String {
    let named = rng.chance(4);
    let mut line = String::from("show ");
    let stream = rng.pick(STREAMS);
    let symbol = rng.pick(SYMBOLS);
    let venue = if rng.chance(2) {
        Some(*rng.pick(VENUES))
    } else {
        None
    };
    let range = if rng.chance(2) {
        Some(*rng.pick(RANGES))
    } else {
        None
    };
    let clock = if rng.chance(2) {
        Some(*rng.pick(CLOCKS))
    } else {
        None
    };
    let path = if rng.chance(2) {
        Some(*rng.pick(PATHS))
    } else {
        None
    };
    if named {
        line.push_str(&format!("stream={stream} symbol={symbol}"));
        if let Some(v) = venue {
            line.push_str(&format!(" venue={v}"));
        }
        if let Some(r) = range {
            line.push_str(&format!(" range={r}"));
        }
        if let Some(c) = clock {
            line.push_str(&format!(" clock={c}"));
        }
        if let Some(p) = path {
            line.push_str(&format!(" path={p}"));
        }
    } else {
        line.push_str(&format!("{stream} {symbol}"));
        if let Some(v) = venue {
            line.push_str(&format!(" {v}"));
        }
        if let Some(r) = range {
            line.push_str(&format!(" {r}"));
            if let Some(c) = clock {
                line.push_str(&format!(" on {c}"));
            }
        }
        if let Some(p) = path {
            line.push_str(&format!(" in {p}"));
        }
    }
    if rng.chance(2) {
        line.push_str(&format!(" | skip {}", rng.next() % 1000));
    }
    if rng.chance(2) {
        line.push_str(&format!(" | take {}", rng.next()));
    }
    if rng.chance(3) {
        let n = 1 + (rng.next() % 3) as usize;
        let cols: Vec<&str> = (0..n).map(|_| *rng.pick(COLUMNS)).collect();
        line.push_str(&format!(" | columns {}", cols.join(",")));
    }
    if rng.chance(3) {
        line.push_str(" | order");
        if rng.chance(2) {
            line.push_str(&format!(" on {}", rng.pick(CLOCKS)));
        }
        if rng.chance(2) {
            line.push_str(" desc");
        }
    }
    line
}

#[test]
fn print_then_parse_is_stable_for_5000_generated_commands() {
    let mut rng = Rng(0x5EED);
    for n in 0..5000 {
        let src = generate(&mut rng);
        let first = parse(&src).unwrap_or_else(|e| panic!("case {n}: {src}\n{e:?}"));
        let canon = to_canonical(&first);
        let second =
            parse(&canon).unwrap_or_else(|e| panic!("case {n}: {src}\ncanonical: {canon}\n{e:?}"));
        assert_eq!(to_canonical(&second), canon, "case {n}: {src}");
        assert_eq!(first.commands.len(), second.commands.len());
    }
}

#[test]
fn the_canonical_form_uses_canonical_clock_names_and_the_positional_shape() {
    let s = parse("SHOW Trades EURUSD bybit 2026-03-01..2026-03-02 ON Wall IN ./data | TAKE 5")
        .unwrap();
    assert_eq!(
        to_canonical(&s),
        "show trades EURUSD bybit 2026-03-01..2026-03-02 on received in ./data | take 5"
    );
    let s = parse("show clock=broker symbol=A stream=book").unwrap();
    assert_eq!(to_canonical(&s), "show stream=book symbol=A clock=source");
    let s = parse(
        "use clock local\nuse path 'my data'\nhelp clocks\nquit\nexplain show trades A in ./d",
    )
    .unwrap();
    assert_eq!(
        to_canonical(&s),
        "use clock received\nuse path 'my data'\nhelp clocks\nquit\nexplain show trades A in ./d"
    );
}
