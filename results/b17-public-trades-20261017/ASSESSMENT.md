# B17: public-data file-level benchmark (published run)

Generated from `summary.json`; protocol `docs/B17_PUBLIC_TRADES_PROTOCOL.md`. Every cell below is reported, wins and losses alike. Verdict codes: BETTER, WORSE, EQUIVALENT, INCONCLUSIVE (ratio gqodb / baseline, 95% moving-block bootstrap interval), plus EXACTNESS FAIL, NOT COMPARABLE, UNDERPOWERED.

Receipt: `BUILD_RECEIPTS.json`.

**Input data: Binance Vision (https://data.binance.vision), PAXGUSDT spot aggregate trades.** Provided by Binance under CC BY-NC-SA 4.0 and the Binance Vision Dataset Terms (version 1.0, last updated 26 August 2026). The data are not redistributed here; fetch them yourself and comply with those terms. This assessment contains aggregates only. Binance does not sponsor, endorse or partner with this project or these results. See `docs/BASELINE_LICENCES.md`.

## D1/v1

Valid rounds 15 of 17 attempted; flagged rounds: 2.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.338 MB | 501.4 ms (MAD 8.3 ms) | 237.5 ms (MAD 2.9 ms) | 245.8 ms (MAD 4.2 ms) | 6.3 ms (MAD 2.0 ms) | 6.3 ms (MAD 2.7 ms) | 245.7 MiB (MAD 4.6 MiB) | 76.0 MiB (MAD 0.0 MiB) | 428.5 ms (MAD 5.1 ms) | 307.9 ms (MAD 4.3 ms) |
| clickhouse-tuned | yes | 9.237 MB | 536.1 ms (MAD 8.8 ms) | 280.4 ms (MAD 3.6 ms) | 284.4 ms (MAD 4.4 ms) | 7.2 ms (MAD 2.1 ms) | 13.0 ms (MAD 4.7 ms) | 224.4 MiB (MAD 2.7 MiB) | 78.0 MiB (MAD 0.0 MiB) | 483.3 ms (MAD 7.4 ms) | 351.2 ms (MAD 2.7 ms) |
| duckdb-native | yes | 9.449 MB | 573.4 ms (MAD 5.5 ms) | 107.6 ms (MAD 0.6 ms) | 119.0 ms (MAD 0.9 ms) | 21.5 ms (MAD 1.0 ms) | 25.2 ms (MAD 0.5 ms) | 79.1 MiB (MAD 0.5 MiB) | 79.2 MiB (MAD 0.3 MiB) | 551.8 ms (MAD 5.1 ms) | 107.7 ms (MAD 1.8 ms) |
| duckdb-parquet-default | yes | 21.113 MB | 326.4 ms (MAD 3.9 ms) | 151.2 ms (MAD 1.4 ms) | 166.2 ms (MAD 1.7 ms) | 29.3 ms (MAD 0.5 ms) | 32.3 ms (MAD 1.0 ms) | 33.6 MiB (MAD 0.5 MiB) | 76.0 MiB (MAD 0.2 MiB) | 304.3 ms (MAD 3.4 ms) | 149.4 ms (MAD 1.3 ms) |
| duckdb-parquet-matched | yes | 8.053 MB | 288.0 ms (MAD 1.8 ms) | 124.3 ms (MAD 0.7 ms) | 128.4 ms (MAD 1.0 ms) | 24.6 ms (MAD 0.3 ms) | 26.5 ms (MAD 0.7 ms) | 17.0 MiB (MAD 0.2 MiB) | 72.5 MiB (MAD 0.3 MiB) | 276.6 ms (MAD 1.3 ms) | 124.2 ms (MAD 2.4 ms) |
| duckdb-parquet-zstd | yes | 6.698 MB | 358.7 ms (MAD 4.3 ms) | 136.3 ms (MAD 0.8 ms) | 139.9 ms (MAD 1.5 ms) | 24.8 ms (MAD 0.6 ms) | 26.4 ms (MAD 0.8 ms) | 18.5 MiB (MAD 0.2 MiB) | 73.0 MiB (MAD 0.1 MiB) | 349.7 ms (MAD 3.9 ms) | 134.9 ms (MAD 1.2 ms) |
| gqodb | yes | 6.512 MB | 232.7 ms (MAD 1.3 ms) | 67.8 ms (MAD 0.1 ms) | 72.1 ms (MAD 0.4 ms) | 3.3 ms (MAD 0.0 ms) | 4.2 ms (MAD 0.1 ms) | 72.6 MiB (MAD 0.0 MiB) | 68.2 MiB (MAD 0.0 MiB) | 223.5 ms (MAD 1.0 ms) | 67.1 ms (MAD 0.4 ms) |
| parquet-delta-bss-brotli5 | yes | 10.719 MB | 1,396.1 ms (MAD 4.8 ms) | 313.9 ms (MAD 1.1 ms) | 317.5 ms (MAD 1.0 ms) | 12.3 ms (MAD 0.1 ms) | 13.4 ms (MAD 0.1 ms) | 88.0 MiB (MAD 0.1 MiB) | 73.1 MiB (MAD 0.1 MiB) | 1,381.8 ms (MAD 3.9 ms) | 312.6 ms (MAD 1.3 ms) |
| parquet-delta-bss-lz4 | yes | 14.190 MB | 262.0 ms (MAD 1.0 ms) | 82.6 ms (MAD 0.5 ms) | 89.3 ms (MAD 0.2 ms) | 4.2 ms (MAD 0.0 ms) | 5.4 ms (MAD 0.1 ms) | 72.0 MiB (MAD 0.1 MiB) | 69.1 MiB (MAD 0.1 MiB) | 247.3 ms (MAD 0.6 ms) | 81.6 ms (MAD 0.7 ms) |
| parquet-delta-bss-snappy | yes | 14.014 MB | 259.2 ms (MAD 1.1 ms) | 86.6 ms (MAD 0.2 ms) | 92.7 ms (MAD 0.5 ms) | 4.4 ms (MAD 0.0 ms) | 5.5 ms (MAD 0.0 ms) | 72.1 MiB (MAD 0.1 MiB) | 68.9 MiB (MAD 0.0 MiB) | 244.0 ms (MAD 0.7 ms) | 85.2 ms (MAD 0.4 ms) |
| parquet-dictionary-lz4 | yes | 28.546 MB | 544.7 ms (MAD 3.6 ms) | 104.4 ms (MAD 0.3 ms) | 110.2 ms (MAD 0.6 ms) | 5.0 ms (MAD 0.0 ms) | 6.2 ms (MAD 0.0 ms) | 75.9 MiB (MAD 0.1 MiB) | 70.6 MiB (MAD 0.1 MiB) | 518.8 ms (MAD 2.8 ms) | 103.5 ms (MAD 0.6 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.279 [0.279, 0.279] BETTER | 0.464 [0.460, 0.476] BETTER | 0.286 [0.285, 0.289] BETTER | 0.293 [0.285, 0.296] BETTER | 0.529 [0.394, 0.690] BETTER | 0.667 [0.479, 1.018] INCONCLUSIVE | 0.296 [0.293, 0.300] BETTER | 0.897 [0.895, 0.900] BETTER | 0.522 [0.514, 0.526] BETTER | 0.218 [0.216, 0.222] BETTER |
| clickhouse-tuned | 0.705 [0.705, 0.705] BETTER | 0.434 [0.425, 0.436] BETTER | 0.242 [0.240, 0.244] BETTER | 0.253 [0.251, 0.257] BETTER | 0.463 [0.285, 0.641] BETTER | 0.323 [0.277, 0.846] BETTER | 0.324 [0.321, 0.327] BETTER | 0.874 [0.872, 0.874] BETTER | 0.463 [0.455, 0.468] BETTER | 0.191 [0.190, 0.196] BETTER |
| duckdb-native | 0.689 [0.689, 0.689] BETTER | 0.406 [0.403, 0.409] BETTER | 0.630 [0.627, 0.632] BETTER | 0.606 [0.601, 0.610] BETTER | 0.154 [0.152, 0.156] BETTER | 0.167 [0.163, 0.172] BETTER | 0.918 [0.913, 0.922] EQUIVALENT | 0.860 [0.857, 0.863] BETTER | 0.405 [0.401, 0.408] BETTER | 0.623 [0.618, 0.641] BETTER |
| duckdb-parquet-default | 0.308 [0.308, 0.308] BETTER | 0.713 [0.702, 0.718] BETTER | 0.449 [0.446, 0.453] BETTER | 0.434 [0.430, 0.438] BETTER | 0.113 [0.111, 0.119] BETTER | 0.130 [0.126, 0.133] BETTER | 2.163 [2.152, 2.194] WORSE | 0.897 [0.895, 0.898] BETTER | 0.735 [0.725, 0.742] BETTER | 0.449 [0.445, 0.451] BETTER |
| duckdb-parquet-matched | 0.809 [0.809, 0.809] BETTER | 0.808 [0.801, 0.811] BETTER | 0.546 [0.533, 0.548] BETTER | 0.561 [0.555, 0.568] BETTER | 0.135 [0.134, 0.136] BETTER | 0.159 [0.156, 0.163] BETTER | 4.261 [4.209, 4.311] WORSE | 0.940 [0.934, 0.943] EQUIVALENT | 0.808 [0.801, 0.811] BETTER | 0.541 [0.527, 0.548] BETTER |
| duckdb-parquet-zstd | 0.972 [0.972, 0.972] BETTER | 0.649 [0.639, 0.653] BETTER | 0.498 [0.496, 0.501] BETTER | 0.515 [0.513, 0.522] BETTER | 0.134 [0.132, 0.139] BETTER | 0.159 [0.151, 0.162] BETTER | 3.931 [3.925, 3.943] WORSE | 0.933 [0.932, 0.934] EQUIVALENT | 0.639 [0.635, 0.645] BETTER | 0.498 [0.495, 0.504] BETTER |
| parquet-delta-bss-brotli5 | 0.608 [0.608, 0.608] BETTER | 0.167 [0.165, 0.167] BETTER | 0.216 [0.216, 0.217] BETTER | 0.227 [0.226, 0.229] BETTER | 0.269 [0.267, 0.271] BETTER | 0.314 [0.311, 0.319] BETTER | 0.825 [0.824, 0.826] BETTER | 0.933 [0.931, 0.935] EQUIVALENT | 0.162 [0.161, 0.162] BETTER | 0.215 [0.213, 0.216] BETTER |
| parquet-delta-bss-lz4 | 0.459 [0.459, 0.459] BETTER | 0.888 [0.881, 0.891] BETTER | 0.821 [0.815, 0.823] BETTER | 0.807 [0.803, 0.812] BETTER | 0.791 [0.783, 0.796] BETTER | 0.787 [0.779, 0.797] BETTER | 1.009 [1.006, 1.010] EQUIVALENT | 0.987 [0.985, 0.988] EQUIVALENT | 0.904 [0.898, 0.908] BETTER | 0.822 [0.818, 0.831] BETTER |
| parquet-delta-bss-snappy | 0.465 [0.465, 0.465] BETTER | 0.898 [0.891, 0.900] BETTER | 0.784 [0.782, 0.785] BETTER | 0.777 [0.772, 0.782] BETTER | 0.752 [0.747, 0.758] BETTER | 0.768 [0.760, 0.781] BETTER | 1.007 [1.006, 1.008] EQUIVALENT | 0.990 [0.987, 0.991] EQUIVALENT | 0.916 [0.909, 0.920] BETTER | 0.788 [0.782, 0.794] BETTER |
| parquet-dictionary-lz4 | 0.228 [0.228, 0.228] BETTER | 0.427 [0.424, 0.428] BETTER | 0.650 [0.648, 0.651] BETTER | 0.654 [0.650, 0.659] BETTER | 0.657 [0.654, 0.661] BETTER | 0.684 [0.676, 0.692] BETTER | 0.957 [0.956, 0.959] EQUIVALENT | 0.966 [0.964, 0.967] EQUIVALENT | 0.431 [0.430, 0.434] BETTER | 0.649 [0.646, 0.655] BETTER |

## D1/v2

Valid rounds 15 of 22 attempted; flagged rounds: 7.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.155 MB | 491.5 ms (MAD 6.0 ms) | 235.1 ms (MAD 6.3 ms) | 241.7 ms (MAD 4.7 ms) | 3.3 ms (MAD 3.3 ms) | 7.4 ms (MAD 2.9 ms) | 243.5 MiB (MAD 3.3 MiB) | 75.6 MiB (MAD 0.1 MiB) | 420.3 ms (MAD 4.9 ms) | 305.9 ms (MAD 4.5 ms) |
| clickhouse-tuned | yes | 7.423 MB | 525.5 ms (MAD 6.8 ms) | 270.9 ms (MAD 2.1 ms) | 284.1 ms (MAD 3.5 ms) | 4.1 ms (MAD 3.5 ms) | 11.2 ms (MAD 3.8 ms) | 221.2 MiB (MAD 1.5 MiB) | 77.8 MiB (MAD 0.1 MiB) | 471.9 ms (MAD 5.2 ms) | 341.5 ms (MAD 5.1 ms) |
| duckdb-native | yes | 13.906 MB | 523.2 ms (MAD 3.3 ms) | 110.4 ms (MAD 0.4 ms) | 124.3 ms (MAD 0.7 ms) | 21.8 ms (MAD 1.2 ms) | 24.9 ms (MAD 0.1 ms) | 82.3 MiB (MAD 0.7 MiB) | 83.6 MiB (MAD 0.3 MiB) | 503.2 ms (MAD 5.5 ms) | 109.5 ms (MAD 1.7 ms) |
| duckdb-parquet-default | yes | 21.321 MB | 309.0 ms (MAD 1.3 ms) | 150.2 ms (MAD 0.9 ms) | 165.3 ms (MAD 1.0 ms) | 28.5 ms (MAD 0.5 ms) | 32.0 ms (MAD 0.8 ms) | 32.8 MiB (MAD 0.5 MiB) | 75.9 MiB (MAD 0.3 MiB) | 288.0 ms (MAD 2.2 ms) | 151.3 ms (MAD 2.7 ms) |
| duckdb-parquet-matched | yes | 8.067 MB | 266.3 ms (MAD 1.2 ms) | 125.2 ms (MAD 1.1 ms) | 128.9 ms (MAD 0.5 ms) | 23.3 ms (MAD 0.4 ms) | 26.6 ms (MAD 0.7 ms) | 17.0 MiB (MAD 0.1 MiB) | 72.9 MiB (MAD 0.2 MiB) | 255.1 ms (MAD 2.5 ms) | 125.2 ms (MAD 2.3 ms) |
| duckdb-parquet-zstd | yes | 6.673 MB | 332.0 ms (MAD 1.8 ms) | 136.7 ms (MAD 0.6 ms) | 140.5 ms (MAD 1.0 ms) | 23.9 ms (MAD 0.5 ms) | 27.0 ms (MAD 0.8 ms) | 17.9 MiB (MAD 0.3 MiB) | 72.8 MiB (MAD 0.2 MiB) | 325.6 ms (MAD 2.4 ms) | 136.9 ms (MAD 1.9 ms) |
| gqodb | yes | 6.512 MB | 246.7 ms (MAD 0.7 ms) | 64.6 ms (MAD 0.4 ms) | 68.8 ms (MAD 0.4 ms) | 3.0 ms (MAD 0.0 ms) | 3.8 ms (MAD 0.0 ms) | 72.6 MiB (MAD 0.1 MiB) | 68.0 MiB (MAD 0.1 MiB) | 237.9 ms (MAD 1.1 ms) | 63.6 ms (MAD 0.4 ms) |
| parquet-delta-bss-brotli5 | yes | 7.241 MB | 979.9 ms (MAD 6.3 ms) | 230.5 ms (MAD 1.1 ms) | 232.5 ms (MAD 1.3 ms) | 9.4 ms (MAD 0.0 ms) | 10.4 ms (MAD 0.1 ms) | 87.6 MiB (MAD 0.1 MiB) | 72.9 MiB (MAD 0.1 MiB) | 970.0 ms (MAD 5.6 ms) | 229.4 ms (MAD 0.7 ms) |
| parquet-delta-bss-lz4 | yes | 8.928 MB | 237.9 ms (MAD 0.7 ms) | 68.1 ms (MAD 0.5 ms) | 70.2 ms (MAD 0.2 ms) | 3.4 ms (MAD 0.0 ms) | 4.4 ms (MAD 0.0 ms) | 70.6 MiB (MAD 0.1 MiB) | 69.7 MiB (MAD 0.0 MiB) | 227.0 ms (MAD 0.7 ms) | 67.1 ms (MAD 0.6 ms) |
| parquet-delta-bss-snappy | yes | 8.872 MB | 234.0 ms (MAD 2.3 ms) | 70.2 ms (MAD 0.5 ms) | 72.2 ms (MAD 0.5 ms) | 3.6 ms (MAD 0.1 ms) | 4.6 ms (MAD 0.0 ms) | 70.8 MiB (MAD 0.1 MiB) | 69.9 MiB (MAD 0.1 MiB) | 222.8 ms (MAD 1.3 ms) | 69.4 ms (MAD 0.7 ms) |
| parquet-dictionary-lz4 | yes | 28.873 MB | 570.2 ms (MAD 3.2 ms) | 103.9 ms (MAD 0.6 ms) | 110.0 ms (MAD 0.6 ms) | 4.9 ms (MAD 0.0 ms) | 6.3 ms (MAD 0.0 ms) | 75.3 MiB (MAD 0.0 MiB) | 70.5 MiB (MAD 0.1 MiB) | 543.7 ms (MAD 3.8 ms) | 102.4 ms (MAD 0.6 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.281 [0.281, 0.281] BETTER | 0.502 [0.496, 0.510] BETTER | 0.275 [0.265, 0.277] BETTER | 0.285 [0.273, 0.286] BETTER | 0.884 [0.511, inf] INCONCLUSIVE | 0.512 [0.366, 0.717] BETTER | 0.298 [0.293, 0.301] BETTER | 0.899 [0.897, 0.900] BETTER | 0.566 [0.560, 0.574] BETTER | 0.208 [0.204, 0.211] BETTER |
| clickhouse-tuned | 0.877 [0.877, 0.877] BETTER | 0.469 [0.457, 0.476] BETTER | 0.238 [0.228, 0.239] BETTER | 0.242 [0.239, 0.245] BETTER | 0.715 [0.441, 1.369] INCONCLUSIVE | 0.338 [0.280, 0.551] BETTER | 0.328 [0.328, 0.340] BETTER | 0.873 [0.873, 0.875] BETTER | 0.504 [0.501, 0.514] BETTER | 0.186 [0.183, 0.187] BETTER |
| duckdb-native | 0.468 [0.468, 0.468] BETTER | 0.471 [0.468, 0.473] BETTER | 0.585 [0.579, 0.586] BETTER | 0.553 [0.550, 0.555] BETTER | 0.136 [0.130, 0.143] BETTER | 0.152 [0.150, 0.155] BETTER | 0.882 [0.873, 0.888] BETTER | 0.813 [0.810, 0.816] BETTER | 0.473 [0.471, 0.478] BETTER | 0.581 [0.574, 0.587] BETTER |
| duckdb-parquet-default | 0.305 [0.305, 0.305] BETTER | 0.798 [0.794, 0.801] BETTER | 0.430 [0.426, 0.433] BETTER | 0.416 [0.411, 0.418] BETTER | 0.104 [0.102, 0.106] BETTER | 0.118 [0.115, 0.119] BETTER | 2.217 [2.165, 2.232] WORSE | 0.896 [0.894, 0.899] BETTER | 0.826 [0.820, 0.834] BETTER | 0.420 [0.411, 0.428] BETTER |
| duckdb-parquet-matched | 0.807 [0.807, 0.807] BETTER | 0.926 [0.924, 0.932] BETTER | 0.516 [0.510, 0.523] BETTER | 0.533 [0.529, 0.536] BETTER | 0.127 [0.123, 0.129] BETTER | 0.143 [0.140, 0.148] BETTER | 4.261 [4.225, 4.286] WORSE | 0.932 [0.931, 0.935] EQUIVALENT | 0.932 [0.925, 0.941] BETTER | 0.508 [0.500, 0.524] BETTER |
| duckdb-parquet-zstd | 0.976 [0.976, 0.976] BETTER | 0.743 [0.739, 0.746] BETTER | 0.472 [0.468, 0.475] BETTER | 0.490 [0.486, 0.492] BETTER | 0.124 [0.117, 0.127] BETTER | 0.140 [0.135, 0.146] BETTER | 4.066 [3.969, 4.105] WORSE | 0.934 [0.932, 0.935] EQUIVALENT | 0.730 [0.729, 0.739] BETTER | 0.465 [0.459, 0.471] BETTER |
| parquet-delta-bss-brotli5 | 0.899 [0.899, 0.899] BETTER | 0.252 [0.250, 0.254] BETTER | 0.280 [0.277, 0.281] BETTER | 0.296 [0.294, 0.297] BETTER | 0.315 [0.312, 0.320] BETTER | 0.364 [0.361, 0.373] BETTER | 0.829 [0.828, 0.830] BETTER | 0.933 [0.932, 0.934] EQUIVALENT | 0.245 [0.244, 0.247] BETTER | 0.277 [0.275, 0.279] BETTER |
| parquet-delta-bss-lz4 | 0.729 [0.729, 0.729] BETTER | 1.037 [1.030, 1.039] EQUIVALENT | 0.947 [0.931, 0.953] BETTER | 0.979 [0.971, 0.982] EQUIVALENT | 0.863 [0.857, 0.872] BETTER | 0.859 [0.851, 0.871] BETTER | 1.028 [1.026, 1.029] EQUIVALENT | 0.975 [0.974, 0.975] EQUIVALENT | 1.048 [1.042, 1.050] EQUIVALENT | 0.947 [0.940, 0.958] BETTER |
| parquet-delta-bss-snappy | 0.734 [0.734, 0.734] BETTER | 1.054 [1.047, 1.063] WORSE | 0.920 [0.906, 0.926] BETTER | 0.952 [0.943, 0.955] INCONCLUSIVE | 0.832 [0.814, 0.846] BETTER | 0.827 [0.813, 0.843] BETTER | 1.025 [1.023, 1.027] EQUIVALENT | 0.972 [0.971, 0.973] EQUIVALENT | 1.067 [1.059, 1.072] WORSE | 0.917 [0.906, 0.926] BETTER |
| parquet-dictionary-lz4 | 0.226 [0.226, 0.226] BETTER | 0.433 [0.429, 0.434] BETTER | 0.621 [0.615, 0.625] BETTER | 0.625 [0.621, 0.628] BETTER | 0.601 [0.595, 0.608] BETTER | 0.605 [0.601, 0.614] BETTER | 0.964 [0.962, 0.965] EQUIVALENT | 0.964 [0.963, 0.965] EQUIVALENT | 0.437 [0.433, 0.439] BETTER | 0.621 [0.612, 0.623] BETTER |

## D2/v1

Valid rounds 15 of 16 attempted; flagged rounds: 1.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 24.169 MB | 489.7 ms (MAD 5.5 ms) | 245.7 ms (MAD 4.3 ms) | 250.3 ms (MAD 3.7 ms) | 7.8 ms (MAD 2.4 ms) | 10.3 ms (MAD 4.0 ms) | 241.5 MiB (MAD 4.4 MiB) | 76.0 MiB (MAD 0.0 MiB) | 420.4 ms (MAD 5.4 ms) | 312.6 ms (MAD 3.7 ms) |
| clickhouse-tuned | yes | 10.296 MB | 542.9 ms (MAD 4.0 ms) | 284.0 ms (MAD 3.1 ms) | 288.4 ms (MAD 4.8 ms) | 5.2 ms (MAD 4.6 ms) | 11.2 ms (MAD 1.7 ms) | 221.8 MiB (MAD 1.0 MiB) | 77.8 MiB (MAD 0.1 MiB) | 485.3 ms (MAD 2.2 ms) | 351.3 ms (MAD 3.9 ms) |
| duckdb-native | yes | 9.187 MB | 566.4 ms (MAD 3.7 ms) | 108.1 ms (MAD 0.9 ms) | 116.5 ms (MAD 0.3 ms) | 21.8 ms (MAD 0.8 ms) | 24.2 ms (MAD 0.1 ms) | 79.4 MiB (MAD 0.4 MiB) | 79.0 MiB (MAD 0.1 MiB) | 550.5 ms (MAD 4.2 ms) | 108.3 ms (MAD 2.5 ms) |
| duckdb-parquet-default | yes | 21.386 MB | 336.0 ms (MAD 1.7 ms) | 149.7 ms (MAD 0.9 ms) | 163.7 ms (MAD 0.9 ms) | 28.7 ms (MAD 0.5 ms) | 32.0 ms (MAD 0.5 ms) | 32.8 MiB (MAD 0.4 MiB) | 75.9 MiB (MAD 0.2 MiB) | 314.7 ms (MAD 3.2 ms) | 147.7 ms (MAD 2.4 ms) |
| duckdb-parquet-matched | yes | 10.072 MB | 299.6 ms (MAD 2.8 ms) | 125.9 ms (MAD 0.9 ms) | 131.8 ms (MAD 1.6 ms) | 22.9 ms (MAD 0.6 ms) | 27.2 ms (MAD 0.9 ms) | 17.2 MiB (MAD 0.2 MiB) | 72.9 MiB (MAD 0.2 MiB) | 289.1 ms (MAD 2.5 ms) | 124.0 ms (MAD 2.1 ms) |
| duckdb-parquet-zstd | yes | 7.629 MB | 395.8 ms (MAD 2.4 ms) | 139.9 ms (MAD 0.8 ms) | 144.2 ms (MAD 1.7 ms) | 23.8 ms (MAD 0.5 ms) | 27.5 ms (MAD 0.5 ms) | 18.5 MiB (MAD 0.2 MiB) | 72.9 MiB (MAD 0.2 MiB) | 385.2 ms (MAD 1.6 ms) | 140.0 ms (MAD 2.8 ms) |
| gqodb | yes | 6.212 MB | 231.5 ms (MAD 3.6 ms) | 66.7 ms (MAD 0.6 ms) | 70.5 ms (MAD 0.5 ms) | 3.3 ms (MAD 0.0 ms) | 4.1 ms (MAD 0.1 ms) | 72.5 MiB (MAD 0.1 MiB) | 68.2 MiB (MAD 0.1 MiB) | 221.9 ms (MAD 2.5 ms) | 65.7 ms (MAD 0.6 ms) |
| parquet-delta-bss-brotli5 | yes | 11.261 MB | 1,501.8 ms (MAD 16.0 ms) | 319.7 ms (MAD 3.2 ms) | 327.7 ms (MAD 3.0 ms) | 12.5 ms (MAD 0.1 ms) | 13.5 ms (MAD 0.1 ms) | 87.9 MiB (MAD 0.1 MiB) | 73.1 MiB (MAD 0.0 MiB) | 1,486.2 ms (MAD 14.7 ms) | 318.7 ms (MAD 2.8 ms) |
| parquet-delta-bss-lz4 | yes | 15.348 MB | 256.4 ms (MAD 2.2 ms) | 79.1 ms (MAD 0.6 ms) | 87.1 ms (MAD 0.8 ms) | 4.1 ms (MAD 0.1 ms) | 5.2 ms (MAD 0.1 ms) | 72.0 MiB (MAD 0.1 MiB) | 69.2 MiB (MAD 0.1 MiB) | 239.4 ms (MAD 2.7 ms) | 78.3 ms (MAD 0.5 ms) |
| parquet-delta-bss-snappy | yes | 15.256 MB | 255.7 ms (MAD 0.8 ms) | 82.9 ms (MAD 0.7 ms) | 90.5 ms (MAD 0.8 ms) | 4.2 ms (MAD 0.1 ms) | 5.2 ms (MAD 0.0 ms) | 72.1 MiB (MAD 0.1 MiB) | 68.9 MiB (MAD 0.1 MiB) | 239.0 ms (MAD 1.2 ms) | 81.6 ms (MAD 0.7 ms) |
| parquet-dictionary-lz4 | yes | 28.791 MB | 543.6 ms (MAD 4.0 ms) | 104.2 ms (MAD 0.7 ms) | 109.7 ms (MAD 0.6 ms) | 5.0 ms (MAD 0.0 ms) | 6.4 ms (MAD 0.0 ms) | 80.0 MiB (MAD 0.1 MiB) | 70.1 MiB (MAD 0.1 MiB) | 515.7 ms (MAD 4.5 ms) | 102.8 ms (MAD 1.0 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.257 [0.257, 0.257] BETTER | 0.473 [0.468, 0.478] BETTER | 0.272 [0.261, 0.277] BETTER | 0.282 [0.277, 0.286] BETTER | 0.419 [0.409, 0.607] BETTER | 0.400 [0.316, 0.570] BETTER | 0.300 [0.293, 0.301] BETTER | 0.898 [0.896, 0.898] BETTER | 0.528 [0.521, 0.539] BETTER | 0.210 [0.203, 0.214] BETTER |
| clickhouse-tuned | 0.603 [0.603, 0.603] BETTER | 0.426 [0.421, 0.433] BETTER | 0.235 [0.227, 0.238] BETTER | 0.244 [0.238, 0.248] BETTER | 0.631 [0.334, 1.139] INCONCLUSIVE | 0.368 [0.320, 0.434] BETTER | 0.327 [0.324, 0.328] BETTER | 0.876 [0.875, 0.877] BETTER | 0.457 [0.456, 0.465] BETTER | 0.187 [0.185, 0.190] BETTER |
| duckdb-native | 0.676 [0.676, 0.676] BETTER | 0.409 [0.405, 0.413] BETTER | 0.617 [0.613, 0.625] BETTER | 0.605 [0.601, 0.613] BETTER | 0.150 [0.145, 0.155] BETTER | 0.170 [0.168, 0.172] BETTER | 0.913 [0.909, 0.922] EQUIVALENT | 0.864 [0.862, 0.864] BETTER | 0.403 [0.401, 0.410] BETTER | 0.607 [0.592, 0.618] BETTER |
| duckdb-parquet-default | 0.290 [0.290, 0.290] BETTER | 0.689 [0.680, 0.698] BETTER | 0.446 [0.444, 0.453] BETTER | 0.431 [0.429, 0.437] BETTER | 0.114 [0.110, 0.116] BETTER | 0.129 [0.124, 0.131] BETTER | 2.210 [2.178, 2.222] WORSE | 0.898 [0.894, 0.900] BETTER | 0.705 [0.700, 0.718] BETTER | 0.445 [0.433, 0.454] BETTER |
| duckdb-parquet-matched | 0.617 [0.617, 0.617] BETTER | 0.773 [0.765, 0.778] BETTER | 0.530 [0.525, 0.536] BETTER | 0.535 [0.528, 0.546] BETTER | 0.143 [0.137, 0.146] BETTER | 0.151 [0.145, 0.156] BETTER | 4.203 [4.161, 4.258] WORSE | 0.935 [0.933, 0.937] EQUIVALENT | 0.768 [0.765, 0.781] BETTER | 0.530 [0.521, 0.540] BETTER |
| duckdb-parquet-zstd | 0.814 [0.814, 0.814] BETTER | 0.585 [0.582, 0.591] BETTER | 0.477 [0.474, 0.482] BETTER | 0.489 [0.485, 0.496] BETTER | 0.137 [0.134, 0.140] BETTER | 0.150 [0.147, 0.152] BETTER | 3.921 [3.917, 3.971] WORSE | 0.935 [0.930, 0.937] EQUIVALENT | 0.576 [0.573, 0.585] BETTER | 0.469 [0.464, 0.484] BETTER |
| parquet-delta-bss-brotli5 | 0.552 [0.552, 0.552] BETTER | 0.154 [0.153, 0.156] BETTER | 0.209 [0.206, 0.211] BETTER | 0.215 [0.214, 0.219] BETTER | 0.261 [0.258, 0.266] BETTER | 0.304 [0.295, 0.308] BETTER | 0.824 [0.822, 0.828] BETTER | 0.933 [0.932, 0.934] EQUIVALENT | 0.149 [0.149, 0.152] BETTER | 0.206 [0.204, 0.209] BETTER |
| parquet-delta-bss-lz4 | 0.405 [0.405, 0.405] BETTER | 0.903 [0.899, 0.912] BETTER | 0.843 [0.828, 0.845] BETTER | 0.810 [0.806, 0.820] BETTER | 0.798 [0.795, 0.807] BETTER | 0.788 [0.731, 0.797] BETTER | 1.007 [1.005, 1.010] EQUIVALENT | 0.985 [0.983, 0.986] EQUIVALENT | 0.927 [0.925, 0.943] BETTER | 0.839 [0.838, 0.853] BETTER |
| parquet-delta-bss-snappy | 0.407 [0.407, 0.407] BETTER | 0.905 [0.899, 0.915] BETTER | 0.805 [0.800, 0.814] BETTER | 0.779 [0.776, 0.790] BETTER | 0.784 [0.772, 0.800] BETTER | 0.796 [0.775, 0.805] BETTER | 1.004 [1.003, 1.008] EQUIVALENT | 0.989 [0.988, 0.990] EQUIVALENT | 0.929 [0.923, 0.944] BETTER | 0.805 [0.799, 0.817] BETTER |
| parquet-dictionary-lz4 | 0.216 [0.216, 0.216] BETTER | 0.426 [0.424, 0.430] BETTER | 0.640 [0.636, 0.648] BETTER | 0.643 [0.641, 0.650] BETTER | 0.659 [0.654, 0.664] BETTER | 0.648 [0.637, 0.655] BETTER | 0.905 [0.904, 0.908] EQUIVALENT | 0.973 [0.972, 0.975] EQUIVALENT | 0.430 [0.427, 0.436] BETTER | 0.640 [0.633, 0.647] BETTER |

## D2/v2

Valid rounds 15 of 23 attempted; flagged rounds: 8.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 23.914 MB | 500.7 ms (MAD 6.2 ms) | 240.4 ms (MAD 2.9 ms) | 245.5 ms (MAD 6.6 ms) | 2.3 ms (MAD 2.3 ms) | 8.0 ms (MAD 4.5 ms) | 252.7 MiB (MAD 7.4 MiB) | 75.7 MiB (MAD 0.0 MiB) | 426.1 ms (MAD 3.8 ms) | 307.4 ms (MAD 5.0 ms) |
| clickhouse-tuned | yes | 7.594 MB | 529.4 ms (MAD 10.7 ms) | 285.0 ms (MAD 3.9 ms) | 290.6 ms (MAD 5.0 ms) | 5.5 ms (MAD 2.0 ms) | 6.9 ms (MAD 2.6 ms) | 220.4 MiB (MAD 7.4 MiB) | 77.8 MiB (MAD 0.2 MiB) | 473.3 ms (MAD 7.9 ms) | 356.2 ms (MAD 6.5 ms) |
| duckdb-native | yes | 13.644 MB | 523.3 ms (MAD 3.1 ms) | 111.0 ms (MAD 0.9 ms) | 125.4 ms (MAD 1.1 ms) | 21.8 ms (MAD 0.6 ms) | 25.1 ms (MAD 0.4 ms) | 81.4 MiB (MAD 0.3 MiB) | 83.1 MiB (MAD 0.3 MiB) | 501.9 ms (MAD 3.8 ms) | 111.9 ms (MAD 2.9 ms) |
| duckdb-parquet-default | yes | 21.679 MB | 321.9 ms (MAD 2.3 ms) | 151.0 ms (MAD 1.2 ms) | 165.3 ms (MAD 0.8 ms) | 29.0 ms (MAD 0.4 ms) | 32.7 ms (MAD 0.9 ms) | 33.1 MiB (MAD 0.4 MiB) | 76.1 MiB (MAD 0.1 MiB) | 300.9 ms (MAD 3.5 ms) | 149.6 ms (MAD 1.4 ms) |
| duckdb-parquet-matched | yes | 8.297 MB | 269.7 ms (MAD 1.5 ms) | 124.7 ms (MAD 1.4 ms) | 128.6 ms (MAD 1.1 ms) | 24.5 ms (MAD 0.9 ms) | 26.8 ms (MAD 0.4 ms) | 16.9 MiB (MAD 0.1 MiB) | 72.5 MiB (MAD 0.2 MiB) | 260.0 ms (MAD 1.5 ms) | 124.2 ms (MAD 1.8 ms) |
| duckdb-parquet-zstd | yes | 6.695 MB | 342.6 ms (MAD 4.2 ms) | 137.1 ms (MAD 0.9 ms) | 140.7 ms (MAD 0.8 ms) | 23.3 ms (MAD 0.5 ms) | 27.3 ms (MAD 1.1 ms) | 18.1 MiB (MAD 0.2 MiB) | 72.8 MiB (MAD 0.1 MiB) | 331.4 ms (MAD 1.8 ms) | 137.6 ms (MAD 2.6 ms) |
| gqodb | yes | 6.212 MB | 247.1 ms (MAD 2.0 ms) | 64.1 ms (MAD 0.4 ms) | 68.0 ms (MAD 0.3 ms) | 2.9 ms (MAD 0.0 ms) | 3.8 ms (MAD 0.1 ms) | 72.4 MiB (MAD 0.1 MiB) | 69.0 MiB (MAD 0.1 MiB) | 236.7 ms (MAD 1.1 ms) | 63.4 ms (MAD 0.5 ms) |
| parquet-delta-bss-brotli5 | yes | 7.463 MB | 1,017.2 ms (MAD 10.9 ms) | 232.0 ms (MAD 1.1 ms) | 234.7 ms (MAD 0.9 ms) | 9.6 ms (MAD 0.1 ms) | 10.6 ms (MAD 0.1 ms) | 87.5 MiB (MAD 0.1 MiB) | 73.3 MiB (MAD 0.1 MiB) | 1,008.0 ms (MAD 9.9 ms) | 231.1 ms (MAD 1.6 ms) |
| parquet-delta-bss-lz4 | yes | 9.570 MB | 240.3 ms (MAD 1.7 ms) | 68.6 ms (MAD 0.4 ms) | 71.1 ms (MAD 0.5 ms) | 3.4 ms (MAD 0.0 ms) | 4.5 ms (MAD 0.1 ms) | 70.6 MiB (MAD 0.1 MiB) | 69.8 MiB (MAD 0.0 MiB) | 227.7 ms (MAD 1.1 ms) | 67.8 ms (MAD 0.6 ms) |
| parquet-delta-bss-snappy | yes | 9.206 MB | 237.4 ms (MAD 1.3 ms) | 71.2 ms (MAD 0.2 ms) | 73.7 ms (MAD 0.6 ms) | 3.5 ms (MAD 0.0 ms) | 4.5 ms (MAD 0.0 ms) | 70.9 MiB (MAD 0.1 MiB) | 70.0 MiB (MAD 0.1 MiB) | 226.9 ms (MAD 0.7 ms) | 70.1 ms (MAD 0.4 ms) |
| parquet-dictionary-lz4 | yes | 29.268 MB | 539.4 ms (MAD 4.6 ms) | 104.2 ms (MAD 0.4 ms) | 110.7 ms (MAD 1.0 ms) | 5.0 ms (MAD 0.0 ms) | 6.4 ms (MAD 0.1 ms) | 78.0 MiB (MAD 0.1 MiB) | 70.5 MiB (MAD 0.1 MiB) | 510.3 ms (MAD 3.0 ms) | 103.2 ms (MAD 0.6 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.260 [0.260, 0.260] BETTER | 0.494 [0.488, 0.500] BETTER | 0.267 [0.262, 0.271] BETTER | 0.277 [0.268, 0.282] BETTER | 1.284 [0.321, 7.956] INCONCLUSIVE | 0.480 [0.295, 1.104] INCONCLUSIVE | 0.286 [0.283, 0.298] BETTER | 0.911 [0.910, 0.913] EQUIVALENT | 0.556 [0.553, 0.560] BETTER | 0.206 [0.205, 0.210] BETTER |
| clickhouse-tuned | 0.818 [0.818, 0.818] BETTER | 0.467 [0.452, 0.472] BETTER | 0.225 [0.222, 0.228] BETTER | 0.234 [0.232, 0.246] BETTER | 0.536 [0.350, 0.741] BETTER | 0.551 [0.500, 0.809] BETTER | 0.328 [0.324, 0.349] BETTER | 0.886 [0.884, 0.886] BETTER | 0.500 [0.489, 0.505] BETTER | 0.178 [0.176, 0.181] BETTER |
| duckdb-native | 0.455 [0.455, 0.455] BETTER | 0.472 [0.467, 0.476] BETTER | 0.578 [0.570, 0.585] BETTER | 0.542 [0.542, 0.554] BETTER | 0.135 [0.133, 0.138] BETTER | 0.152 [0.150, 0.156] BETTER | 0.888 [0.888, 0.893] BETTER | 0.830 [0.829, 0.832] BETTER | 0.472 [0.465, 0.476] BETTER | 0.566 [0.550, 0.579] BETTER |
| duckdb-parquet-default | 0.287 [0.287, 0.287] BETTER | 0.768 [0.759, 0.773] BETTER | 0.425 [0.421, 0.430] BETTER | 0.411 [0.409, 0.417] BETTER | 0.101 [0.100, 0.103] BETTER | 0.117 [0.115, 0.120] BETTER | 2.186 [2.167, 2.198] WORSE | 0.906 [0.905, 0.907] EQUIVALENT | 0.787 [0.783, 0.798] BETTER | 0.424 [0.421, 0.428] BETTER |
| duckdb-parquet-matched | 0.749 [0.749, 0.749] BETTER | 0.916 [0.913, 0.924] BETTER | 0.514 [0.509, 0.520] BETTER | 0.529 [0.524, 0.537] BETTER | 0.120 [0.116, 0.128] BETTER | 0.142 [0.141, 0.147] BETTER | 4.290 [4.275, 4.346] WORSE | 0.952 [0.949, 0.953] EQUIVALENT | 0.911 [0.905, 0.922] BETTER | 0.510 [0.503, 0.518] BETTER |
| duckdb-parquet-zstd | 0.928 [0.928, 0.928] BETTER | 0.721 [0.712, 0.730] BETTER | 0.468 [0.464, 0.472] BETTER | 0.483 [0.482, 0.488] BETTER | 0.126 [0.124, 0.130] BETTER | 0.140 [0.134, 0.143] BETTER | 4.003 [3.967, 4.097] WORSE | 0.948 [0.944, 0.949] EQUIVALENT | 0.714 [0.710, 0.719] BETTER | 0.460 [0.457, 0.470] BETTER |
| parquet-delta-bss-brotli5 | 0.832 [0.832, 0.832] BETTER | 0.243 [0.240, 0.245] BETTER | 0.276 [0.274, 0.277] BETTER | 0.290 [0.288, 0.292] BETTER | 0.306 [0.304, 0.308] BETTER | 0.360 [0.354, 0.367] BETTER | 0.827 [0.826, 0.829] BETTER | 0.940 [0.940, 0.942] EQUIVALENT | 0.235 [0.232, 0.237] BETTER | 0.274 [0.270, 0.277] BETTER |
| parquet-delta-bss-lz4 | 0.649 [0.649, 0.649] BETTER | 1.029 [1.025, 1.042] EQUIVALENT | 0.934 [0.928, 0.939] BETTER | 0.957 [0.949, 0.962] INCONCLUSIVE | 0.865 [0.861, 0.873] BETTER | 0.858 [0.832, 0.871] BETTER | 1.025 [1.024, 1.027] EQUIVALENT | 0.988 [0.987, 0.988] EQUIVALENT | 1.040 [1.033, 1.049] EQUIVALENT | 0.935 [0.923, 0.942] BETTER |
| parquet-delta-bss-snappy | 0.675 [0.675, 0.675] BETTER | 1.041 [1.037, 1.049] EQUIVALENT | 0.900 [0.893, 0.910] BETTER | 0.923 [0.920, 0.934] BETTER | 0.832 [0.827, 0.839] BETTER | 0.845 [0.828, 0.852] BETTER | 1.020 [1.020, 1.023] EQUIVALENT | 0.985 [0.985, 0.986] EQUIVALENT | 1.043 [1.039, 1.055] INCONCLUSIVE | 0.903 [0.896, 0.910] BETTER |
| parquet-dictionary-lz4 | 0.212 [0.212, 0.212] BETTER | 0.458 [0.453, 0.461] BETTER | 0.616 [0.612, 0.621] BETTER | 0.615 [0.577, 0.622] BETTER | 0.585 [0.572, 0.588] BETTER | 0.594 [0.445, 0.604] BETTER | 0.928 [0.927, 0.930] EQUIVALENT | 0.978 [0.976, 0.980] EQUIVALENT | 0.464 [0.461, 0.468] BETTER | 0.614 [0.608, 0.620] BETTER |

## S1/v1

Valid rounds 15 of 26 attempted; flagged rounds: 11.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 69.962 MB | 1,166.7 ms (MAD 5.2 ms) | 700.1 ms (MAD 8.9 ms) | 714.3 ms (MAD 7.7 ms) | 7.1 ms (MAD 3.6 ms) | 7.7 ms (MAD 3.3 ms) | 515.3 MiB (MAD 5.3 MiB) | 76.4 MiB (MAD 0.2 MiB) | 1,054.0 ms (MAD 6.1 ms) | 732.4 ms (MAD 7.6 ms) |
| clickhouse-tuned | yes | 27.626 MB | 1,336.0 ms (MAD 11.4 ms) | 821.0 ms (MAD 3.5 ms) | 830.8 ms (MAD 9.5 ms) | 4.5 ms (MAD 3.6 ms) | 6.9 ms (MAD 4.7 ms) | 489.0 MiB (MAD 1.5 MiB) | 78.2 MiB (MAD 0.2 MiB) | 1,249.0 ms (MAD 8.2 ms) | 857.7 ms (MAD 5.3 ms) |
| duckdb-native | yes | 27.013 MB | 1,594.4 ms (MAD 10.8 ms) | 282.7 ms (MAD 1.2 ms) | 312.5 ms (MAD 2.0 ms) | 23.3 ms (MAD 0.8 ms) | 26.2 ms (MAD 0.4 ms) | 109.8 MiB (MAD 0.9 MiB) | 218.8 MiB (MAD 0.2 MiB) | 1,557.5 ms (MAD 9.4 ms) | 282.3 ms (MAD 3.1 ms) |
| duckdb-parquet-default | yes | 63.269 MB | 940.1 ms (MAD 7.5 ms) | 405.9 ms (MAD 1.5 ms) | 449.5 ms (MAD 3.6 ms) | 32.4 ms (MAD 1.1 ms) | 35.1 ms (MAD 0.9 ms) | 45.2 MiB (MAD 0.9 MiB) | 199.3 MiB (MAD 0.4 MiB) | 877.9 ms (MAD 4.2 ms) | 405.6 ms (MAD 4.4 ms) |
| gqodb | yes | 19.505 MB | 635.6 ms (MAD 3.1 ms) | 199.3 ms (MAD 0.6 ms) | 210.2 ms (MAD 0.9 ms) | 3.4 ms (MAD 0.0 ms) | 4.5 ms (MAD 0.1 ms) | 202.1 MiB (MAD 0.1 MiB) | 195.8 MiB (MAD 0.1 MiB) | 614.2 ms (MAD 2.1 ms) | 198.6 ms (MAD 0.8 ms) |
| parquet-delta-bss-lz4 | yes | 42.597 MB | 761.3 ms (MAD 2.2 ms) | 246.1 ms (MAD 0.8 ms) | 262.6 ms (MAD 0.9 ms) | 4.6 ms (MAD 0.1 ms) | 5.7 ms (MAD 0.1 ms) | 201.3 MiB (MAD 0.1 MiB) | 197.3 MiB (MAD 0.0 MiB) | 721.0 ms (MAD 1.3 ms) | 245.0 ms (MAD 1.2 ms) |
| parquet-dictionary-lz4 | yes | 85.580 MB | 1,602.5 ms (MAD 5.4 ms) | 310.6 ms (MAD 0.3 ms) | 325.9 ms (MAD 0.8 ms) | 5.5 ms (MAD 0.0 ms) | 6.9 ms (MAD 0.1 ms) | 207.9 MiB (MAD 0.1 MiB) | 199.2 MiB (MAD 0.1 MiB) | 1,528.1 ms (MAD 5.9 ms) | 309.2 ms (MAD 0.8 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.279 [0.279, 0.279] BETTER | 0.545 [0.541, 0.548] BETTER | 0.285 [0.281, 0.287] BETTER | 0.294 [0.291, 0.296] BETTER | 0.480 [0.344, 0.547] BETTER | 0.585 [0.377, 1.024] INCONCLUSIVE | 0.392 [0.380, 0.393] BETTER | 2.562 [2.560, 2.566] WORSE | 0.583 [0.577, 0.585] BETTER | 0.271 [0.268, 0.273] BETTER |
| clickhouse-tuned | 0.706 [0.706, 0.706] BETTER | 0.476 [0.473, 0.484] BETTER | 0.243 [0.240, 0.244] BETTER | 0.253 [0.249, 0.254] BETTER | 0.764 [0.407, 3.865] INCONCLUSIVE | 0.645 [0.378, 0.830] BETTER | 0.413 [0.411, 0.414] BETTER | 2.502 [2.494, 2.503] WORSE | 0.492 [0.487, 0.494] BETTER | 0.232 [0.229, 0.233] BETTER |
| duckdb-native | 0.722 [0.722, 0.722] BETTER | 0.399 [0.397, 0.402] BETTER | 0.705 [0.701, 0.708] BETTER | 0.672 [0.667, 0.678] BETTER | 0.147 [0.142, 0.152] BETTER | 0.171 [0.156, 0.172] BETTER | 1.842 [1.836, 1.859] WORSE | 0.895 [0.894, 0.895] BETTER | 0.394 [0.392, 0.397] BETTER | 0.704 [0.698, 0.709] BETTER |
| duckdb-parquet-default | 0.308 [0.308, 0.308] BETTER | 0.676 [0.671, 0.681] BETTER | 0.491 [0.490, 0.493] BETTER | 0.468 [0.462, 0.472] BETTER | 0.106 [0.104, 0.108] BETTER | 0.128 [0.124, 0.129] BETTER | 4.474 [4.395, 4.556] WORSE | 0.982 [0.982, 0.984] EQUIVALENT | 0.700 [0.695, 0.702] BETTER | 0.490 [0.484, 0.492] BETTER |
| parquet-delta-bss-lz4 | 0.458 [0.458, 0.458] BETTER | 0.835 [0.831, 0.836] BETTER | 0.810 [0.807, 0.814] BETTER | 0.800 [0.796, 0.803] BETTER | 0.750 [0.742, 0.763] BETTER | 0.785 [0.769, 0.787] BETTER | 1.004 [1.004, 1.004] EQUIVALENT | 0.992 [0.992, 0.992] EQUIVALENT | 0.852 [0.850, 0.854] BETTER | 0.811 [0.806, 0.816] BETTER |
| parquet-dictionary-lz4 | 0.228 [0.228, 0.228] BETTER | 0.397 [0.394, 0.397] BETTER | 0.642 [0.640, 0.644] BETTER | 0.645 [0.642, 0.646] BETTER | 0.629 [0.625, 0.634] BETTER | 0.651 [0.641, 0.656] BETTER | 0.972 [0.971, 0.972] EQUIVALENT | 0.983 [0.982, 0.983] EQUIVALENT | 0.402 [0.399, 0.404] BETTER | 0.642 [0.640, 0.644] BETTER |

## S1/v2

Valid rounds 15 of 17 attempted; flagged rounds: 2.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 69.371 MB | 1,158.6 ms (MAD 14.4 ms) | 688.9 ms (MAD 9.5 ms) | 700.2 ms (MAD 6.9 ms) | 4.4 ms (MAD 2.0 ms) | 7.4 ms (MAD 3.3 ms) | 516.1 MiB (MAD 3.3 MiB) | 76.2 MiB (MAD 0.1 MiB) | 1,043.9 ms (MAD 13.3 ms) | 719.4 ms (MAD 9.4 ms) |
| clickhouse-tuned | yes | 21.936 MB | 1,292.5 ms (MAD 11.7 ms) | 817.9 ms (MAD 6.4 ms) | 821.9 ms (MAD 6.9 ms) | 6.7 ms (MAD 2.3 ms) | 7.3 ms (MAD 6.3 ms) | 489.1 MiB (MAD 2.2 MiB) | 78.2 MiB (MAD 0.2 MiB) | 1,223.1 ms (MAD 15.4 ms) | 855.1 ms (MAD 8.9 ms) |
| duckdb-native | yes | 40.645 MB | 1,439.6 ms (MAD 12.7 ms) | 293.1 ms (MAD 2.8 ms) | 336.3 ms (MAD 1.8 ms) | 22.3 ms (MAD 1.0 ms) | 24.9 ms (MAD 0.3 ms) | 123.1 MiB (MAD 0.3 MiB) | 231.9 MiB (MAD 0.2 MiB) | 1,396.9 ms (MAD 14.0 ms) | 290.9 ms (MAD 5.2 ms) |
| duckdb-parquet-default | yes | 63.907 MB | 888.7 ms (MAD 6.4 ms) | 410.2 ms (MAD 4.3 ms) | 449.3 ms (MAD 2.7 ms) | 32.3 ms (MAD 0.6 ms) | 34.9 ms (MAD 0.8 ms) | 44.1 MiB (MAD 1.5 MiB) | 199.0 MiB (MAD 0.3 MiB) | 823.2 ms (MAD 6.3 ms) | 406.1 ms (MAD 7.1 ms) |
| gqodb | yes | 19.471 MB | 690.1 ms (MAD 8.4 ms) | 191.0 ms (MAD 1.5 ms) | 202.7 ms (MAD 1.4 ms) | 3.1 ms (MAD 0.0 ms) | 4.1 ms (MAD 0.0 ms) | 202.1 MiB (MAD 0.1 MiB) | 195.9 MiB (MAD 0.1 MiB) | 667.5 ms (MAD 5.7 ms) | 189.4 ms (MAD 1.2 ms) |
| parquet-delta-bss-lz4 | yes | 26.809 MB | 698.2 ms (MAD 3.3 ms) | 202.4 ms (MAD 1.6 ms) | 208.3 ms (MAD 2.0 ms) | 3.8 ms (MAD 0.1 ms) | 4.9 ms (MAD 0.0 ms) | 201.1 MiB (MAD 0.1 MiB) | 199.8 MiB (MAD 0.1 MiB) | 672.2 ms (MAD 3.7 ms) | 202.0 ms (MAD 2.6 ms) |
| parquet-dictionary-lz4 | yes | 86.663 MB | 1,620.7 ms (MAD 21.6 ms) | 308.3 ms (MAD 2.2 ms) | 325.2 ms (MAD 2.0 ms) | 5.4 ms (MAD 0.1 ms) | 6.8 ms (MAD 0.1 ms) | 208.9 MiB (MAD 0.1 MiB) | 198.0 MiB (MAD 0.1 MiB) | 1,547.5 ms (MAD 17.5 ms) | 308.0 ms (MAD 1.8 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.281 [0.281, 0.281] BETTER | 0.596 [0.586, 0.600] BETTER | 0.277 [0.275, 0.281] BETTER | 0.289 [0.286, 0.291] BETTER | 0.695 [0.524, 0.754] BETTER | 0.545 [0.431, 1.019] INCONCLUSIVE | 0.392 [0.381, 0.393] BETTER | 2.570 [2.565, 2.571] WORSE | 0.639 [0.630, 0.642] BETTER | 0.263 [0.261, 0.267] BETTER |
| clickhouse-tuned | 0.888 [0.888, 0.888] BETTER | 0.534 [0.529, 0.537] BETTER | 0.234 [0.231, 0.236] BETTER | 0.247 [0.244, 0.248] BETTER | 0.461 [0.435, 0.767] BETTER | 0.557 [0.289, 1.676] INCONCLUSIVE | 0.413 [0.412, 0.414] BETTER | 2.504 [2.504, 2.511] WORSE | 0.546 [0.542, 0.552] BETTER | 0.222 [0.219, 0.224] BETTER |
| duckdb-native | 0.479 [0.479, 0.479] BETTER | 0.479 [0.472, 0.483] BETTER | 0.652 [0.647, 0.661] BETTER | 0.603 [0.596, 0.606] BETTER | 0.138 [0.132, 0.140] BETTER | 0.163 [0.161, 0.166] BETTER | 1.642 [1.636, 1.646] WORSE | 0.845 [0.844, 0.846] BETTER | 0.478 [0.473, 0.482] BETTER | 0.651 [0.640, 0.661] BETTER |
| duckdb-parquet-default | 0.305 [0.305, 0.305] BETTER | 0.777 [0.767, 0.782] BETTER | 0.466 [0.456, 0.469] BETTER | 0.451 [0.445, 0.454] BETTER | 0.095 [0.093, 0.096] BETTER | 0.116 [0.113, 0.118] BETTER | 4.586 [4.442, 4.737] WORSE | 0.985 [0.983, 0.986] EQUIVALENT | 0.811 [0.790, 0.815] BETTER | 0.467 [0.452, 0.473] BETTER |
| parquet-delta-bss-lz4 | 0.726 [0.726, 0.726] BETTER | 0.988 [0.974, 0.994] EQUIVALENT | 0.944 [0.934, 0.950] BETTER | 0.973 [0.962, 0.983] EQUIVALENT | 0.815 [0.798, 0.823] BETTER | 0.830 [0.824, 0.847] BETTER | 1.005 [1.004, 1.005] EQUIVALENT | 0.981 [0.980, 0.981] EQUIVALENT | 0.993 [0.983, 0.999] EQUIVALENT | 0.938 [0.928, 0.952] BETTER |
| parquet-dictionary-lz4 | 0.225 [0.225, 0.225] BETTER | 0.426 [0.421, 0.431] BETTER | 0.620 [0.613, 0.625] BETTER | 0.623 [0.618, 0.627] BETTER | 0.572 [0.564, 0.578] BETTER | 0.599 [0.571, 0.606] BETTER | 0.967 [0.966, 0.968] EQUIVALENT | 0.989 [0.989, 0.990] EQUIVALENT | 0.431 [0.428, 0.437] BETTER | 0.615 [0.612, 0.623] BETTER |

## S2/v1

Valid rounds 15 of 15 attempted; flagged rounds: 0.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 83.815 MB | 1,362.4 ms (MAD 9.1 ms) | 825.1 ms (MAD 7.9 ms) | 850.0 ms (MAD 6.1 ms) | 7.5 ms (MAD 3.9 ms) | 5.9 ms (MAD 4.9 ms) | 637.0 MiB (MAD 2.5 MiB) | 76.2 MiB (MAD 0.0 MiB) | 1,239.2 ms (MAD 6.5 ms) | 849.3 ms (MAD 9.1 ms) |
| clickhouse-tuned | yes | 34.483 MB | 1,538.1 ms (MAD 16.3 ms) | 983.7 ms (MAD 10.8 ms) | 994.5 ms (MAD 14.8 ms) | 5.4 ms (MAD 3.2 ms) | 6.4 ms (MAD 3.3 ms) | 574.3 MiB (MAD 6.4 MiB) | 78.5 MiB (MAD 0.1 MiB) | 1,458.4 ms (MAD 13.6 ms) | 1,020.0 ms (MAD 7.8 ms) |
| duckdb-native | yes | 31.470 MB | 1,833.3 ms (MAD 10.9 ms) | 328.0 ms (MAD 1.0 ms) | 363.4 ms (MAD 1.9 ms) | 21.3 ms (MAD 0.8 ms) | 23.9 ms (MAD 0.1 ms) | 110.8 MiB (MAD 0.3 MiB) | 253.5 MiB (MAD 0.2 MiB) | 1,793.3 ms (MAD 12.4 ms) | 327.6 ms (MAD 2.6 ms) |
| duckdb-parquet-default | yes | 75.027 MB | 1,122.1 ms (MAD 16.6 ms) | 476.0 ms (MAD 2.6 ms) | 520.5 ms (MAD 3.5 ms) | 33.6 ms (MAD 0.9 ms) | 35.4 ms (MAD 0.5 ms) | 47.2 MiB (MAD 0.9 MiB) | 232.4 MiB (MAD 0.3 MiB) | 1,048.0 ms (MAD 15.0 ms) | 473.3 ms (MAD 3.9 ms) |
| gqodb | yes | 22.169 MB | 747.3 ms (MAD 4.2 ms) | 229.4 ms (MAD 0.6 ms) | 241.3 ms (MAD 1.0 ms) | 3.4 ms (MAD 0.0 ms) | 4.4 ms (MAD 0.0 ms) | 234.4 MiB (MAD 0.1 MiB) | 229.0 MiB (MAD 0.1 MiB) | 724.8 ms (MAD 5.8 ms) | 228.3 ms (MAD 0.6 ms) |
| parquet-delta-bss-lz4 | yes | 52.118 MB | 851.1 ms (MAD 5.6 ms) | 278.6 ms (MAD 0.5 ms) | 300.6 ms (MAD 1.8 ms) | 4.6 ms (MAD 0.0 ms) | 5.8 ms (MAD 0.0 ms) | 233.1 MiB (MAD 0.1 MiB) | 229.3 MiB (MAD 0.1 MiB) | 801.3 ms (MAD 2.8 ms) | 278.2 ms (MAD 0.6 ms) |
| parquet-dictionary-lz4 | yes | 100.826 MB | 1,879.7 ms (MAD 11.3 ms) | 361.0 ms (MAD 0.8 ms) | 377.7 ms (MAD 1.2 ms) | 5.4 ms (MAD 0.1 ms) | 6.7 ms (MAD 0.1 ms) | 239.2 MiB (MAD 0.1 MiB) | 233.1 MiB (MAD 0.1 MiB) | 1,790.4 ms (MAD 9.2 ms) | 360.2 ms (MAD 1.7 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.264 [0.264, 0.264] BETTER | 0.549 [0.544, 0.558] BETTER | 0.278 [0.276, 0.282] BETTER | 0.284 [0.281, 0.286] BETTER | 0.448 [0.316, 0.951] BETTER | 0.743 [0.434, inf] INCONCLUSIVE | 0.368 [0.367, 0.368] BETTER | 3.004 [3.003, 3.005] WORSE | 0.585 [0.579, 0.592] BETTER | 0.269 [0.269, 0.270] BETTER |
| clickhouse-tuned | 0.643 [0.643, 0.643] BETTER | 0.486 [0.484, 0.491] BETTER | 0.233 [0.230, 0.235] BETTER | 0.243 [0.239, 0.244] BETTER | 0.622 [0.514, 1.333] INCONCLUSIVE | 0.692 [0.452, 0.746] BETTER | 0.408 [0.406, 0.424] BETTER | 2.918 [2.917, 2.927] WORSE | 0.497 [0.493, 0.505] BETTER | 0.224 [0.222, 0.226] BETTER |
| duckdb-native | 0.704 [0.704, 0.704] BETTER | 0.408 [0.406, 0.414] BETTER | 0.699 [0.697, 0.712] BETTER | 0.664 [0.663, 0.671] BETTER | 0.158 [0.150, 0.161] BETTER | 0.185 [0.182, 0.188] BETTER | 2.116 [2.104, 2.120] WORSE | 0.903 [0.903, 0.904] EQUIVALENT | 0.404 [0.400, 0.411] BETTER | 0.697 [0.695, 0.705] BETTER |
| duckdb-parquet-default | 0.295 [0.295, 0.295] BETTER | 0.666 [0.659, 0.679] BETTER | 0.482 [0.480, 0.492] BETTER | 0.464 [0.461, 0.467] BETTER | 0.100 [0.100, 0.104] BETTER | 0.124 [0.123, 0.126] BETTER | 4.960 [4.919, 5.071] WORSE | 0.985 [0.984, 0.986] EQUIVALENT | 0.692 [0.680, 0.699] BETTER | 0.482 [0.475, 0.486] BETTER |
| parquet-delta-bss-lz4 | 0.425 [0.425, 0.425] BETTER | 0.878 [0.874, 0.885] BETTER | 0.823 [0.821, 0.835] BETTER | 0.803 [0.788, 0.807] BETTER | 0.728 [0.722, 0.741] BETTER | 0.754 [0.737, 0.766] BETTER | 1.005 [1.005, 1.006] EQUIVALENT | 0.998 [0.998, 0.998] EQUIVALENT | 0.905 [0.899, 0.912] BETTER | 0.821 [0.819, 0.823] BETTER |
| parquet-dictionary-lz4 | 0.220 [0.220, 0.220] BETTER | 0.398 [0.393, 0.401] BETTER | 0.635 [0.634, 0.645] BETTER | 0.639 [0.635, 0.641] BETTER | 0.622 [0.618, 0.630] BETTER | 0.662 [0.652, 0.672] BETTER | 0.980 [0.979, 0.980] EQUIVALENT | 0.982 [0.982, 0.982] EQUIVALENT | 0.405 [0.400, 0.409] BETTER | 0.634 [0.631, 0.636] BETTER |

## S2/v2

Valid rounds 15 of 18 attempted; flagged rounds: 3.

| candidate | exact | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | yes | 82.760 MB | 1,341.6 ms (MAD 17.8 ms) | 803.7 ms (MAD 9.0 ms) | 827.5 ms (MAD 7.9 ms) | 4.0 ms (MAD 3.4 ms) | 12.5 ms (MAD 3.9 ms) | 635.9 MiB (MAD 3.0 MiB) | 76.2 MiB (MAD 0.1 MiB) | 1,221.5 ms (MAD 15.6 ms) | 831.8 ms (MAD 12.6 ms) |
| clickhouse-tuned | yes | 26.428 MB | 1,500.7 ms (MAD 14.0 ms) | 958.1 ms (MAD 8.2 ms) | 971.1 ms (MAD 10.1 ms) | 1.8 ms (MAD 1.8 ms) | 8.4 ms (MAD 2.4 ms) | 576.0 MiB (MAD 1.5 MiB) | 78.3 MiB (MAD 0.2 MiB) | 1,422.6 ms (MAD 13.6 ms) | 982.8 ms (MAD 7.7 ms) |
| duckdb-native | yes | 46.936 MB | 1,669.0 ms (MAD 28.4 ms) | 334.7 ms (MAD 3.9 ms) | 388.4 ms (MAD 7.3 ms) | 21.3 ms (MAD 0.7 ms) | 24.8 ms (MAD 1.3 ms) | 123.3 MiB (MAD 0.4 MiB) | 268.6 MiB (MAD 0.1 MiB) | 1,605.5 ms (MAD 19.9 ms) | 334.1 ms (MAD 2.9 ms) |
| duckdb-parquet-default | yes | 75.872 MB | 1,039.3 ms (MAD 15.0 ms) | 474.8 ms (MAD 3.8 ms) | 522.4 ms (MAD 6.7 ms) | 32.5 ms (MAD 0.9 ms) | 35.8 ms (MAD 0.9 ms) | 47.1 MiB (MAD 0.7 MiB) | 232.3 MiB (MAD 0.3 MiB) | 968.8 ms (MAD 11.9 ms) | 470.8 ms (MAD 2.7 ms) |
| gqodb | yes | 22.168 MB | 805.9 ms (MAD 6.9 ms) | 221.9 ms (MAD 1.0 ms) | 234.0 ms (MAD 2.3 ms) | 3.0 ms (MAD 0.0 ms) | 4.1 ms (MAD 0.0 ms) | 234.1 MiB (MAD 0.1 MiB) | 230.4 MiB (MAD 0.0 MiB) | 780.1 ms (MAD 5.1 ms) | 220.6 ms (MAD 1.7 ms) |
| parquet-delta-bss-lz4 | yes | 32.915 MB | 812.3 ms (MAD 6.4 ms) | 234.7 ms (MAD 2.1 ms) | 241.2 ms (MAD 1.5 ms) | 3.9 ms (MAD 0.0 ms) | 5.0 ms (MAD 0.0 ms) | 232.8 MiB (MAD 0.1 MiB) | 232.5 MiB (MAD 0.1 MiB) | 779.9 ms (MAD 5.1 ms) | 233.4 ms (MAD 1.8 ms) |
| parquet-dictionary-lz4 | yes | 102.275 MB | 1,822.6 ms (MAD 21.2 ms) | 362.8 ms (MAD 1.2 ms) | 379.2 ms (MAD 1.9 ms) | 5.4 ms (MAD 0.1 ms) | 6.7 ms (MAD 0.1 ms) | 240.5 MiB (MAD 0.1 MiB) | 232.1 MiB (MAD 0.1 MiB) | 1,725.2 ms (MAD 17.1 ms) | 361.6 ms (MAD 1.8 ms) |

Ratios gqodb / baseline (median ratio [95% interval] verdict):

| baseline | bytes | write_op | read_full_warm_op | read_full_cold_op | read_range_warm_op | read_range_cold_op | rss_write | rss_full | cpu_write | cpu_full |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| clickhouse-default | 0.268 [0.268, 0.268] BETTER | 0.601 [0.594, 0.603] BETTER | 0.276 [0.272, 0.278] BETTER | 0.283 [0.280, 0.286] BETTER | 0.759 [0.412, 2.136] INCONCLUSIVE | 0.324 [0.317, 0.988] BETTER | 0.368 [0.368, 0.370] BETTER | 3.025 [3.023, 3.031] WORSE | 0.639 [0.634, 0.645] BETTER | 0.265 [0.263, 0.269] BETTER |
| clickhouse-tuned | 0.839 [0.839, 0.839] BETTER | 0.537 [0.532, 0.540] BETTER | 0.232 [0.230, 0.232] BETTER | 0.241 [0.238, 0.244] BETTER | 1.673 [0.266, 3.023] INCONCLUSIVE | 0.480 [0.351, 0.551] BETTER | 0.406 [0.405, 0.407] BETTER | 2.944 [2.939, 2.945] WORSE | 0.548 [0.544, 0.552] BETTER | 0.224 [0.223, 0.225] BETTER |
| duckdb-native | 0.472 [0.472, 0.472] BETTER | 0.483 [0.481, 0.488] BETTER | 0.663 [0.656, 0.665] BETTER | 0.602 [0.570, 0.608] BETTER | 0.143 [0.137, 0.147] BETTER | 0.163 [0.149, 0.170] BETTER | 1.898 [1.892, 1.899] WORSE | 0.858 [0.858, 0.858] BETTER | 0.486 [0.482, 0.491] BETTER | 0.660 [0.656, 0.662] BETTER |
| duckdb-parquet-default | 0.292 [0.292, 0.292] BETTER | 0.775 [0.769, 0.781] BETTER | 0.467 [0.464, 0.472] BETTER | 0.448 [0.442, 0.449] BETTER | 0.094 [0.087, 0.094] BETTER | 0.113 [0.112, 0.117] BETTER | 4.972 [4.850, 4.994] WORSE | 0.992 [0.990, 0.993] EQUIVALENT | 0.805 [0.795, 0.811] BETTER | 0.469 [0.464, 0.471] BETTER |
| parquet-delta-bss-lz4 | 0.674 [0.674, 0.674] BETTER | 0.992 [0.985, 0.994] EQUIVALENT | 0.946 [0.940, 0.956] BETTER | 0.970 [0.966, 0.987] EQUIVALENT | 0.783 [0.771, 0.785] BETTER | 0.806 [0.794, 0.822] BETTER | 1.006 [1.005, 1.006] EQUIVALENT | 0.991 [0.991, 0.991] EQUIVALENT | 1.000 [0.995, 1.005] EQUIVALENT | 0.945 [0.943, 0.957] BETTER |
| parquet-dictionary-lz4 | 0.217 [0.217, 0.217] BETTER | 0.442 [0.439, 0.445] BETTER | 0.612 [0.610, 0.615] BETTER | 0.617 [0.609, 0.622] BETTER | 0.566 [0.556, 0.568] BETTER | 0.606 [0.599, 0.613] BETTER | 0.974 [0.973, 0.974] EQUIVALENT | 0.993 [0.992, 0.993] EQUIVALENT | 0.452 [0.449, 0.455] BETTER | 0.610 [0.608, 0.617] BETTER |

## Headline claims (protocol section 10)

| baseline | bytes | write_op | read_full_warm_op | read_range_warm_op | rss_write | rss_full |
| --- | --- | --- | --- | --- | --- | --- |
| parquet-delta-bss-lz4 | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | EQUIVALENT / EQUIVALENT |
| parquet-dictionary-lz4 | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | EQUIVALENT / EQUIVALENT |
| duckdb-native | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | EQUIVALENT / EQUIVALENT | BETTER / BETTER **win** |
| duckdb-parquet-default | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | WORSE / WORSE | BETTER / BETTER **win** |
| clickhouse-default | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** |
| clickhouse-tuned | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / BETTER **win** | BETTER / INCONCLUSIVE | BETTER / BETTER **win** | BETTER / BETTER **win** |

GQODB wins every headline axis against every headline candidate: **False**.

## What this does not show

Aggregate trades only, one instrument, two months, one host class, one thread, no concurrency, no network; a young prototype against mature products; defaults plus one pre-registered alternative per tool; embedded libraries against a process per statement for ClickHouse. Full list: protocol section 11.

## Interpretation

**Result in one sentence.** On this input and these settings GQODB is smaller and faster than every other
file layout on write, full read and the 60-second range read, with a few exceptions listed below; it
does not win every headline axis, and its memory use is not better than the alternatives. The claim
"wins every axis" is **false** (headline verdict above).

**Exactness.** All 10,262 adapter runs finished (0 process failures) and every read returned the
reference table bit for bit (0 digest mismatches, 0 EXACTNESS FAIL cells), for GQODB and all ten baselines, both
variants, both slices and both full months. No cell is NOT COMPARABLE: the files were evicted with
`posix_fadvise` and `mincore` showed 0% residency before every cold read. "Cold" here means evicted from
the operating-system page cache of a fast local disk; the cold and warm figures are close for these
file sizes and this is not a cold-device measurement.

**Headline cells that are not BETTER (V1, D1 and D2, GQODB / baseline).**

* Peak memory of the write, against the three DuckDB Parquet writers: WORSE on both slices
  (about 72 MiB for GQODB against about 17 to 34 MiB; ratios 2.2 against default, 4.2 against 32,768-row
  LZ4, 3.9 against zstd). The cause lies in the measured adapter path and not in a step that is specific
  to the GQODB writer: the Rust adapter (`trades_candidate`, phase `write`) reads the whole source table
  into memory before it writes, for every Rust candidate including the Rust Parquet writers, which is
  why those sit at the same level; DuckDB streams its input. Against `duckdb-native` and the Rust Parquet
  writers the write memory is EQUIVALENT (ratio 0.91 to 1.01; BETTER against the brotli writer).
* Peak memory of the full read, against the Rust Parquet configurations: EQUIVALENT (0.93 to 0.99).
* Warm range read against `clickhouse-tuned`: BETTER on D1 (0.463 [0.285, 0.641]) but INCONCLUSIVE on D2
  (0.631 [0.334, 1.139]); against `clickhouse-default` on D2 in V2 it is INCONCLUSIVE too. The range
  reads take 3 to 7 ms and the intervals are wide.

**Other losses and non-wins** (all cells are in the tables; none was removed).

* Full months (S1, S2, V1): write memory WORSE against `duckdb-native` (1.8 and 2.1) and
  `duckdb-parquet-default` (4.5 and 5.0); full-read memory WORSE against both ClickHouse configurations
  (2.5 to 3.0), where the ClickHouse figure is the engine process only and excludes the buffer in which the
  Python client receives the result (see "What this does not show": the two sides do not measure the
  same thing). Warm range read against `clickhouse-tuned` is INCONCLUSIVE in S1 (0.76 [0.41, 3.87]) and S2.
* V2: write time against `parquet-delta-bss-snappy` on D1 is WORSE (1.05, with interval above 1) and
  against `parquet-delta-bss-lz4` it is EQUIVALENT (1.03 to 1.04) on D1 and D2; the V2 table has no
  floats, which presumably removes the part of the advantage that comes from GQODB's exact decimal scaling of
  float columns (not isolated by an experiment); the full-month V2 writes against `parquet-delta-bss-lz4` are
  EQUIVALENT too (0.99). The secondary CPU-time metric of the write (`cpu_write`) against
  `parquet-delta-bss-snappy` is WORSE on D1 in V2 (1.067) and INCONCLUSIVE on D2 in V2 (1.043).
* End-to-end wall time (`*_e2e`, secondary) is not comparable for reads: it contains the time spent
  computing the exactness digest after the clock stopped, and that code differs per adapter (Rust for
  GQODB and Parquet, pyarrow for DuckDB and ClickHouse). On the full months the e2e warm full read is
  WORSE than DuckDB (1.1 to 1.3) while the operation time is BETTER (0.48 to 0.70); the e2e columns must not be used to
  rank engines.
* Time advantage is moderate against the strongest Parquet configuration (`parquet-delta-bss-lz4`,
  LZ4_RAW with delta and byte-stream-split): write 0.90 (D1) and 0.84 to 0.88 on full months, full read
  0.81 to 0.84, range read about 0.7 to 0.8. The large ratios against other baselines come mostly from
  the baselines' defaults (dictionary encoding of floats, uncompressed or general-purpose codecs,
  process start of ClickHouse subtracted but its engine work included) and, for the ClickHouse full read
  (0.22 to 0.29, also against the pre-registered tuned chain at 0.22 to 0.24), from its process model: one
  process per statement that hands the result to the Python client as Arrow IPC through a pipe.
* Size: GQODB is the smallest layout in every slice and month (6.51 MB on D1 against 6.70 MB for DuckDB
  Parquet with zstd, 0.972, BETTER only by a small margin; 6.21 against 7.63 MB on D2; 19.5 against 27.0 MB
  for the best baseline, `duckdb-native`, on S1). The ClickHouse default layout is 3.6 to 3.9 times larger than GQODB
  and the tuned chain 1.4 to 1.7 times larger. The tuned ClickHouse chain and the DuckDB settings are
  the single pre-registered alternatives, not the result of a search.
* One byte of nondeterminism was seen: the `clickhouse-tuned` size on S1/V1 differed by 1 byte between
  two rounds (median used).
* The 60-second window holds 45 rows (D1) and 468 rows (D2): the "range read" is a very small query.
* Range reads against ClickHouse are noisy. Its operation time is the wall time of the statement process
  minus the wall time of a preceding `SELECT 1` process, floored at zero; 76 of the 712 ClickHouse
  range-read samples (warm and cold, all rounds) record an operation time of 0 ns. The warm range-read
  verdicts against ClickHouse (8 of the 16 cells are INCONCLUSIVE, none WORSE) therefore carry little
  information in either direction, including the BETTER ones.

**Flagged rounds.** Of 154 measured rounds attempted across the 8 cells (plus 3 warm-up rounds each), 34 were flagged and repeated: 2 (D1/v1),
7 (D1/v2), 1 (D2/v1), 8 (D2/v2), 11 (S1/v1), 2 (S1/v2), 0 (S2/v1), 3 (S2/v2). 133 samples carried the
`sibling_busy` flag (foreign processes landing on the hyperthread sibling of the benchmark core, in bursts)
and 4 the `low_cpu_ratio` flag. Every cell reached the 15 unflagged measured rounds, so no cell is labelled
UNDERPOWERED, but only under the thresholds and the repeat budget of addendum A-004 (see "Deviations from the
protocol": with the registered thresholds every cell would have been UNDERPOWERED).
The sensitivity analysis over all measured rounds (`sensitivity_all_rounds` in `summary.json`) changes 6
verdicts out of 864 comparisons and none on a headline axis: cold range read against `clickhouse-default`
(D1/v2, D2/v2, S1/v1, S1/v2), cold full read and CPU write against `parquet-delta-bss-lz4` (V2).

**Conditions.** One physical core pair (benchmark on one core, its sibling idle apart from the flagged
bursts), single thread. The fuzzing campaign of this release (`docs/FUZZING.md`) ran on four other cores at nice 19 during the whole
run, so the last-level cache and memory bandwidth were shared; its CPUs were declared foreign and their busy
time is stored per round (`foreign_busy`). Results are for this host class only.

## Deviations from the protocol

All deviations are recorded with their timing and what had been seen in `docs/B17_PUBLIC_TRADES_ADDENDA.md`:

* A-001 (pre-measurement): implementation clarifications of the frozen text (rule tie order, one extra ClickHouse
  ingest setting, startup subtraction details, noop per candidate, and others).
* A-002 (pre-measurement): the tooling was run once on the real input (data preparation only).
* A-003 (pre-measurement): owner decisions. An optional baseline that had been planned was dropped and the
  protocol file was edited once, only to delete that row and phrase, with a new pinned hash; Binance Vision data are
  used at the owner's own risk with attribution and aggregates only; the run was made resumable; the CPUs of the
  concurrent fuzz campaign were declared foreign cores for the quiet-box rule.
* A-004 (an amendment, not a clarification; made after a first start was aborted after five rounds of one cell):
  the sibling-busy threshold changed from max(2 jiffies, 2%) to max(5 jiffies, 10%), the CPU-ratio flag is only
  evaluated for samples of at least 100 ms, and the number of extra rounds allowed per cell rose from 5 to 25.
  On the 345 samples of the aborted start the registered rule flagged 17 (14 sibling-busy, 3 low CPU ratio) and
  the new rule flags 4 (3 and 1). The addendum states that the decision rested on flag statistics and not on
  timings; the aborted checkpoint held timings as well, it was not used for any number and is not published, and
  this statement cannot be checked from the repository. Applied to the stored samples, the registered thresholds
  would have left every cell UNDERPOWERED (0 to 8 valid rounds per cell even over all rounds attempted), and the
  new thresholds with the registered repeat budget (at most 20 measured rounds per cell) would have left three
  cells UNDERPOWERED (D1/v2 with 13 valid rounds, D2/v2 with 12, S1/v1 with 11). "No cell is UNDERPOWERED" above
  therefore depends on A-004.
