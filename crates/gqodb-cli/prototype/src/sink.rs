//! Where result rows go: an aligned table, CSV, a JSON array or one JSON object per line.
use crate::value::Cell;
use std::io::{self, Write};

pub trait Sink {
    fn begin(&mut self, columns: &[String]) -> io::Result<()>;
    fn row(&mut self, cells: &[Cell]) -> io::Result<()>;
    fn finish(&mut self) -> io::Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Table,
    Csv,
    Json,
    Ndjson,
}

impl Format {
    pub fn parse(word: &str) -> Option<Format> {
        match word {
            "table" => Some(Format::Table),
            "csv" => Some(Format::Csv),
            "json" => Some(Format::Json),
            "ndjson" => Some(Format::Ndjson),
            _ => None,
        }
    }
}

pub fn make_sink<'a>(format: Format, out: &'a mut dyn Write) -> Box<dyn Sink + 'a> {
    match format {
        Format::Table => Box::new(TableSink {
            out,
            columns: Vec::new(),
            rows: Vec::new(),
        }),
        Format::Csv => Box::new(CsvSink { out }),
        Format::Json => Box::new(JsonSink {
            out,
            columns: Vec::new(),
            rows: 0,
        }),
        Format::Ndjson => Box::new(NdjsonSink {
            out,
            columns: Vec::new(),
        }),
    }
}

fn text(cell: &Cell) -> String {
    match cell {
        Cell::Null => String::new(),
        Cell::Int(i) => i.to_string(),
        Cell::Text(t) => t.clone(),
    }
}

pub fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_cell(cell: &Cell) -> String {
    match cell {
        Cell::Null => "null".to_string(),
        Cell::Int(i) => i.to_string(),
        Cell::Text(t) => json_string(t),
    }
}

fn json_object(columns: &[String], cells: &[Cell]) -> String {
    let parts: Vec<String> = columns
        .iter()
        .zip(cells)
        .map(|(c, v)| format!("{}:{}", json_string(c), json_cell(v)))
        .collect();
    format!("{{{}}}", parts.join(","))
}

struct TableSink<'a> {
    out: &'a mut dyn Write,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Sink for TableSink<'_> {
    fn begin(&mut self, columns: &[String]) -> io::Result<()> {
        self.columns = columns.to_vec();
        Ok(())
    }
    fn row(&mut self, cells: &[Cell]) -> io::Result<()> {
        self.rows.push(cells.iter().map(text).collect());
        Ok(())
    }
    fn finish(&mut self) -> io::Result<()> {
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.chars().count()).collect();
        for row in &self.rows {
            for (w, cell) in widths.iter_mut().zip(row) {
                *w = (*w).max(cell.chars().count());
            }
        }
        let line = |cells: &[String]| -> String {
            let padded: Vec<String> = cells
                .iter()
                .zip(&widths)
                .map(|(c, w)| format!("{c:<w$}"))
                .collect();
            padded.join("  ").trim_end().to_string()
        };
        writeln!(self.out, "{}", line(&self.columns))?;
        let rule: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        writeln!(self.out, "{}", rule.join("  "))?;
        for row in &self.rows {
            writeln!(self.out, "{}", line(row))?;
        }
        self.out.flush()
    }
}

struct CsvSink<'a> {
    out: &'a mut dyn Write,
}

impl Sink for CsvSink<'_> {
    fn begin(&mut self, columns: &[String]) -> io::Result<()> {
        let header: Vec<String> = columns.iter().map(|c| csv_field(c)).collect();
        writeln!(self.out, "{}", header.join(","))
    }
    fn row(&mut self, cells: &[Cell]) -> io::Result<()> {
        let fields: Vec<String> = cells.iter().map(|c| csv_field(&text(c))).collect();
        writeln!(self.out, "{}", fields.join(","))
    }
    fn finish(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

struct JsonSink<'a> {
    out: &'a mut dyn Write,
    columns: Vec<String>,
    rows: u64,
}

impl Sink for JsonSink<'_> {
    fn begin(&mut self, columns: &[String]) -> io::Result<()> {
        self.columns = columns.to_vec();
        write!(self.out, "[")
    }
    fn row(&mut self, cells: &[Cell]) -> io::Result<()> {
        write!(
            self.out,
            "{}\n{}",
            if self.rows == 0 { "" } else { "," },
            json_object(&self.columns, cells)
        )?;
        self.rows += 1;
        Ok(())
    }
    fn finish(&mut self) -> io::Result<()> {
        writeln!(self.out, "{}]", if self.rows == 0 { "" } else { "\n" })?;
        self.out.flush()
    }
}

struct NdjsonSink<'a> {
    out: &'a mut dyn Write,
    columns: Vec<String>,
}

impl Sink for NdjsonSink<'_> {
    fn begin(&mut self, columns: &[String]) -> io::Result<()> {
        self.columns = columns.to_vec();
        Ok(())
    }
    fn row(&mut self, cells: &[Cell]) -> io::Result<()> {
        writeln!(self.out, "{}", json_object(&self.columns, cells))
    }
    fn finish(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(format: Format) -> String {
        let mut buf = Vec::new();
        {
            let mut sink = make_sink(format, &mut buf);
            sink.begin(&["symbol".into(), "price".into(), "note".into(), "n".into()])
                .unwrap();
            sink.row(&[
                Cell::Text("EURUSD".into()),
                Cell::Text("1.08342".into()),
                Cell::Null,
                Cell::Int(-7),
            ])
            .unwrap();
            sink.row(&[
                Cell::Text("a,\"b\"".into()),
                Cell::Text("2.00000".into()),
                Cell::Text("x\ny".into()),
                Cell::Int(0),
            ])
            .unwrap();
            sink.finish().unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn csv_quotes_only_where_needed_and_writes_empty_for_null() {
        assert_eq!(
            run(Format::Csv),
            "symbol,price,note,n\nEURUSD,1.08342,,-7\n\"a,\"\"b\"\"\",2.00000,\"x\ny\",0\n"
        );
    }

    #[test]
    fn json_keeps_decimals_as_text_and_ints_as_numbers() {
        assert_eq!(
            run(Format::Json),
            "[\n{\"symbol\":\"EURUSD\",\"price\":\"1.08342\",\"note\":null,\"n\":-7},\n{\"symbol\":\"a,\\\"b\\\"\",\"price\":\"2.00000\",\"note\":\"x\\ny\",\"n\":0}\n]\n"
        );
    }

    #[test]
    fn ndjson_is_one_object_per_line() {
        assert_eq!(
            run(Format::Ndjson),
            "{\"symbol\":\"EURUSD\",\"price\":\"1.08342\",\"note\":null,\"n\":-7}\n{\"symbol\":\"a,\\\"b\\\"\",\"price\":\"2.00000\",\"note\":\"x\\ny\",\"n\":0}\n"
        );
    }

    #[test]
    fn an_empty_json_result_is_an_empty_array() {
        let mut buf = Vec::new();
        {
            let mut sink = make_sink(Format::Json, &mut buf);
            sink.begin(&["a".into()]).unwrap();
            sink.finish().unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "[]\n");
    }

    #[test]
    fn the_table_aligns_columns_and_keeps_a_header_when_empty() {
        let table = run(Format::Table);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[0], "symbol  price    note  n");
        assert_eq!(lines[1], "------  -------  ----  --");
        assert_eq!(lines[2], "EURUSD  1.08342        -7");
        let mut buf = Vec::new();
        {
            let mut sink = make_sink(Format::Table, &mut buf);
            sink.begin(&["a".into(), "bb".into()]).unwrap();
            sink.finish().unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "a  bb\n-  --\n");
    }

    #[test]
    fn json_strings_escape_control_characters() {
        assert_eq!(json_string("a\u{1}\t\"\\"), "\"a\\u0001\\t\\\"\\\\\"");
    }
}
