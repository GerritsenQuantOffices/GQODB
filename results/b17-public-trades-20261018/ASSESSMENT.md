# B17: public-data file-level benchmark (published run)

Generated from `summary.json`; protocol `docs/B17_PUBLIC_TRADES_PROTOCOL.md`. Every cell below is reported, wins and losses alike. Verdict codes: BETTER, WORSE, EQUIVALENT, INCONCLUSIVE (ratio gqodb / baseline, 95% moving-block bootstrap interval), plus EXACTNESS FAIL, NOT COMPARABLE, UNDERPOWERED.

Receipt: `BUILD_RECEIPTS.json`.

**Input data: Binance Vision (https://data.binance.vision), PAXGUSDT spot aggregate trades.** Provided by Binance under CC BY-NC-SA 4.0 and the Binance Vision Dataset Terms (version 1.0, last updated 26 August 2026). The data are not redistributed here; fetch them yourself and comply with those terms. This assessment contains aggregates only. Binance does not sponsor, endorse or partner with this project or these results. See `docs/BASELINE_LICENCES.md`.

## D1/v1

Valid rounds 15 of 23 attempted; flagged rounds: 8.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.338 MB | 466.6 ms (MAD 13.1 ms) | 230.0 ms (MAD 3.1 ms) | 237.7 ms (MAD 11.2 ms) | 6.3 ms (MAD 5.9 ms) | 9.7 ms (MAD 2.8 ms) | 244.8 MiB (MAD 4.0 MiB) | 76.0 MiB (MAD 0.0 MiB) | 406.2 ms (MAD 16.3 ms) | 294.4 ms (MAD 8.0 ms) |
| clickhouse-tuned | yes | 9.237 MB | 512.8 ms (MAD 11.9 ms) | 266.1 ms (MAD 7.0 ms) | 264.6 ms (MAD 5.0 ms) | 5.4 ms (MAD 5.4 ms) | 6.2 ms (MAD 2.3 ms) | 221.7 MiB (MAD 5.2 MiB) | 78.0 MiB (MAD 0.2 MiB) | 458.3 ms (MAD 8.4 ms) | 333.3 ms (MAD 10.0 ms) |
| duckdb-native | yes | 9.449 MB | 551.6 ms (MAD 17.8 ms) | 103.2 ms (MAD 3.8 ms) | 114.7 ms (MAD 2.4 ms) | 20.2 ms (MAD 1.3 ms) | 24.6 ms (MAD 1.5 ms) | 79.2 MiB (MAD 0.2 MiB) | 79.1 MiB (MAD 0.1 MiB) | 530.3 ms (MAD 12.5 ms) | 100.4 ms (MAD 3.8 ms) |
| duckdb-parquet-default | yes | 21.113 MB | 305.0 ms (MAD 9.6 ms) | 141.4 ms (MAD 3.9 ms) | 156.5 ms (MAD 1.9 ms) | 27.4 ms (MAD 1.0 ms) | 30.2 ms (MAD 0.8 ms) | 32.8 MiB (MAD 0.6 MiB) | 76.0 MiB (MAD 0.2 MiB) | 286.8 ms (MAD 3.8 ms) | 144.2 ms (MAD 2.7 ms) |
| duckdb-parquet-matched | yes | 8.053 MB | 278.1 ms (MAD 6.6 ms) | 119.3 ms (MAD 3.1 ms) | 124.8 ms (MAD 4.1 ms) | 23.6 ms (MAD 0.7 ms) | 25.9 ms (MAD 0.8 ms) | 17.1 MiB (MAD 0.3 MiB) | 72.8 MiB (MAD 0.1 MiB) | 266.3 ms (MAD 6.3 ms) | 118.0 ms (MAD 3.5 ms) |
| duckdb-parquet-zstd | yes | 6.698 MB | 344.6 ms (MAD 7.0 ms) | 130.8 ms (MAD 3.2 ms) | 136.5 ms (MAD 4.8 ms) | 23.2 ms (MAD 0.6 ms) | 25.0 ms (MAD 1.1 ms) | 18.5 MiB (MAD 0.2 MiB) | 73.0 MiB (MAD 0.2 MiB) | 332.2 ms (MAD 6.5 ms) | 134.4 ms (MAD 3.5 ms) |
| gqodb | yes | 6.512 MB | 225.2 ms (MAD 4.3 ms) | 65.5 ms (MAD 1.7 ms) | 70.3 ms (MAD 1.2 ms) | 3.2 ms (MAD 0.1 ms) | 4.0 ms (MAD 0.1 ms) | 72.6 MiB (MAD 0.1 MiB) | 68.1 MiB (MAD 0.1 MiB) | 215.3 ms (MAD 5.0 ms) | 65.0 ms (MAD 2.1 ms) |
| parquet-delta-bss-brotli5 | yes | 10.719 MB | 1,332.1 ms (MAD 31.2 ms) | 304.5 ms (MAD 4.8 ms) | 300.7 ms (MAD 6.5 ms) | 12.1 ms (MAD 0.1 ms) | 12.9 ms (MAD 0.2 ms) | 87.6 MiB (MAD 0.1 MiB) | 72.9 MiB (MAD 0.0 MiB) | 1,311.6 ms (MAD 29.7 ms) | 303.2 ms (MAD 5.0 ms) |
| parquet-delta-bss-lz4 | yes | 14.190 MB | 256.8 ms (MAD 8.1 ms) | 81.2 ms (MAD 1.1 ms) | 86.6 ms (MAD 1.9 ms) | 4.1 ms (MAD 0.1 ms) | 5.4 ms (MAD 0.1 ms) | 72.0 MiB (MAD 0.1 MiB) | 69.0 MiB (MAD 0.1 MiB) | 241.4 ms (MAD 7.2 ms) | 79.7 ms (MAD 1.5 ms) |
| parquet-delta-bss-snappy | yes | 14.014 MB | 253.6 ms (MAD 6.2 ms) | 83.7 ms (MAD 2.0 ms) | 89.8 ms (MAD 2.2 ms) | 4.3 ms (MAD 0.1 ms) | 5.4 ms (MAD 0.1 ms) | 72.1 MiB (MAD 0.1 MiB) | 68.8 MiB (MAD 0.1 MiB) | 239.0 ms (MAD 5.0 ms) | 82.2 ms (MAD 2.2 ms) |
| parquet-dictionary-lz4 | yes | 28.546 MB | 533.5 ms (MAD 9.6 ms) | 101.8 ms (MAD 3.1 ms) | 107.2 ms (MAD 3.6 ms) | 4.9 ms (MAD 0.1 ms) | 6.1 ms (MAD 0.1 ms) | 76.1 MiB (MAD 0.1 MiB) | 70.6 MiB (MAD 0.1 MiB) | 504.7 ms (MAD 12.8 ms) | 100.8 ms (MAD 2.9 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.279 [0.279, 0.279] BETTER | 0.483 [0.473, 0.501] BETTER | 0.285 [0.279, 0.295] BETTER | 0.296 [0.277, 0.302] BETTER | 0.509 [0.268, 19.898] INCONCLUSIVE | 0.409 [0.281, 0.479] BETTER | 0.296 [0.287, 0.300] BETTER | 0.896 [0.896, 0.898] BETTER | 0.530 [0.526, 0.554] BETTER | 0.221 [0.213, 0.224] BETTER |
| clickhouse-tuned | 0.705 [0.705, 0.705] BETTER | 0.439 [0.424, 0.457] BETTER | 0.246 [0.240, 0.252] BETTER | 0.266 [0.260, 0.269] BETTER | 0.587 [0.285, 0.715] BETTER | 0.637 [0.370, 0.735] BETTER | 0.327 [0.323, 0.332] BETTER | 0.873 [0.870, 0.874] BETTER | 0.470 [0.460, 0.487] BETTER | 0.195 [0.186, 0.200] BETTER |
| duckdb-native | 0.689 [0.689, 0.689] BETTER | 0.408 [0.406, 0.421] BETTER | 0.635 [0.624, 0.666] BETTER | 0.612 [0.607, 0.629] BETTER | 0.158 [0.147, 0.166] BETTER | 0.162 [0.148, 0.170] BETTER | 0.916 [0.914, 0.920] EQUIVALENT | 0.862 [0.861, 0.862] BETTER | 0.406 [0.401, 0.420] BETTER | 0.648 [0.613, 0.659] BETTER |
| duckdb-parquet-default | 0.308 [0.308, 0.308] BETTER | 0.738 [0.726, 0.758] BETTER | 0.463 [0.452, 0.473] BETTER | 0.449 [0.438, 0.454] BETTER | 0.116 [0.114, 0.120] BETTER | 0.131 [0.128, 0.135] BETTER | 2.216 [2.105, 2.247] WORSE | 0.897 [0.895, 0.900] BETTER | 0.751 [0.743, 0.781] BETTER | 0.451 [0.429, 0.458] BETTER |
| duckdb-parquet-matched | 0.809 [0.809, 0.809] BETTER | 0.810 [0.790, 0.835] BETTER | 0.549 [0.536, 0.561] BETTER | 0.563 [0.558, 0.582] BETTER | 0.135 [0.131, 0.139] BETTER | 0.153 [0.151, 0.160] BETTER | 4.244 [4.119, 4.299] WORSE | 0.936 [0.934, 0.937] EQUIVALENT | 0.808 [0.800, 0.834] BETTER | 0.551 [0.522, 0.556] BETTER |
| duckdb-parquet-zstd | 0.972 [0.972, 0.972] BETTER | 0.654 [0.643, 0.674] BETTER | 0.501 [0.483, 0.513] BETTER | 0.515 [0.508, 0.538] BETTER | 0.137 [0.132, 0.143] BETTER | 0.159 [0.150, 0.164] BETTER | 3.925 [3.895, 3.945] WORSE | 0.933 [0.930, 0.935] EQUIVALENT | 0.648 [0.630, 0.671] BETTER | 0.484 [0.463, 0.497] BETTER |
| parquet-delta-bss-brotli5 | 0.608 [0.608, 0.608] BETTER | 0.169 [0.168, 0.173] BETTER | 0.215 [0.212, 0.221] BETTER | 0.234 [0.229, 0.238] BETTER | 0.263 [0.259, 0.271] BETTER | 0.306 [0.302, 0.313] BETTER | 0.828 [0.825, 0.829] BETTER | 0.935 [0.934, 0.936] EQUIVALENT | 0.164 [0.163, 0.168] BETTER | 0.215 [0.207, 0.219] BETTER |
| parquet-delta-bss-lz4 | 0.459 [0.459, 0.459] BETTER | 0.877 [0.867, 0.903] BETTER | 0.806 [0.791, 0.828] BETTER | 0.811 [0.798, 0.823] BETTER | 0.772 [0.757, 0.788] BETTER | 0.741 [0.729, 0.758] BETTER | 1.008 [1.006, 1.009] EQUIVALENT | 0.988 [0.986, 0.989] EQUIVALENT | 0.892 [0.880, 0.924] BETTER | 0.816 [0.780, 0.832] BETTER |
| parquet-delta-bss-snappy | 0.465 [0.465, 0.465] BETTER | 0.888 [0.879, 0.907] BETTER | 0.782 [0.772, 0.812] BETTER | 0.782 [0.771, 0.797] BETTER | 0.733 [0.725, 0.753] BETTER | 0.738 [0.730, 0.766] BETTER | 1.007 [1.003, 1.008] EQUIVALENT | 0.990 [0.989, 0.992] EQUIVALENT | 0.901 [0.893, 0.924] BETTER | 0.791 [0.771, 0.813] BETTER |
| parquet-dictionary-lz4 | 0.228 [0.228, 0.228] BETTER | 0.422 [0.415, 0.432] BETTER | 0.643 [0.633, 0.659] BETTER | 0.655 [0.643, 0.676] BETTER | 0.644 [0.631, 0.662] BETTER | 0.648 [0.641, 0.672] BETTER | 0.954 [0.951, 0.954] EQUIVALENT | 0.965 [0.965, 0.967] EQUIVALENT | 0.427 [0.416, 0.437] BETTER | 0.645 [0.624, 0.654] BETTER |

## D1/v2

Valid rounds 15 of 32 attempted; flagged rounds: 17.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.155 MB | 456.1 ms (MAD 8.6 ms) | 218.0 ms (MAD 6.1 ms) | 225.4 ms (MAD 6.0 ms) | 3.9 ms (MAD 3.9 ms) | 8.4 ms (MAD 4.6 ms) | 242.6 MiB (MAD 5.0 MiB) | 75.7 MiB (MAD 0.2 MiB) | 386.7 ms (MAD 5.5 ms) | 282.0 ms (MAD 3.8 ms) |
| clickhouse-tuned | yes | 7.423 MB | 486.0 ms (MAD 7.6 ms) | 256.9 ms (MAD 5.9 ms) | 264.0 ms (MAD 5.4 ms) | 6.6 ms (MAD 3.3 ms) | 13.4 ms (MAD 3.1 ms) | 222.3 MiB (MAD 2.3 MiB) | 78.0 MiB (MAD 0.1 MiB) | 425.6 ms (MAD 4.4 ms) | 320.7 ms (MAD 5.5 ms) |
| duckdb-native | yes | 13.906 MB | 488.4 ms (MAD 4.0 ms) | 103.1 ms (MAD 0.6 ms) | 118.5 ms (MAD 1.1 ms) | 20.2 ms (MAD 0.6 ms) | 23.6 ms (MAD 0.3 ms) | 81.7 MiB (MAD 0.8 MiB) | 83.5 MiB (MAD 0.4 MiB) | 467.8 ms (MAD 3.2 ms) | 102.2 ms (MAD 2.3 ms) |
| duckdb-parquet-default | yes | 21.321 MB | 278.0 ms (MAD 3.2 ms) | 140.1 ms (MAD 2.1 ms) | 153.1 ms (MAD 1.3 ms) | 25.9 ms (MAD 0.9 ms) | 30.2 ms (MAD 1.0 ms) | 32.8 MiB (MAD 0.4 MiB) | 75.9 MiB (MAD 0.1 MiB) | 255.5 ms (MAD 2.2 ms) | 138.7 ms (MAD 3.1 ms) |
| duckdb-parquet-matched | yes | 8.067 MB | 246.5 ms (MAD 1.2 ms) | 115.8 ms (MAD 0.6 ms) | 120.5 ms (MAD 1.5 ms) | 21.9 ms (MAD 0.6 ms) | 25.4 ms (MAD 0.6 ms) | 16.9 MiB (MAD 0.3 MiB) | 72.8 MiB (MAD 0.3 MiB) | 237.8 ms (MAD 3.3 ms) | 116.1 ms (MAD 3.0 ms) |
| duckdb-parquet-zstd | yes | 6.673 MB | 308.2 ms (MAD 2.2 ms) | 127.2 ms (MAD 0.6 ms) | 131.1 ms (MAD 0.7 ms) | 21.6 ms (MAD 0.5 ms) | 25.1 ms (MAD 0.3 ms) | 17.9 MiB (MAD 0.2 MiB) | 72.9 MiB (MAD 0.2 MiB) | 298.7 ms (MAD 2.1 ms) | 123.3 ms (MAD 2.0 ms) |
| gqodb | yes | 6.512 MB | 232.9 ms (MAD 1.2 ms) | 60.8 ms (MAD 1.0 ms) | 65.4 ms (MAD 0.3 ms) | 2.8 ms (MAD 0.1 ms) | 3.8 ms (MAD 0.1 ms) | 72.4 MiB (MAD 0.0 MiB) | 67.9 MiB (MAD 0.0 MiB) | 223.8 ms (MAD 0.8 ms) | 59.9 ms (MAD 1.1 ms) |
| parquet-delta-bss-brotli5 | yes | 7.241 MB | 901.6 ms (MAD 3.7 ms) | 214.6 ms (MAD 1.7 ms) | 216.1 ms (MAD 0.8 ms) | 8.9 ms (MAD 0.2 ms) | 9.9 ms (MAD 0.2 ms) | 87.3 MiB (MAD 0.1 MiB) | 72.7 MiB (MAD 0.0 MiB) | 891.7 ms (MAD 3.7 ms) | 213.8 ms (MAD 1.1 ms) |
| parquet-delta-bss-lz4 | yes | 8.928 MB | 226.0 ms (MAD 2.4 ms) | 64.0 ms (MAD 0.3 ms) | 66.1 ms (MAD 0.8 ms) | 3.2 ms (MAD 0.1 ms) | 4.2 ms (MAD 0.1 ms) | 70.5 MiB (MAD 0.1 MiB) | 69.6 MiB (MAD 0.1 MiB) | 214.6 ms (MAD 1.7 ms) | 62.5 ms (MAD 0.4 ms) |
| parquet-delta-bss-snappy | yes | 8.872 MB | 219.7 ms (MAD 1.2 ms) | 66.2 ms (MAD 0.5 ms) | 67.6 ms (MAD 0.6 ms) | 3.4 ms (MAD 0.1 ms) | 4.5 ms (MAD 0.2 ms) | 70.6 MiB (MAD 0.1 MiB) | 69.8 MiB (MAD 0.0 MiB) | 209.7 ms (MAD 1.4 ms) | 64.9 ms (MAD 0.4 ms) |
| parquet-dictionary-lz4 | yes | 28.873 MB | 514.9 ms (MAD 10.6 ms) | 97.6 ms (MAD 0.7 ms) | 103.0 ms (MAD 1.3 ms) | 4.8 ms (MAD 0.1 ms) | 6.1 ms (MAD 0.1 ms) | 76.7 MiB (MAD 0.2 MiB) | 70.3 MiB (MAD 0.1 MiB) | 489.2 ms (MAD 10.3 ms) | 96.1 ms (MAD 0.8 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.281 [0.281, 0.281] BETTER | 0.511 [0.505, 0.525] BETTER | 0.279 [0.269, 0.286] BETTER | 0.290 [0.288, 0.298] BETTER | 0.711 [0.403, inf] INCONCLUSIVE | 0.445 [0.215, 0.468] BETTER | 0.299 [0.293, 0.302] BETTER | 0.896 [0.896, 0.899] BETTER | 0.579 [0.571, 0.584] BETTER | 0.212 [0.210, 0.217] BETTER |
| clickhouse-tuned | 0.877 [0.877, 0.877] BETTER | 0.479 [0.474, 0.485] BETTER | 0.237 [0.232, 0.242] BETTER | 0.248 [0.245, 0.259] BETTER | 0.426 [0.353, 0.605] BETTER | 0.281 [0.250, 0.340] BETTER | 0.326 [0.323, 0.329] BETTER | 0.870 [0.869, 0.871] BETTER | 0.526 [0.519, 0.528] BETTER | 0.187 [0.184, 0.190] BETTER |
| duckdb-native | 0.468 [0.468, 0.468] BETTER | 0.477 [0.469, 0.482] BETTER | 0.589 [0.586, 0.602] BETTER | 0.552 [0.548, 0.556] BETTER | 0.139 [0.135, 0.144] BETTER | 0.159 [0.156, 0.161] BETTER | 0.886 [0.886, 0.891] BETTER | 0.813 [0.811, 0.816] BETTER | 0.478 [0.473, 0.481] BETTER | 0.586 [0.562, 0.604] BETTER |
| duckdb-parquet-default | 0.305 [0.305, 0.305] BETTER | 0.838 [0.833, 0.848] BETTER | 0.434 [0.430, 0.440] BETTER | 0.427 [0.423, 0.430] BETTER | 0.108 [0.106, 0.113] BETTER | 0.125 [0.120, 0.128] BETTER | 2.211 [2.187, 2.238] WORSE | 0.894 [0.892, 0.895] BETTER | 0.876 [0.871, 0.881] BETTER | 0.432 [0.425, 0.446] BETTER |
| duckdb-parquet-matched | 0.807 [0.807, 0.807] BETTER | 0.945 [0.938, 0.952] BETTER | 0.525 [0.521, 0.536] BETTER | 0.543 [0.536, 0.548] BETTER | 0.128 [0.127, 0.133] BETTER | 0.148 [0.145, 0.153] BETTER | 4.292 [4.249, 4.343] WORSE | 0.932 [0.928, 0.935] EQUIVALENT | 0.941 [0.936, 0.958] BETTER | 0.516 [0.507, 0.531] BETTER |
| duckdb-parquet-zstd | 0.976 [0.976, 0.976] BETTER | 0.756 [0.750, 0.759] BETTER | 0.478 [0.474, 0.490] BETTER | 0.499 [0.496, 0.502] BETTER | 0.130 [0.126, 0.134] BETTER | 0.150 [0.147, 0.154] BETTER | 4.043 [4.002, 4.087] WORSE | 0.931 [0.929, 0.933] EQUIVALENT | 0.749 [0.744, 0.755] BETTER | 0.486 [0.479, 0.497] BETTER |
| parquet-delta-bss-brotli5 | 0.899 [0.899, 0.899] BETTER | 0.258 [0.257, 0.259] BETTER | 0.283 [0.280, 0.289] BETTER | 0.303 [0.299, 0.304] BETTER | 0.314 [0.306, 0.323] BETTER | 0.379 [0.371, 0.389] BETTER | 0.830 [0.829, 0.831] BETTER | 0.934 [0.933, 0.935] EQUIVALENT | 0.251 [0.250, 0.252] BETTER | 0.280 [0.277, 0.285] BETTER |
| parquet-delta-bss-lz4 | 0.729 [0.729, 0.729] BETTER | 1.031 [1.020, 1.042] EQUIVALENT | 0.950 [0.944, 0.976] BETTER | 0.989 [0.983, 1.002] EQUIVALENT | 0.871 [0.856, 0.892] BETTER | 0.892 [0.867, 0.902] BETTER | 1.028 [1.027, 1.029] EQUIVALENT | 0.975 [0.974, 0.976] EQUIVALENT | 1.043 [1.038, 1.050] INCONCLUSIVE | 0.958 [0.940, 0.980] INCONCLUSIVE |
| parquet-delta-bss-snappy | 0.734 [0.734, 0.734] BETTER | 1.060 [1.045, 1.062] WORSE | 0.918 [0.910, 0.936] BETTER | 0.967 [0.959, 0.971] EQUIVALENT | 0.831 [0.803, 0.868] BETTER | 0.830 [0.799, 0.859] BETTER | 1.026 [1.025, 1.027] EQUIVALENT | 0.973 [0.972, 0.974] EQUIVALENT | 1.067 [1.059, 1.070] WORSE | 0.923 [0.911, 0.937] BETTER |
| parquet-dictionary-lz4 | 0.226 [0.226, 0.226] BETTER | 0.452 [0.440, 0.459] BETTER | 0.623 [0.618, 0.638] BETTER | 0.635 [0.623, 0.639] BETTER | 0.583 [0.570, 0.598] BETTER | 0.614 [0.596, 0.625] BETTER | 0.944 [0.944, 0.964] EQUIVALENT | 0.965 [0.964, 0.966] EQUIVALENT | 0.457 [0.445, 0.465] BETTER | 0.623 [0.612, 0.634] BETTER |

## D2/v1

Valid rounds 11 of 40 attempted; flagged rounds: 29; **UNDERPOWERED**.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 24.169 MB | 484.8 ms (MAD 8.0 ms) | 241.6 ms (MAD 4.3 ms) | 252.9 ms (MAD 8.1 ms) | 9.9 ms (MAD 4.8 ms) | 5.4 ms (MAD 3.5 ms) | 241.1 MiB (MAD 2.0 MiB) | 76.0 MiB (MAD 0.0 MiB) | 412.5 ms (MAD 8.4 ms) | 306.7 ms (MAD 3.3 ms) |
| clickhouse-tuned | yes | 10.296 MB | 527.0 ms (MAD 13.0 ms) | 279.9 ms (MAD 4.3 ms) | 284.6 ms (MAD 6.2 ms) | 9.8 ms (MAD 3.5 ms) | 10.0 ms (MAD 3.8 ms) | 223.8 MiB (MAD 3.1 MiB) | 78.0 MiB (MAD 0.2 MiB) | 472.3 ms (MAD 7.4 ms) | 346.5 ms (MAD 7.0 ms) |
| duckdb-native | yes | 9.187 MB | 559.2 ms (MAD 2.5 ms) | 104.0 ms (MAD 1.0 ms) | 114.5 ms (MAD 1.3 ms) | 20.4 ms (MAD 0.7 ms) | 23.7 ms (MAD 0.5 ms) | 79.1 MiB (MAD 0.4 MiB) | 78.9 MiB (MAD 0.2 MiB) | 540.3 ms (MAD 2.0 ms) | 101.7 ms (MAD 2.8 ms) |
| duckdb-parquet-default | yes | 21.386 MB | 322.0 ms (MAD 2.5 ms) | 144.2 ms (MAD 2.1 ms) | 158.6 ms (MAD 1.7 ms) | 27.9 ms (MAD 0.8 ms) | 31.2 ms (MAD 0.6 ms) | 33.1 MiB (MAD 0.6 MiB) | 76.1 MiB (MAD 0.1 MiB) | 298.7 ms (MAD 2.7 ms) | 142.4 ms (MAD 4.5 ms) |
| duckdb-parquet-matched | yes | 10.072 MB | 292.0 ms (MAD 2.1 ms) | 123.1 ms (MAD 1.1 ms) | 127.7 ms (MAD 1.2 ms) | 22.2 ms (MAD 0.8 ms) | 25.8 ms (MAD 0.8 ms) | 17.4 MiB (MAD 0.1 MiB) | 73.3 MiB (MAD 0.1 MiB) | 278.4 ms (MAD 4.8 ms) | 122.6 ms (MAD 4.1 ms) |
| duckdb-parquet-zstd | yes | 7.629 MB | 382.4 ms (MAD 1.3 ms) | 137.1 ms (MAD 1.7 ms) | 141.7 ms (MAD 0.6 ms) | 22.4 ms (MAD 0.5 ms) | 27.1 ms (MAD 0.7 ms) | 18.9 MiB (MAD 0.3 MiB) | 72.8 MiB (MAD 0.2 MiB) | 372.7 ms (MAD 4.9 ms) | 136.0 ms (MAD 4.3 ms) |
| gqodb | yes | 6.212 MB | 228.5 ms (MAD 2.5 ms) | 66.5 ms (MAD 0.4 ms) | 69.8 ms (MAD 0.5 ms) | 3.2 ms (MAD 0.0 ms) | 4.0 ms (MAD 0.0 ms) | 72.5 MiB (MAD 0.1 MiB) | 68.1 MiB (MAD 0.1 MiB) | 219.9 ms (MAD 1.7 ms) | 65.4 ms (MAD 0.5 ms) |
| parquet-delta-bss-brotli5 | yes | 11.261 MB | 1,454.7 ms (MAD 9.9 ms) | 313.6 ms (MAD 1.6 ms) | 317.7 ms (MAD 1.7 ms) | 12.4 ms (MAD 0.1 ms) | 13.5 ms (MAD 0.1 ms) | 87.7 MiB (MAD 0.1 MiB) | 72.9 MiB (MAD 0.1 MiB) | 1,440.4 ms (MAD 8.3 ms) | 312.1 ms (MAD 1.1 ms) |
| parquet-delta-bss-lz4 | yes | 15.348 MB | 253.7 ms (MAD 5.1 ms) | 77.8 ms (MAD 0.2 ms) | 85.1 ms (MAD 1.1 ms) | 4.1 ms (MAD 0.0 ms) | 5.1 ms (MAD 0.0 ms) | 71.8 MiB (MAD 0.1 MiB) | 69.1 MiB (MAD 0.0 MiB) | 235.0 ms (MAD 3.2 ms) | 76.6 ms (MAD 0.7 ms) |
| parquet-delta-bss-snappy | yes | 15.256 MB | 250.0 ms (MAD 1.4 ms) | 81.5 ms (MAD 0.6 ms) | 88.2 ms (MAD 0.6 ms) | 4.1 ms (MAD 0.0 ms) | 5.1 ms (MAD 0.1 ms) | 72.0 MiB (MAD 0.1 MiB) | 68.8 MiB (MAD 0.1 MiB) | 234.9 ms (MAD 1.2 ms) | 80.6 ms (MAD 0.5 ms) |
| parquet-dictionary-lz4 | yes | 28.791 MB | 517.8 ms (MAD 4.7 ms) | 101.8 ms (MAD 1.0 ms) | 107.2 ms (MAD 0.8 ms) | 4.9 ms (MAD 0.1 ms) | 6.3 ms (MAD 0.1 ms) | 78.1 MiB (MAD 0.1 MiB) | 70.1 MiB (MAD 0.1 MiB) | 489.5 ms (MAD 4.3 ms) | 100.5 ms (MAD 1.4 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.257 [0.257, 0.257] BETTER | 0.471 [0.468, 0.480] BETTER | 0.275 [0.268, 0.290] BETTER | 0.276 [0.275, 0.285] BETTER | 0.325 [0.194, 0.582] BETTER | 0.736 [0.645, 1.290] INCONCLUSIVE | 0.301 [0.292, 0.302] BETTER | 0.897 [0.896, 0.897] BETTER | 0.533 [0.521, 0.541] BETTER | 0.213 [0.212, 0.222] BETTER |
| clickhouse-tuned | 0.603 [0.603, 0.603] BETTER | 0.434 [0.431, 0.444] BETTER | 0.237 [0.228, 0.244] BETTER | 0.245 [0.240, 0.251] BETTER | 0.328 [0.277, 0.711] BETTER | 0.402 [0.317, 0.778] BETTER | 0.324 [0.321, 0.328] BETTER | 0.873 [0.872, 0.876] BETTER | 0.466 [0.460, 0.475] BETTER | 0.189 [0.183, 0.194] BETTER |
| duckdb-native | 0.676 [0.676, 0.676] BETTER | 0.409 [0.407, 0.417] BETTER | 0.639 [0.632, 0.650] BETTER | 0.610 [0.606, 0.622] BETTER | 0.158 [0.144, 0.163] BETTER | 0.169 [0.153, 0.171] BETTER | 0.916 [0.915, 0.922] EQUIVALENT | 0.863 [0.861, 0.865] BETTER | 0.407 [0.402, 0.411] BETTER | 0.643 [0.617, 0.655] BETTER |
| duckdb-parquet-default | 0.290 [0.290, 0.290] BETTER | 0.710 [0.704, 0.722] BETTER | 0.461 [0.451, 0.476] BETTER | 0.440 [0.433, 0.444] BETTER | 0.116 [0.114, 0.119] BETTER | 0.128 [0.125, 0.131] BETTER | 2.187 [2.121, 2.210] WORSE | 0.895 [0.894, 0.898] BETTER | 0.736 [0.723, 0.741] BETTER | 0.459 [0.444, 0.477] BETTER |
| duckdb-parquet-matched | 0.617 [0.617, 0.617] BETTER | 0.782 [0.774, 0.798] BETTER | 0.540 [0.532, 0.548] BETTER | 0.547 [0.532, 0.552] BETTER | 0.145 [0.136, 0.148] BETTER | 0.155 [0.148, 0.160] BETTER | 4.175 [4.148, 4.211] WORSE | 0.930 [0.929, 0.933] EQUIVALENT | 0.790 [0.776, 0.801] BETTER | 0.533 [0.530, 0.558] BETTER |
| duckdb-parquet-zstd | 0.814 [0.814, 0.814] BETTER | 0.598 [0.589, 0.605] BETTER | 0.485 [0.481, 0.490] BETTER | 0.493 [0.490, 0.495] BETTER | 0.144 [0.135, 0.146] BETTER | 0.148 [0.145, 0.152] BETTER | 3.842 [3.709, 3.941] WORSE | 0.936 [0.933, 0.938] EQUIVALENT | 0.590 [0.582, 0.596] BETTER | 0.481 [0.465, 0.490] BETTER |
| parquet-delta-bss-brotli5 | 0.552 [0.552, 0.552] BETTER | 0.157 [0.156, 0.159] BETTER | 0.212 [0.210, 0.215] BETTER | 0.220 [0.215, 0.221] BETTER | 0.259 [0.256, 0.261] BETTER | 0.296 [0.291, 0.304] BETTER | 0.826 [0.825, 0.827] BETTER | 0.935 [0.934, 0.935] EQUIVALENT | 0.153 [0.151, 0.154] BETTER | 0.209 [0.209, 0.215] BETTER |
| parquet-delta-bss-lz4 | 0.405 [0.405, 0.405] BETTER | 0.901 [0.888, 0.921] BETTER | 0.855 [0.849, 0.866] BETTER | 0.820 [0.816, 0.843] BETTER | 0.784 [0.776, 0.792] BETTER | 0.786 [0.776, 0.793] BETTER | 1.009 [1.008, 1.010] EQUIVALENT | 0.986 [0.985, 0.987] EQUIVALENT | 0.936 [0.923, 0.948] BETTER | 0.853 [0.845, 0.879] BETTER |
| parquet-delta-bss-snappy | 0.407 [0.407, 0.407] BETTER | 0.914 [0.901, 0.925] BETTER | 0.816 [0.795, 0.829] BETTER | 0.792 [0.784, 0.799] BETTER | 0.786 [0.781, 0.791] BETTER | 0.788 [0.777, 0.794] BETTER | 1.006 [1.004, 1.007] EQUIVALENT | 0.991 [0.989, 0.991] EQUIVALENT | 0.936 [0.929, 0.941] BETTER | 0.811 [0.774, 0.835] BETTER |
| parquet-dictionary-lz4 | 0.216 [0.216, 0.216] BETTER | 0.441 [0.437, 0.447] BETTER | 0.653 [0.646, 0.659] BETTER | 0.651 [0.646, 0.660] BETTER | 0.652 [0.641, 0.659] BETTER | 0.639 [0.624, 0.644] BETTER | 0.928 [0.927, 0.929] EQUIVALENT | 0.972 [0.969, 0.974] EQUIVALENT | 0.449 [0.445, 0.454] BETTER | 0.650 [0.641, 0.662] BETTER |

## D2/v2

Valid rounds 12 of 40 attempted; flagged rounds: 28; **UNDERPOWERED**.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.914 MB | 470.2 ms (MAD 8.7 ms) | 224.0 ms (MAD 6.5 ms) | 236.1 ms (MAD 4.5 ms) | 8.1 ms (MAD 3.3 ms) | 8.7 ms (MAD 3.7 ms) | 241.4 MiB (MAD 2.1 MiB) | 75.6 MiB (MAD 0.0 MiB) | 396.0 ms (MAD 12.8 ms) | 291.8 ms (MAD 7.7 ms) |
| clickhouse-tuned | yes | 7.594 MB | 504.7 ms (MAD 6.0 ms) | 271.7 ms (MAD 9.7 ms) | 272.5 ms (MAD 8.1 ms) | 6.1 ms (MAD 3.8 ms) | 10.5 ms (MAD 4.3 ms) | 222.9 MiB (MAD 4.3 MiB) | 77.9 MiB (MAD 0.1 MiB) | 448.1 ms (MAD 8.6 ms) | 338.3 ms (MAD 10.2 ms) |
| duckdb-native | yes | 13.644 MB | 493.8 ms (MAD 6.9 ms) | 105.2 ms (MAD 3.2 ms) | 120.5 ms (MAD 1.0 ms) | 20.8 ms (MAD 0.4 ms) | 24.5 ms (MAD 0.5 ms) | 81.3 MiB (MAD 0.6 MiB) | 83.1 MiB (MAD 0.1 MiB) | 472.1 ms (MAD 7.9 ms) | 101.6 ms (MAD 6.6 ms) |
| duckdb-parquet-default | yes | 21.679 MB | 292.8 ms (MAD 4.8 ms) | 143.1 ms (MAD 2.5 ms) | 156.1 ms (MAD 0.6 ms) | 27.3 ms (MAD 0.9 ms) | 30.4 ms (MAD 0.9 ms) | 33.0 MiB (MAD 0.5 MiB) | 76.1 MiB (MAD 0.3 MiB) | 268.5 ms (MAD 8.5 ms) | 143.2 ms (MAD 5.4 ms) |
| duckdb-parquet-matched | yes | 8.297 MB | 253.8 ms (MAD 3.2 ms) | 121.4 ms (MAD 3.6 ms) | 121.4 ms (MAD 1.7 ms) | 23.1 ms (MAD 0.7 ms) | 25.9 ms (MAD 0.7 ms) | 16.8 MiB (MAD 0.3 MiB) | 72.6 MiB (MAD 0.3 MiB) | 245.8 ms (MAD 4.7 ms) | 121.0 ms (MAD 6.1 ms) |
| duckdb-parquet-zstd | yes | 6.695 MB | 320.4 ms (MAD 5.9 ms) | 130.2 ms (MAD 1.6 ms) | 133.9 ms (MAD 2.0 ms) | 22.4 ms (MAD 0.6 ms) | 26.6 ms (MAD 1.1 ms) | 17.9 MiB (MAD 0.3 MiB) | 72.7 MiB (MAD 0.1 MiB) | 307.6 ms (MAD 5.0 ms) | 129.0 ms (MAD 4.9 ms) |
| gqodb | yes | 6.212 MB | 235.0 ms (MAD 3.8 ms) | 62.2 ms (MAD 1.1 ms) | 66.0 ms (MAD 1.2 ms) | 2.9 ms (MAD 0.1 ms) | 3.7 ms (MAD 0.1 ms) | 72.3 MiB (MAD 0.1 MiB) | 68.9 MiB (MAD 0.1 MiB) | 227.1 ms (MAD 4.1 ms) | 61.3 ms (MAD 0.9 ms) |
| parquet-delta-bss-brotli5 | yes | 7.463 MB | 945.8 ms (MAD 5.6 ms) | 219.6 ms (MAD 2.7 ms) | 221.6 ms (MAD 1.2 ms) | 9.3 ms (MAD 0.1 ms) | 10.3 ms (MAD 0.3 ms) | 87.3 MiB (MAD 0.2 MiB) | 73.1 MiB (MAD 0.1 MiB) | 936.8 ms (MAD 5.5 ms) | 218.2 ms (MAD 2.9 ms) |
| parquet-delta-bss-lz4 | yes | 9.570 MB | 230.3 ms (MAD 3.1 ms) | 66.2 ms (MAD 1.2 ms) | 68.1 ms (MAD 1.0 ms) | 3.3 ms (MAD 0.1 ms) | 4.3 ms (MAD 0.1 ms) | 70.5 MiB (MAD 0.1 MiB) | 69.6 MiB (MAD 0.1 MiB) | 218.4 ms (MAD 2.0 ms) | 65.1 ms (MAD 0.9 ms) |
| parquet-delta-bss-snappy | yes | 9.206 MB | 229.1 ms (MAD 2.9 ms) | 68.0 ms (MAD 1.3 ms) | 71.1 ms (MAD 1.5 ms) | 3.4 ms (MAD 0.1 ms) | 4.4 ms (MAD 0.1 ms) | 70.7 MiB (MAD 0.2 MiB) | 69.9 MiB (MAD 0.0 MiB) | 218.1 ms (MAD 3.3 ms) | 67.1 ms (MAD 1.8 ms) |
| parquet-dictionary-lz4 | yes | 29.268 MB | 506.8 ms (MAD 2.6 ms) | 100.0 ms (MAD 1.4 ms) | 105.5 ms (MAD 1.7 ms) | 4.9 ms (MAD 0.1 ms) | 6.4 ms (MAD 0.2 ms) | 78.7 MiB (MAD 0.2 MiB) | 70.4 MiB (MAD 0.1 MiB) | 480.8 ms (MAD 4.1 ms) | 98.3 ms (MAD 1.1 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.260 [0.260, 0.260] BETTER | 0.500 [0.494, 0.514] BETTER | 0.278 [0.267, 0.286] BETTER | 0.280 [0.275, 0.285] BETTER | 0.356 [0.290, 0.516] BETTER | 0.428 [0.251, 0.556] BETTER | 0.300 [0.296, 0.301] BETTER | 0.911 [0.909, 0.912] EQUIVALENT | 0.573 [0.563, 0.585] BETTER | 0.210 [0.206, 0.215] BETTER |
| clickhouse-tuned | 0.818 [0.818, 0.818] BETTER | 0.466 [0.455, 0.472] BETTER | 0.229 [0.222, 0.237] BETTER | 0.242 [0.239, 0.249] BETTER | 0.476 [0.294, 1.329] INCONCLUSIVE | 0.355 [0.308, 0.465] BETTER | 0.325 [0.320, 0.350] BETTER | 0.884 [0.882, 0.887] BETTER | 0.507 [0.488, 0.510] BETTER | 0.181 [0.177, 0.186] BETTER |
| duckdb-native | 0.455 [0.455, 0.455] BETTER | 0.476 [0.470, 0.485] BETTER | 0.592 [0.575, 0.606] BETTER | 0.548 [0.542, 0.557] BETTER | 0.139 [0.134, 0.143] BETTER | 0.152 [0.149, 0.155] BETTER | 0.889 [0.883, 0.892] BETTER | 0.828 [0.826, 0.829] BETTER | 0.481 [0.472, 0.491] BETTER | 0.603 [0.573, 0.618] BETTER |
| duckdb-parquet-default | 0.287 [0.287, 0.287] BETTER | 0.803 [0.794, 0.816] BETTER | 0.435 [0.427, 0.441] BETTER | 0.423 [0.415, 0.428] BETTER | 0.106 [0.100, 0.111] BETTER | 0.122 [0.115, 0.125] BETTER | 2.192 [2.183, 2.226] WORSE | 0.905 [0.902, 0.909] EQUIVALENT | 0.846 [0.832, 0.865] BETTER | 0.428 [0.421, 0.461] BETTER |
| duckdb-parquet-matched | 0.749 [0.749, 0.749] BETTER | 0.926 [0.916, 0.946] BETTER | 0.513 [0.501, 0.536] BETTER | 0.544 [0.533, 0.549] BETTER | 0.125 [0.123, 0.129] BETTER | 0.143 [0.138, 0.148] BETTER | 4.293 [4.239, 4.389] WORSE | 0.949 [0.946, 0.954] EQUIVALENT | 0.924 [0.910, 0.948] BETTER | 0.507 [0.475, 0.534] BETTER |
| duckdb-parquet-zstd | 0.928 [0.928, 0.928] BETTER | 0.734 [0.701, 0.745] BETTER | 0.478 [0.470, 0.485] BETTER | 0.493 [0.479, 0.497] BETTER | 0.129 [0.123, 0.132] BETTER | 0.140 [0.134, 0.144] BETTER | 4.043 [3.976, 4.100] WORSE | 0.948 [0.947, 0.949] EQUIVALENT | 0.738 [0.698, 0.749] BETTER | 0.476 [0.466, 0.493] BETTER |
| parquet-delta-bss-brotli5 | 0.832 [0.832, 0.832] BETTER | 0.249 [0.246, 0.252] BETTER | 0.283 [0.279, 0.283] BETTER | 0.298 [0.294, 0.302] BETTER | 0.312 [0.301, 0.320] BETTER | 0.360 [0.346, 0.372] BETTER | 0.829 [0.827, 0.830] BETTER | 0.942 [0.941, 0.943] EQUIVALENT | 0.242 [0.239, 0.247] BETTER | 0.281 [0.278, 0.282] BETTER |
| parquet-delta-bss-lz4 | 0.649 [0.649, 0.649] BETTER | 1.021 [1.012, 1.034] EQUIVALENT | 0.940 [0.926, 0.946] BETTER | 0.969 [0.966, 0.979] EQUIVALENT | 0.872 [0.846, 0.900] BETTER | 0.857 [0.795, 0.876] BETTER | 1.026 [1.024, 1.027] EQUIVALENT | 0.989 [0.988, 0.990] EQUIVALENT | 1.040 [1.025, 1.050] EQUIVALENT | 0.942 [0.930, 0.953] BETTER |
| parquet-delta-bss-snappy | 0.675 [0.675, 0.675] BETTER | 1.026 [0.987, 1.035] EQUIVALENT | 0.915 [0.887, 0.919] BETTER | 0.928 [0.909, 0.949] BETTER | 0.855 [0.825, 0.872] BETTER | 0.837 [0.811, 0.864] BETTER | 1.023 [1.019, 1.024] EQUIVALENT | 0.986 [0.985, 0.986] EQUIVALENT | 1.041 [0.999, 1.050] EQUIVALENT | 0.913 [0.884, 0.920] BETTER |
| parquet-dictionary-lz4 | 0.212 [0.212, 0.212] BETTER | 0.464 [0.460, 0.471] BETTER | 0.623 [0.610, 0.632] BETTER | 0.626 [0.622, 0.636] BETTER | 0.592 [0.580, 0.609] BETTER | 0.582 [0.562, 0.593] BETTER | 0.919 [0.917, 0.920] EQUIVALENT | 0.979 [0.978, 0.980] EQUIVALENT | 0.472 [0.465, 0.480] BETTER | 0.624 [0.609, 0.632] BETTER |

## S1/v1

Valid rounds 15 of 15 attempted; flagged rounds: 0.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 69.962 MB | 1,142.1 ms (MAD 8.9 ms) | 668.3 ms (MAD 9.2 ms) | 682.5 ms (MAD 11.0 ms) | 6.9 ms (MAD 2.9 ms) | 9.7 ms (MAD 5.6 ms) | 516.2 MiB (MAD 2.0 MiB) | 76.4 MiB (MAD 0.0 MiB) | 1,032.3 ms (MAD 8.7 ms) | 700.6 ms (MAD 12.5 ms) |
| clickhouse-tuned | yes | 27.626 MB | 1,296.7 ms (MAD 8.4 ms) | 790.4 ms (MAD 8.2 ms) | 803.1 ms (MAD 8.4 ms) | 5.9 ms (MAD 4.0 ms) | 9.8 ms (MAD 3.0 ms) | 488.9 MiB (MAD 2.0 MiB) | 78.5 MiB (MAD 0.2 MiB) | 1,217.4 ms (MAD 9.6 ms) | 824.9 ms (MAD 5.8 ms) |
| duckdb-native | yes | 27.013 MB | 1,547.4 ms (MAD 6.6 ms) | 277.6 ms (MAD 1.9 ms) | 305.0 ms (MAD 1.6 ms) | 22.2 ms (MAD 1.1 ms) | 25.3 ms (MAD 0.6 ms) | 110.0 MiB (MAD 0.6 MiB) | 218.8 MiB (MAD 0.2 MiB) | 1,513.5 ms (MAD 5.6 ms) | 275.7 ms (MAD 3.6 ms) |
| duckdb-parquet-default | yes | 63.269 MB | 880.2 ms (MAD 4.7 ms) | 392.0 ms (MAD 4.2 ms) | 430.6 ms (MAD 5.1 ms) | 30.8 ms (MAD 1.1 ms) | 33.9 ms (MAD 0.4 ms) | 44.0 MiB (MAD 0.6 MiB) | 199.1 MiB (MAD 0.1 MiB) | 825.2 ms (MAD 5.4 ms) | 390.8 ms (MAD 6.9 ms) |
| gqodb | yes | 19.505 MB | 623.8 ms (MAD 4.4 ms) | 195.4 ms (MAD 1.4 ms) | 205.5 ms (MAD 2.1 ms) | 3.4 ms (MAD 0.0 ms) | 4.3 ms (MAD 0.0 ms) | 202.0 MiB (MAD 0.1 MiB) | 195.7 MiB (MAD 0.1 MiB) | 603.3 ms (MAD 4.0 ms) | 194.1 ms (MAD 1.4 ms) |
| parquet-delta-bss-lz4 | yes | 42.597 MB | 747.6 ms (MAD 3.5 ms) | 239.2 ms (MAD 1.7 ms) | 254.8 ms (MAD 1.7 ms) | 4.5 ms (MAD 0.1 ms) | 5.6 ms (MAD 0.1 ms) | 201.1 MiB (MAD 0.1 MiB) | 197.2 MiB (MAD 0.1 MiB) | 707.1 ms (MAD 4.1 ms) | 238.6 ms (MAD 1.6 ms) |
| parquet-dictionary-lz4 | yes | 85.580 MB | 1,547.3 ms (MAD 15.5 ms) | 302.5 ms (MAD 1.7 ms) | 317.9 ms (MAD 3.7 ms) | 5.4 ms (MAD 0.1 ms) | 6.8 ms (MAD 0.1 ms) | 209.1 MiB (MAD 0.8 MiB) | 199.1 MiB (MAD 0.1 MiB) | 1,471.7 ms (MAD 14.9 ms) | 300.0 ms (MAD 1.3 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.279 [0.279, 0.279] BETTER | 0.546 [0.541, 0.555] BETTER | 0.292 [0.290, 0.298] BETTER | 0.301 [0.299, 0.306] BETTER | 0.489 [0.423, 1.270] INCONCLUSIVE | 0.446 [0.369, 0.727] BETTER | 0.391 [0.390, 0.393] BETTER | 2.560 [2.560, 2.568] WORSE | 0.584 [0.579, 0.593] BETTER | 0.277 [0.273, 0.284] BETTER |
| clickhouse-tuned | 0.706 [0.706, 0.706] BETTER | 0.481 [0.474, 0.486] BETTER | 0.247 [0.245, 0.250] BETTER | 0.256 [0.254, 0.262] BETTER | 0.574 [0.341, 0.587] BETTER | 0.442 [0.423, 0.918] BETTER | 0.413 [0.412, 0.415] BETTER | 2.494 [2.493, 2.500] WORSE | 0.496 [0.486, 0.499] BETTER | 0.235 [0.233, 0.238] BETTER |
| duckdb-native | 0.722 [0.722, 0.722] BETTER | 0.403 [0.390, 0.408] BETTER | 0.704 [0.699, 0.708] BETTER | 0.674 [0.670, 0.676] BETTER | 0.153 [0.146, 0.161] BETTER | 0.171 [0.164, 0.173] BETTER | 1.836 [1.830, 1.840] WORSE | 0.895 [0.894, 0.895] BETTER | 0.399 [0.388, 0.402] BETTER | 0.704 [0.694, 0.714] BETTER |
| duckdb-parquet-default | 0.308 [0.308, 0.308] BETTER | 0.709 [0.687, 0.712] BETTER | 0.498 [0.489, 0.504] BETTER | 0.477 [0.474, 0.484] BETTER | 0.110 [0.107, 0.110] BETTER | 0.128 [0.126, 0.130] BETTER | 4.589 [4.414, 4.629] WORSE | 0.983 [0.983, 0.983] EQUIVALENT | 0.731 [0.717, 0.738] BETTER | 0.497 [0.483, 0.506] BETTER |
| parquet-delta-bss-lz4 | 0.458 [0.458, 0.458] BETTER | 0.834 [0.815, 0.842] BETTER | 0.817 [0.805, 0.823] BETTER | 0.807 [0.801, 0.813] BETTER | 0.746 [0.730, 0.760] BETTER | 0.776 [0.767, 0.780] BETTER | 1.005 [1.004, 1.005] EQUIVALENT | 0.992 [0.992, 0.993] EQUIVALENT | 0.853 [0.837, 0.861] BETTER | 0.814 [0.810, 0.821] BETTER |
| parquet-dictionary-lz4 | 0.228 [0.228, 0.228] BETTER | 0.403 [0.389, 0.410] BETTER | 0.646 [0.640, 0.654] BETTER | 0.646 [0.639, 0.649] BETTER | 0.632 [0.621, 0.639] BETTER | 0.642 [0.633, 0.646] BETTER | 0.966 [0.964, 0.973] EQUIVALENT | 0.983 [0.983, 0.983] EQUIVALENT | 0.410 [0.396, 0.418] BETTER | 0.647 [0.641, 0.655] BETTER |

## S1/v2

Valid rounds 15 of 25 attempted; flagged rounds: 10.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 69.371 MB | 1,093.7 ms (MAD 10.3 ms) | 633.4 ms (MAD 8.2 ms) | 656.7 ms (MAD 9.1 ms) | 5.5 ms (MAD 2.3 ms) | 3.0 ms (MAD 2.9 ms) | 516.6 MiB (MAD 2.2 MiB) | 76.5 MiB (MAD 0.0 MiB) | 981.1 ms (MAD 6.7 ms) | 666.7 ms (MAD 6.0 ms) |
| clickhouse-tuned | yes | 21.936 MB | 1,221.4 ms (MAD 9.9 ms) | 754.0 ms (MAD 8.8 ms) | 762.3 ms (MAD 7.3 ms) | 6.7 ms (MAD 3.9 ms) | 4.7 ms (MAD 4.7 ms) | 509.1 MiB (MAD 4.9 MiB) | 78.4 MiB (MAD 0.2 MiB) | 1,151.0 ms (MAD 7.5 ms) | 787.9 ms (MAD 3.7 ms) |
| duckdb-native | yes | 40.645 MB | 1,338.9 ms (MAD 9.4 ms) | 272.3 ms (MAD 2.3 ms) | 320.0 ms (MAD 2.6 ms) | 20.6 ms (MAD 0.5 ms) | 23.7 ms (MAD 0.5 ms) | 123.4 MiB (MAD 0.2 MiB) | 232.0 MiB (MAD 0.2 MiB) | 1,291.8 ms (MAD 10.4 ms) | 270.9 ms (MAD 4.6 ms) |
| duckdb-parquet-default | yes | 63.907 MB | 782.3 ms (MAD 4.2 ms) | 377.7 ms (MAD 2.2 ms) | 422.4 ms (MAD 3.7 ms) | 30.5 ms (MAD 1.0 ms) | 33.5 ms (MAD 0.8 ms) | 44.9 MiB (MAD 0.9 MiB) | 199.0 MiB (MAD 0.2 MiB) | 730.5 ms (MAD 5.8 ms) | 376.4 ms (MAD 2.0 ms) |
| gqodb | yes | 19.471 MB | 642.9 ms (MAD 2.8 ms) | 179.5 ms (MAD 1.3 ms) | 191.2 ms (MAD 0.6 ms) | 3.0 ms (MAD 0.1 ms) | 3.9 ms (MAD 0.1 ms) | 201.9 MiB (MAD 0.2 MiB) | 195.9 MiB (MAD 0.0 MiB) | 621.9 ms (MAD 1.9 ms) | 178.7 ms (MAD 1.3 ms) |
| parquet-delta-bss-lz4 | yes | 26.809 MB | 657.7 ms (MAD 3.5 ms) | 189.1 ms (MAD 1.0 ms) | 193.8 ms (MAD 1.3 ms) | 3.6 ms (MAD 0.1 ms) | 4.8 ms (MAD 0.1 ms) | 201.1 MiB (MAD 0.1 MiB) | 199.6 MiB (MAD 0.1 MiB) | 632.4 ms (MAD 3.4 ms) | 188.2 ms (MAD 1.0 ms) |
| parquet-dictionary-lz4 | yes | 86.663 MB | 1,455.1 ms (MAD 5.2 ms) | 287.2 ms (MAD 0.4 ms) | 304.4 ms (MAD 1.9 ms) | 5.2 ms (MAD 0.1 ms) | 6.8 ms (MAD 0.3 ms) | 210.8 MiB (MAD 0.1 MiB) | 198.8 MiB (MAD 0.1 MiB) | 1,381.9 ms (MAD 5.8 ms) | 286.5 ms (MAD 1.8 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.281 [0.281, 0.281] BETTER | 0.588 [0.583, 0.593] BETTER | 0.283 [0.281, 0.289] BETTER | 0.291 [0.286, 0.293] BETTER | 0.551 [0.464, 0.921] BETTER | 1.311 [0.483, 1.667] INCONCLUSIVE | 0.391 [0.390, 0.392] BETTER | 2.563 [2.562, 2.563] WORSE | 0.634 [0.624, 0.636] BETTER | 0.268 [0.266, 0.273] BETTER |
| clickhouse-tuned | 0.888 [0.888, 0.888] BETTER | 0.526 [0.524, 0.530] BETTER | 0.238 [0.234, 0.245] BETTER | 0.251 [0.248, 0.253] BETTER | 0.450 [0.234, 0.658] BETTER | 0.833 [0.390, 1.868] INCONCLUSIVE | 0.397 [0.395, 0.413] BETTER | 2.499 [2.497, 2.504] WORSE | 0.540 [0.537, 0.543] BETTER | 0.227 [0.225, 0.230] BETTER |
| duckdb-native | 0.479 [0.479, 0.479] BETTER | 0.480 [0.477, 0.483] BETTER | 0.659 [0.655, 0.671] BETTER | 0.598 [0.592, 0.603] BETTER | 0.147 [0.143, 0.154] BETTER | 0.166 [0.163, 0.172] BETTER | 1.637 [1.635, 1.639] WORSE | 0.845 [0.844, 0.845] BETTER | 0.481 [0.478, 0.485] BETTER | 0.659 [0.653, 0.668] BETTER |
| duckdb-parquet-default | 0.305 [0.305, 0.305] BETTER | 0.822 [0.818, 0.824] BETTER | 0.475 [0.473, 0.485] BETTER | 0.453 [0.450, 0.457] BETTER | 0.099 [0.096, 0.102] BETTER | 0.117 [0.115, 0.125] BETTER | 4.500 [4.382, 4.549] WORSE | 0.985 [0.984, 0.986] EQUIVALENT | 0.851 [0.847, 0.857] BETTER | 0.475 [0.472, 0.480] BETTER |
| parquet-delta-bss-lz4 | 0.726 [0.726, 0.726] BETTER | 0.977 [0.971, 0.981] EQUIVALENT | 0.949 [0.945, 0.972] BETTER | 0.986 [0.980, 0.989] EQUIVALENT | 0.828 [0.811, 0.845] BETTER | 0.820 [0.802, 0.842] BETTER | 1.004 [1.003, 1.006] EQUIVALENT | 0.982 [0.981, 0.982] EQUIVALENT | 0.983 [0.979, 0.990] EQUIVALENT | 0.949 [0.943, 0.965] BETTER |
| parquet-dictionary-lz4 | 0.225 [0.225, 0.225] BETTER | 0.442 [0.439, 0.443] BETTER | 0.625 [0.622, 0.638] BETTER | 0.628 [0.619, 0.632] BETTER | 0.583 [0.567, 0.588] BETTER | 0.576 [0.551, 0.606] BETTER | 0.958 [0.957, 0.958] EQUIVALENT | 0.985 [0.985, 0.986] EQUIVALENT | 0.450 [0.448, 0.453] BETTER | 0.624 [0.619, 0.632] BETTER |

## S2/v1

Valid rounds 15 of 17 attempted; flagged rounds: 2.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 83.815 MB | 1,298.7 ms (MAD 13.6 ms) | 766.1 ms (MAD 6.6 ms) | 781.4 ms (MAD 3.5 ms) | 2.7 ms (MAD 2.7 ms) | 8.1 ms (MAD 3.5 ms) | 636.6 MiB (MAD 3.7 MiB) | 76.2 MiB (MAD 0.1 MiB) | 1,179.1 ms (MAD 11.2 ms) | 792.9 ms (MAD 10.9 ms) |
| clickhouse-tuned | yes | 34.483 MB | 1,453.3 ms (MAD 8.4 ms) | 895.1 ms (MAD 12.8 ms) | 905.5 ms (MAD 5.2 ms) | 5.7 ms (MAD 2.7 ms) | 8.3 ms (MAD 3.1 ms) | 554.2 MiB (MAD 5.2 MiB) | 78.4 MiB (MAD 0.3 MiB) | 1,372.5 ms (MAD 6.4 ms) | 926.9 ms (MAD 7.6 ms) |
| duckdb-native | yes | 31.470 MB | 1,704.8 ms (MAD 14.4 ms) | 307.7 ms (MAD 3.0 ms) | 342.8 ms (MAD 3.0 ms) | 19.6 ms (MAD 0.6 ms) | 22.9 ms (MAD 0.5 ms) | 111.1 MiB (MAD 0.4 MiB) | 253.5 MiB (MAD 0.3 MiB) | 1,660.5 ms (MAD 9.0 ms) | 307.6 ms (MAD 2.3 ms) |
| duckdb-parquet-default | yes | 75.027 MB | 991.7 ms (MAD 6.4 ms) | 436.3 ms (MAD 4.1 ms) | 488.4 ms (MAD 4.5 ms) | 31.1 ms (MAD 0.7 ms) | 34.2 ms (MAD 0.7 ms) | 47.3 MiB (MAD 1.0 MiB) | 231.8 MiB (MAD 0.5 MiB) | 925.2 ms (MAD 5.6 ms) | 435.2 ms (MAD 6.4 ms) |
| gqodb | yes | 22.169 MB | 702.9 ms (MAD 4.8 ms) | 216.9 ms (MAD 2.4 ms) | 230.2 ms (MAD 1.2 ms) | 3.3 ms (MAD 0.1 ms) | 4.4 ms (MAD 0.1 ms) | 234.1 MiB (MAD 0.1 MiB) | 228.9 MiB (MAD 0.1 MiB) | 679.7 ms (MAD 1.8 ms) | 215.9 ms (MAD 2.5 ms) |
| parquet-delta-bss-lz4 | yes | 52.118 MB | 803.7 ms (MAD 5.2 ms) | 260.6 ms (MAD 1.6 ms) | 286.0 ms (MAD 3.4 ms) | 4.4 ms (MAD 0.1 ms) | 5.7 ms (MAD 0.2 ms) | 232.8 MiB (MAD 0.1 MiB) | 229.2 MiB (MAD 0.0 MiB) | 752.0 ms (MAD 1.6 ms) | 259.4 ms (MAD 1.7 ms) |
| parquet-dictionary-lz4 | yes | 100.826 MB | 1,700.0 ms (MAD 24.0 ms) | 337.6 ms (MAD 2.3 ms) | 354.7 ms (MAD 2.8 ms) | 5.2 ms (MAD 0.1 ms) | 6.5 ms (MAD 0.1 ms) | 241.5 MiB (MAD 0.1 MiB) | 232.8 MiB (MAD 0.1 MiB) | 1,615.4 ms (MAD 22.3 ms) | 335.4 ms (MAD 2.1 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.264 [0.264, 0.264] BETTER | 0.541 [0.535, 0.546] BETTER | 0.283 [0.281, 0.286] BETTER | 0.295 [0.293, 0.297] BETTER | 1.242 [0.437, 4.342] INCONCLUSIVE | 0.535 [0.370, 0.781] BETTER | 0.368 [0.366, 0.371] BETTER | 3.002 [2.994, 3.004] WORSE | 0.576 [0.573, 0.582] BETTER | 0.272 [0.270, 0.275] BETTER |
| clickhouse-tuned | 0.643 [0.643, 0.643] BETTER | 0.484 [0.481, 0.489] BETTER | 0.242 [0.239, 0.246] BETTER | 0.254 [0.252, 0.255] BETTER | 0.585 [0.435, 1.427] INCONCLUSIVE | 0.527 [0.503, 1.784] INCONCLUSIVE | 0.422 [0.405, 0.424] BETTER | 2.920 [2.910, 2.932] WORSE | 0.495 [0.493, 0.499] BETTER | 0.233 [0.230, 0.236] BETTER |
| duckdb-native | 0.704 [0.704, 0.704] BETTER | 0.412 [0.411, 0.415] BETTER | 0.705 [0.701, 0.714] BETTER | 0.671 [0.667, 0.678] BETTER | 0.169 [0.164, 0.179] BETTER | 0.190 [0.185, 0.195] BETTER | 2.107 [2.100, 2.113] WORSE | 0.903 [0.903, 0.903] EQUIVALENT | 0.409 [0.405, 0.411] BETTER | 0.702 [0.693, 0.710] BETTER |
| duckdb-parquet-default | 0.295 [0.295, 0.295] BETTER | 0.709 [0.703, 0.713] BETTER | 0.497 [0.494, 0.508] BETTER | 0.471 [0.468, 0.476] BETTER | 0.107 [0.103, 0.110] BETTER | 0.127 [0.123, 0.129] BETTER | 4.944 [4.844, 4.972] WORSE | 0.987 [0.986, 0.994] EQUIVALENT | 0.735 [0.730, 0.738] BETTER | 0.496 [0.492, 0.503] BETTER |
| parquet-delta-bss-lz4 | 0.425 [0.425, 0.425] BETTER | 0.875 [0.867, 0.876] BETTER | 0.832 [0.828, 0.844] BETTER | 0.805 [0.800, 0.813] BETTER | 0.753 [0.726, 0.780] BETTER | 0.765 [0.686, 0.784] BETTER | 1.005 [1.005, 1.006] EQUIVALENT | 0.998 [0.998, 0.999] EQUIVALENT | 0.904 [0.892, 0.905] BETTER | 0.832 [0.825, 0.844] BETTER |
| parquet-dictionary-lz4 | 0.220 [0.220, 0.220] BETTER | 0.413 [0.402, 0.420] BETTER | 0.642 [0.637, 0.650] BETTER | 0.649 [0.638, 0.654] BETTER | 0.639 [0.614, 0.655] BETTER | 0.665 [0.636, 0.673] BETTER | 0.969 [0.969, 0.979] EQUIVALENT | 0.983 [0.982, 0.983] EQUIVALENT | 0.421 [0.411, 0.425] BETTER | 0.644 [0.632, 0.650] BETTER |

## S2/v2

Valid rounds 15 of 17 attempted; flagged rounds: 2.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 82.760 MB | 1,321.7 ms (MAD 5.9 ms) | 789.6 ms (MAD 8.3 ms) | 800.2 ms (MAD 4.9 ms) | 5.9 ms (MAD 2.9 ms) | 9.8 ms (MAD 3.9 ms) | 638.9 MiB (MAD 2.6 MiB) | 76.2 MiB (MAD 0.0 MiB) | 1,201.8 ms (MAD 4.6 ms) | 812.4 ms (MAD 5.7 ms) |
| clickhouse-tuned | yes | 26.428 MB | 1,470.9 ms (MAD 8.9 ms) | 927.9 ms (MAD 4.3 ms) | 933.3 ms (MAD 4.9 ms) | 7.1 ms (MAD 4.0 ms) | 5.7 ms (MAD 3.2 ms) | 578.4 MiB (MAD 1.6 MiB) | 78.3 MiB (MAD 0.2 MiB) | 1,392.6 ms (MAD 4.1 ms) | 956.2 ms (MAD 5.1 ms) |
| duckdb-native | yes | 46.936 MB | 1,615.5 ms (MAD 7.3 ms) | 329.1 ms (MAD 1.6 ms) | 378.1 ms (MAD 2.7 ms) | 20.8 ms (MAD 0.7 ms) | 23.6 ms (MAD 0.3 ms) | 123.7 MiB (MAD 0.5 MiB) | 268.6 MiB (MAD 0.2 MiB) | 1,564.9 ms (MAD 11.1 ms) | 327.2 ms (MAD 1.4 ms) |
| duckdb-parquet-default | yes | 75.872 MB | 982.0 ms (MAD 1.6 ms) | 462.6 ms (MAD 2.2 ms) | 504.8 ms (MAD 2.5 ms) | 31.9 ms (MAD 0.9 ms) | 35.0 ms (MAD 0.9 ms) | 46.8 MiB (MAD 0.9 MiB) | 232.8 MiB (MAD 0.4 MiB) | 915.3 ms (MAD 6.7 ms) | 460.5 ms (MAD 4.0 ms) |
| gqodb | yes | 22.168 MB | 791.8 ms (MAD 2.3 ms) | 217.5 ms (MAD 1.3 ms) | 229.4 ms (MAD 1.3 ms) | 3.0 ms (MAD 0.0 ms) | 4.0 ms (MAD 0.0 ms) | 234.0 MiB (MAD 0.1 MiB) | 230.4 MiB (MAD 0.0 MiB) | 769.0 ms (MAD 1.7 ms) | 216.7 ms (MAD 0.6 ms) |
| parquet-delta-bss-lz4 | yes | 32.915 MB | 797.9 ms (MAD 2.5 ms) | 229.1 ms (MAD 1.1 ms) | 234.6 ms (MAD 1.1 ms) | 3.9 ms (MAD 0.0 ms) | 4.9 ms (MAD 0.1 ms) | 232.7 MiB (MAD 0.0 MiB) | 232.4 MiB (MAD 0.1 MiB) | 766.2 ms (MAD 2.1 ms) | 227.4 ms (MAD 1.0 ms) |
| parquet-dictionary-lz4 | yes | 102.275 MB | 1,773.0 ms (MAD 5.6 ms) | 354.8 ms (MAD 1.2 ms) | 369.4 ms (MAD 1.6 ms) | 5.4 ms (MAD 0.0 ms) | 6.6 ms (MAD 0.1 ms) | 241.0 MiB (MAD 0.5 MiB) | 232.6 MiB (MAD 0.3 MiB) | 1,684.6 ms (MAD 6.8 ms) | 353.5 ms (MAD 1.5 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.268 [0.268, 0.268] BETTER | 0.599 [0.597, 0.603] BETTER | 0.275 [0.273, 0.279] BETTER | 0.287 [0.285, 0.289] BETTER | 0.512 [0.345, 1.219] INCONCLUSIVE | 0.406 [0.301, 0.503] BETTER | 0.366 [0.366, 0.368] BETTER | 3.024 [3.022, 3.024] WORSE | 0.640 [0.637, 0.641] BETTER | 0.267 [0.266, 0.269] BETTER |
| clickhouse-tuned | 0.839 [0.839, 0.839] BETTER | 0.538 [0.535, 0.542] BETTER | 0.234 [0.234, 0.237] BETTER | 0.246 [0.245, 0.248] BETTER | 0.425 [0.341, 0.958] BETTER | 0.698 [0.267, 0.984] BETTER | 0.405 [0.404, 0.405] BETTER | 2.944 [2.937, 2.953] WORSE | 0.552 [0.546, 0.554] BETTER | 0.227 [0.225, 0.228] BETTER |
| duckdb-native | 0.472 [0.472, 0.472] BETTER | 0.490 [0.488, 0.492] BETTER | 0.661 [0.657, 0.666] BETTER | 0.607 [0.603, 0.614] BETTER | 0.145 [0.142, 0.153] BETTER | 0.168 [0.166, 0.170] BETTER | 1.892 [1.885, 1.904] WORSE | 0.858 [0.857, 0.858] BETTER | 0.491 [0.487, 0.494] BETTER | 0.662 [0.658, 0.663] BETTER |
| duckdb-parquet-default | 0.292 [0.292, 0.292] BETTER | 0.806 [0.805, 0.807] BETTER | 0.470 [0.469, 0.476] BETTER | 0.454 [0.452, 0.460] BETTER | 0.094 [0.093, 0.099] BETTER | 0.114 [0.110, 0.115] BETTER | 5.000 [4.956, 5.083] WORSE | 0.990 [0.988, 0.990] EQUIVALENT | 0.840 [0.836, 0.846] BETTER | 0.470 [0.467, 0.474] BETTER |
| parquet-delta-bss-lz4 | 0.674 [0.674, 0.674] BETTER | 0.992 [0.990, 0.995] EQUIVALENT | 0.949 [0.945, 0.958] BETTER | 0.978 [0.971, 0.987] EQUIVALENT | 0.770 [0.764, 0.778] BETTER | 0.805 [0.794, 0.812] BETTER | 1.006 [1.005, 1.006] EQUIVALENT | 0.991 [0.991, 0.992] EQUIVALENT | 1.004 [1.001, 1.008] EQUIVALENT | 0.953 [0.944, 0.957] INCONCLUSIVE |
| parquet-dictionary-lz4 | 0.217 [0.217, 0.217] BETTER | 0.447 [0.445, 0.448] BETTER | 0.613 [0.610, 0.619] BETTER | 0.621 [0.615, 0.626] BETTER | 0.562 [0.555, 0.571] BETTER | 0.607 [0.605, 0.608] BETTER | 0.971 [0.970, 0.974] EQUIVALENT | 0.991 [0.990, 0.994] EQUIVALENT | 0.456 [0.455, 0.460] BETTER | 0.613 [0.609, 0.616] BETTER |

## Headline claims (protocol section 10)

| baseline | bytes | write_op | read_full_warm_op | read_range_warm_op | rss_write | rss_full |
| --- | --- | --- | --- | --- | --- | --- |
| parquet-delta-bss-lz4 | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | EQUIVALENT / EQUIVALENT |
| parquet-dictionary-lz4 | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | EQUIVALENT / EQUIVALENT |
| duckdb-native | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | BETTER / BETTER **win** |
| duckdb-parquet-default | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | WORSE / WORSE | BETTER / BETTER **win** |
| clickhouse-default | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | INCONCLUSIVE / BETTER | BETTER / BETTER **win** | BETTER / BETTER **win** |
| clickhouse-tuned | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** |

GQODB wins every headline axis against every headline candidate: **False**.

## What this does not show

Aggregate trades only, one instrument, two months, one host class, one thread, no concurrency, no network; a young prototype against mature products; defaults plus one pre-registered alternative per tool; embedded libraries against a process per statement for ClickHouse. Full list: protocol section 11.

## Interpretation

**Result in one sentence.** On this input and these settings GQODB is the smallest layout and, with the
exceptions below, faster than every other file layout on write, full read and the 60-second range read; it does
not win every headline axis, and its memory use is not better than the alternatives. The claim "wins every
axis" is **false** (headline verdict above).

**This is the second run of the same matrix.** Run 1 (`results/b17-public-trades-20261017`) was measured on a
commit before the decoded-string-size limit was added to the reader; this run, the main result, was measured on
the commit that is released (`c07fbde`). After that fix `real::decode` makes one extra pass over the id vector of
each string column and `real::encode` adds one saturating addition and a comparison per string row. The
GQODB medians of this run are 0.4% to 7% lower than those of run 1 on every time axis and the baselines moved
the same way (for example `parquet-delta-bss-lz4` 0.94 to 1.00, `duckdb-native` 0.92 to 0.99), stored bytes are
identical to the byte, so the extra pass is not visible against the drift between the two runs (a few percent;
the machine was not in the same state). The comparison cannot rule out an effect of one or two percent.
A cell-by-cell comparison of the two runs is in the repository README: 26 of 864 comparisons change verdict;
15 are range reads against ClickHouse (3 to 7 ms, noisy), two of them on V1 headline cells (D1 warm range read
against `clickhouse-default`, BETTER to INCONCLUSIVE; D2 against `clickhouse-tuned`, INCONCLUSIVE to BETTER),
and one is a V2 write cell (D1 write including normalisation against `parquet-delta-bss-snappy`, INCONCLUSIVE to
WORSE); no stored-bytes verdict changes.

**Exactness.** All 14,139 adapter runs finished (0 process failures) and every read returned the reference
table bit for bit (0 digest mismatches, 0 EXACTNESS FAIL cells), for GQODB and all ten baselines, both
variants, both slices and both full months. No cell is NOT COMPARABLE: the files were evicted with
`posix_fadvise` and `mincore` showed 0% residency before every cold read. "Cold" here means evicted from
the operating-system page cache of a fast local disk; the cold and warm figures are close for these
file sizes and this is not a cold-device measurement.

**Two cells are UNDERPOWERED.** D2/v1 (11 valid rounds) and D2/v2 (12 valid rounds) did not reach 15 unflagged
rounds within the repeat budget of addendum A-004 (3 warm-up + 15 measured + 25 extra rounds): unrelated
processes on the shared host kept landing on the hyperthread sibling of the benchmark core for long stretches
(29 of 40 and 28 of 40 measured rounds were flagged). D2/v1 is one of the two slices behind the headline table, so
the D2 rows of the headline table rest on 11 rounds, not 15, and their intervals are correspondingly wider.
The cells were not repeated (the protocol allows no more attempts); the sensitivity analysis over all
measured rounds is in `summary.json`. Run 1 had 15 valid rounds in D2/v1 and D2/v2 and the D2 ratios of the two
runs agree to within 0.01 to 0.02 on every headline axis against the strongest baseline; only the range reads against
ClickHouse move.

**Headline cells that are not BETTER (V1, D1 and D2, GQODB / baseline).**

* Peak memory of the write, against the three DuckDB Parquet writers: WORSE on both slices
  (about 72.5 MiB for GQODB against about 17 to 33 MiB; ratios 2.2 against default, 4.2 against 32,768-row
  LZ4, 3.8 to 3.9 against zstd). The cause lies in the measured adapter path and not in a step that is specific
  to the GQODB writer: the Rust adapter (`trades_candidate`, phase `write`) reads the whole source table
  into memory before it writes, for every Rust candidate including the Rust Parquet writers, which is
  why those sit at the same level; DuckDB streams its input. Against `duckdb-native` and the Rust Parquet
  writers the write memory is EQUIVALENT (ratio 0.91 to 1.01; BETTER against the brotli writer).
* Peak memory of the full read, against the Rust Parquet configurations: EQUIVALENT (0.93 to 0.99).
* Warm range read against `clickhouse-default`: INCONCLUSIVE on D1 (0.509) and BETTER on D2; against
  `clickhouse-tuned` BETTER on both slices in this run (D1 0.587 [0.285, 0.715], D2 0.328 [0.277, 0.711]; in run 1 the D2 cell
  was INCONCLUSIVE). The range reads take 3 to 7 ms and these ClickHouse verdicts move between runs.

**Other losses and non-wins** (all cells are in the tables; none was removed).

* Full months (S1, S2, V1): write memory WORSE against `duckdb-native` (1.8 and 2.1) and
  `duckdb-parquet-default` (4.6 and 4.9); full-read memory WORSE against both ClickHouse configurations
  (2.5 to 3.0), where the ClickHouse figure is the engine process only and excludes the buffer in which the
  Python client receives the result (see "What this does not show": the two sides do not measure the
  same thing). Warm range read against ClickHouse on the full months is INCONCLUSIVE in S1 (default, 0.49) and S2
  (default 1.24, tuned 0.59), none WORSE.
* V2: write time against `parquet-delta-bss-snappy` on D1 is WORSE (1.06, interval above 1) and
  against `parquet-delta-bss-lz4` it is EQUIVALENT on D1 and D2; the V2 table has no
  floats, which presumably removes the part of the advantage that comes from GQODB's exact decimal scaling of
  float columns (not isolated by an experiment). The secondary CPU-time metric of the write (`cpu_write`)
  against `parquet-delta-bss-snappy` is WORSE on D1 in V2 (1.067).
* End-to-end wall time (`*_e2e`, secondary) is not comparable for reads: it contains the time spent
  computing the exactness digest after the clock stopped, and that code differs per adapter (Rust for
  GQODB and Parquet, pyarrow for DuckDB and ClickHouse). On the full months the e2e warm full read is
  WORSE than DuckDB (1.1 to 1.3) while the operation time is BETTER (0.50 to 0.71); the e2e columns must not be used to
  rank engines.
* Time advantage is moderate against the strongest Parquet configuration (`parquet-delta-bss-lz4`,
  LZ4_RAW with delta and byte-stream-split): write 0.83 to 0.91 (against the Snappy writer on D1 and D2, the LZ4 writer on the full months), full read
  0.81 to 0.86, range read about 0.75 to 0.79 (D1, D2, S1). The large ratios against other baselines come mostly from
  the baselines' defaults (dictionary encoding of floats, uncompressed or general-purpose codecs,
  process start of ClickHouse subtracted but its engine work included) and, for the ClickHouse full read
  (0.23 to 0.29), from its process model: one process per statement that hands the result to the Python client as
  Arrow IPC through a pipe.
* Size: GQODB is the smallest layout in every slice and month (6.51 MB on D1 against 6.70 MB for DuckDB
  Parquet with zstd, 0.972, BETTER only by a small margin; 6.21 against 7.63 MB on D2; 19.5 against 27.0 MB
  for the best baseline, `duckdb-native`, on S1). The ClickHouse default layout is 3.6 to 3.9 times larger than GQODB
  and the tuned chain 1.1 to 1.7 times larger. The tuned ClickHouse chain and the DuckDB settings are
  the single pre-registered alternatives, not the result of a search. Sizes were identical in every round.
* The 60-second window holds 45 rows (D1) and 468 rows (D2): the "range read" is a very small query.
* Range reads against ClickHouse are noisy. Its operation time is the wall time of the statement process
  minus the wall time of a preceding `SELECT 1` process, floored at zero; 141 of the 932 ClickHouse
  range-read samples (warm and cold, all rounds) record an operation time of 0 ns. The warm range-read
  verdicts against ClickHouse therefore carry little information in either direction, including the BETTER ones.

**Flagged rounds.** Of 209 measured rounds attempted across the 8 cells (plus 3 warm-up rounds each), 96 were flagged and
repeated: 8 (D1/v1), 17 (D1/v2), 29 (D2/v1), 28 (D2/v2), 0 (S1/v1), 10 (S1/v2), 2 (S2/v1), 2 (S2/v2).
1,115 samples carried the `sibling_busy` flag (unrelated processes on the hyperthread sibling of the benchmark core, in
bursts) and 8 the `low_cpu_ratio` flag. Six cells reached the 15 unflagged measured rounds; two did not (see above).
The sensitivity analysis over all measured rounds (`sensitivity_all_rounds` in `summary.json`) changes 22
verdicts out of 864 comparisons; one of them is on a headline axis (warm range read against `clickhouse-default`
on D2, BETTER to INCONCLUSIVE); the others are V2 write and CPU cells and full-read cells against
`parquet-delta-bss-lz4` and ClickHouse range reads.

**Conditions.** One physical core pair (benchmark on one core, its sibling idle apart from the flagged
bursts), single thread, a host shared with unrelated jobs that were not controlled (the last-level cache and
memory bandwidth were shared). The busy time outside the benchmark cores, measured before every round, is stored per
round (`box_busy`, `foreign_busy`). Results are for this host class only.

## Deviations from the protocol

All deviations are recorded with their timing and what had been seen in `docs/B17_PUBLIC_TRADES_ADDENDA.md`
(they apply to this run exactly as to run 1; this run changed no setting):

* A-001 (pre-measurement): implementation clarifications of the frozen text (rule tie order, one extra ClickHouse
  ingest setting, startup subtraction details, noop per candidate, and others).
* A-002 (pre-measurement): the tooling was run once on the real input (data preparation only).
* A-003 (pre-measurement): owner decisions. An optional baseline that had been planned was dropped and the
  protocol file was edited once, only to delete that row and phrase, with a new pinned hash; Binance Vision data are
  used at the owner's own risk with attribution and aggregates only; the run was made resumable. For this run the
  declared foreign cores are CPUs 6 and 20 only, as in the protocol.
* A-004 (an amendment made after a first start of run 1 was aborted): the sibling-busy threshold is max(5 jiffies, 10%)
  instead of max(2 jiffies, 2%), the CPU-ratio flag is only evaluated for samples of at least 100 ms, and the number of
  extra rounds allowed per cell is 25 instead of 5. Applied to the stored samples of this run, the registered thresholds would have left every cell below 15 valid
  rounds (1 to 12 over all attempted rounds); with the amended thresholds two cells (D2/v1, D2/v2) still did not
  reach 15.
* A-005: this run was measured on the released commit `c07fbde` with a clean tree (run 1 on an earlier commit).
