//! Optional bounded concurrency across independent blocks. No background threads.
use crate::real::{self, Decoder, Mode};
use anyhow::{Result, ensure};
use arrow_array::RecordBatch;
use bytes::Bytes;

/// One worker by default; additional workers trade CPU/RAM for throughput.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub workers: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { workers: 1 }
    }
}

impl Options {
    pub fn encode(self, batches: &[RecordBatch], mode: Mode) -> Result<Vec<Vec<u8>>> {
        self.map(batches, || (), |_, batch| real::encode(batch, mode))
    }

    pub fn decode(self, blocks: &[Bytes], mode: Mode) -> Result<Vec<RecordBatch>> {
        self.map(blocks, Decoder::default, |decoder, block| {
            decoder.decode(block.clone(), mode)
        })
    }

    fn map<T: Sync, U: Send, S>(
        self,
        input: &[T],
        state: impl Fn() -> S + Sync,
        operation: impl Fn(&mut S, &T) -> Result<U> + Sync,
    ) -> Result<Vec<U>> {
        ensure!(
            (1..=64).contains(&self.workers),
            "workers must be between 1 and 64"
        );
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let workers = self.workers.min(input.len());
        if workers == 1 {
            let mut state = state();
            return input.iter().map(|v| operation(&mut state, v)).collect();
        }
        std::thread::scope(|scope| {
            let handles: Vec<_> = input
                .chunks(input.len().div_ceil(workers))
                .map(|chunk| {
                    let state = &state;
                    let operation = &operation;
                    scope.spawn(move || {
                        let mut state = state();
                        chunk
                            .iter()
                            .map(|v| operation(&mut state, v))
                            .collect::<Result<Vec<U>>>()
                    })
                })
                .collect();
            // Join every worker even when one fails; no work escapes this call.
            let results: Vec<_> = handles.into_iter().map(|h| h.join()).collect();
            let mut output = Vec::with_capacity(input.len());
            for result in results {
                output.extend(result.map_err(|_| anyhow::anyhow!("block worker panicked"))??);
            }
            Ok(output)
        })
    }
}
