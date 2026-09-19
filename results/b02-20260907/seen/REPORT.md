# B02 adaptive block results

Synthetic warm in-memory blocks; owned integer columns in and out.
Five process repetitions; actual counts and seed offset are in results.json.
All custom blocks and five Parquet variants include complete framing and CRC.
No Zstd, real data, disk IO, durability or database functionality tested.

## New candidate screen

Ambitious gate: >=20% smaller AND >=2x faster full decode AND no encode regression.
Three-axis win: smaller AND faster encode AND faster decode.
Practical gate: >=20% smaller AND >=20% faster encode AND >=20% faster decode.
All-Parquet applies each criterion against every tested Parquet variant.
Small differences are descriptive, not statistically established wins.

| Case | Candidate | Bytes/row | Encode Mrow/s | Decode Mrow/s | Size / delta-LZ4 | Encode speedup / delta-LZ4 | Decode speedup / delta-LZ4 | All-Parquet three-axis win |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| L2/Quiet/262144 | AdaptiveSimd | 1.948 | 12.87 | 79.19 | 0.797 | 1.46x | 2.05x | NO |
| L2/Quiet/262144 | AdaptiveSimdLz4 | 1.661 | 12.81 | 78.99 | 0.680 | 1.45x | 2.04x | YES |
| L2/Quiet/4096 | AdaptiveSimd | 1.837 | 7.57 | 89.61 | 0.580 | 0.96x | 3.28x | NO |
| L2/Quiet/4096 | AdaptiveSimdLz4 | 1.623 | 7.45 | 89.03 | 0.512 | 0.95x | 3.26x | NO |
| L2/Quiet/65536 | AdaptiveSimd | 1.864 | 12.90 | 90.04 | 0.755 | 1.37x | 2.03x | NO |
| L2/Quiet/65536 | AdaptiveSimdLz4 | 1.581 | 12.77 | 91.54 | 0.640 | 1.36x | 2.06x | YES |
| L2/Volatile/262144 | AdaptiveSimd | 10.330 | 13.79 | 75.55 | 1.000 | 1.62x | 2.03x | NO |
| L2/Volatile/262144 | AdaptiveSimdLz4 | 9.918 | 13.22 | 75.88 | 0.960 | 1.55x | 2.04x | YES |
| L2/Volatile/4096 | AdaptiveSimd | 10.153 | 10.16 | 90.55 | 0.925 | 1.33x | 3.34x | YES |
| L2/Volatile/4096 | AdaptiveSimdLz4 | 10.040 | 9.77 | 87.87 | 0.914 | 1.28x | 3.24x | YES |
| L2/Volatile/65536 | AdaptiveSimd | 10.131 | 15.11 | 86.36 | 0.978 | 1.65x | 2.02x | YES |
| L2/Volatile/65536 | AdaptiveSimdLz4 | 9.923 | 14.68 | 87.32 | 0.958 | 1.60x | 2.05x | YES |
| Quote/Quiet/262144 | AdaptiveSimd | 1.189 | 12.20 | 70.60 | 0.358 | 1.51x | 3.67x | YES |
| Quote/Quiet/262144 | AdaptiveSimdLz4 | 1.026 | 12.05 | 25.55 | 0.309 | 1.49x | 1.33x | YES |
| Quote/Quiet/4096 | AdaptiveSimd | 1.278 | 7.42 | 42.12 | 0.320 | 1.11x | 2.38x | NO |
| Quote/Quiet/4096 | AdaptiveSimdLz4 | 1.163 | 7.31 | 40.66 | 0.291 | 1.10x | 2.30x | NO |
| Quote/Quiet/65536 | AdaptiveSimd | 1.193 | 12.24 | 25.25 | 0.357 | 1.51x | 1.37x | YES |
| Quote/Quiet/65536 | AdaptiveSimdLz4 | 1.033 | 12.10 | 26.22 | 0.309 | 1.49x | 1.43x | YES |
| Quote/Volatile/262144 | AdaptiveSimd | 10.955 | 13.64 | 68.27 | 0.789 | 1.77x | 3.21x | YES |
| Quote/Volatile/262144 | AdaptiveSimdLz4 | 10.675 | 12.91 | 69.47 | 0.769 | 1.67x | 3.26x | YES |
| Quote/Volatile/4096 | AdaptiveSimd | 10.778 | 10.13 | 82.33 | 0.744 | 1.34x | 2.98x | YES |
| Quote/Volatile/4096 | AdaptiveSimdLz4 | 10.769 | 9.66 | 80.80 | 0.743 | 1.28x | 2.92x | YES |
| Quote/Volatile/65536 | AdaptiveSimd | 10.756 | 15.02 | 77.53 | 0.774 | 1.68x | 1.83x | YES |
| Quote/Volatile/65536 | AdaptiveSimdLz4 | 10.678 | 14.50 | 75.68 | 0.768 | 1.63x | 1.78x | YES |
| Trade/Quiet/262144 | AdaptiveSimd | 0.939 | 12.80 | 73.31 | 0.489 | 1.47x | 1.93x | YES |
| Trade/Quiet/262144 | AdaptiveSimdLz4 | 0.776 | 12.69 | 72.57 | 0.404 | 1.45x | 1.91x | YES |
| Trade/Quiet/4096 | AdaptiveSimd | 1.028 | 7.36 | 81.44 | 0.396 | 0.94x | 2.88x | NO |
| Trade/Quiet/4096 | AdaptiveSimdLz4 | 0.913 | 7.28 | 79.18 | 0.351 | 0.92x | 2.80x | NO |
| Trade/Quiet/65536 | AdaptiveSimd | 0.943 | 12.68 | 81.98 | 0.486 | 1.36x | 1.79x | YES |
| Trade/Quiet/65536 | AdaptiveSimdLz4 | 0.783 | 12.63 | 84.11 | 0.403 | 1.35x | 1.84x | YES |
| Trade/Volatile/262144 | AdaptiveSimd | 9.955 | 13.53 | 69.44 | 0.987 | 1.57x | 1.85x | YES |
| Trade/Volatile/262144 | AdaptiveSimdLz4 | 9.667 | 13.04 | 70.98 | 0.958 | 1.52x | 1.89x | YES |
| Trade/Volatile/4096 | AdaptiveSimd | 9.778 | 10.03 | 82.37 | 0.913 | 1.29x | 2.94x | YES |
| Trade/Volatile/4096 | AdaptiveSimdLz4 | 9.764 | 9.66 | 82.12 | 0.912 | 1.24x | 2.93x | YES |
| Trade/Volatile/65536 | AdaptiveSimd | 9.756 | 14.96 | 80.58 | 0.965 | 1.63x | 1.85x | YES |
| Trade/Volatile/65536 | AdaptiveSimdLz4 | 9.671 | 14.55 | 79.04 | 0.957 | 1.58x | 1.81x | YES |

Matched delta-LZ4 ambitious gate: 11/36.
All-Parquet ambitious gate: 3/36.
Matched delta-LZ4 three-axis wins: 32/36.
All-Parquet three-axis wins: 27/36.
Matched delta-LZ4 practical gate: 18/36.
All-Parquet practical gate: 14/36.

## All variants

Rates are medians across process repetitions. Min/max and individual timings are
in results.json and trial files. Rates include format conversion and allocations.

| Case | Codec | Bytes/row | Encode Mrow/s | Decode Mrow/s | Decode min–max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| L2/Quiet/262144 | AdaptiveSimd | 1.948 | 12.87 | 79.19 | 3.226–9.863 |
| L2/Quiet/262144 | AdaptiveSimdLz4 | 1.661 | 12.81 | 78.99 | 3.174–9.925 |
| L2/Quiet/262144 | ColumnLz4 | 26.826 | 6.07 | 13.21 | 19.774–21.119 |
| L2/Quiet/262144 | DeltaBitpackLz4 | 2.415 | 21.94 | 17.87 | 14.432–21.665 |
| L2/Quiet/262144 | DeltaVarintLz4 | 2.568 | 24.58 | 31.42 | 8.335–15.353 |
| L2/Quiet/262144 | ParquetDeltaBrotli | 1.765 | 3.49 | 11.29 | 22.957–25.592 |
| L2/Quiet/262144 | ParquetDeltaLz4 | 2.444 | 8.83 | 38.68 | 6.513–7.215 |
| L2/Quiet/262144 | ParquetDeltaSnappy | 2.524 | 8.83 | 37.68 | 6.755–13.690 |
| L2/Quiet/262144 | ParquetDictionaryLz4 | 16.035 | 3.10 | 18.34 | 13.605–16.665 |
| L2/Quiet/262144 | ParquetPlainLz4 | 26.844 | 5.00 | 11.35 | 22.335–24.926 |
| L2/Quiet/4096 | AdaptiveSimd | 1.837 | 7.57 | 89.61 | 0.045–0.049 |
| L2/Quiet/4096 | AdaptiveSimdLz4 | 1.623 | 7.45 | 89.03 | 0.046–0.047 |
| L2/Quiet/4096 | ColumnLz4 | 27.006 | 6.48 | 14.11 | 0.290–0.293 |
| L2/Quiet/4096 | DeltaBitpackLz4 | 2.560 | 23.05 | 17.91 | 0.227–0.238 |
| L2/Quiet/4096 | DeltaVarintLz4 | 3.061 | 26.98 | 30.55 | 0.133–0.139 |
| L2/Quiet/4096 | ParquetDeltaBrotli | 2.753 | 2.46 | 2.07 | 1.965–2.032 |
| L2/Quiet/4096 | ParquetDeltaLz4 | 3.168 | 7.87 | 27.35 | 0.148–0.154 |
| L2/Quiet/4096 | ParquetDeltaSnappy | 3.266 | 8.02 | 27.47 | 0.147–0.151 |
| L2/Quiet/4096 | ParquetDictionaryLz4 | 27.932 | 2.97 | 11.47 | 0.357–0.365 |
| L2/Quiet/4096 | ParquetPlainLz4 | 27.591 | 5.14 | 10.66 | 0.381–0.400 |
| L2/Quiet/65536 | AdaptiveSimd | 1.864 | 12.90 | 90.04 | 0.685–0.815 |
| L2/Quiet/65536 | AdaptiveSimdLz4 | 1.581 | 12.77 | 91.54 | 0.695–0.773 |
| L2/Quiet/65536 | ColumnLz4 | 26.834 | 6.22 | 13.69 | 4.762–4.806 |
| L2/Quiet/65536 | DeltaBitpackLz4 | 2.427 | 24.16 | 18.08 | 3.580–3.856 |
| L2/Quiet/65536 | DeltaVarintLz4 | 2.588 | 27.03 | 31.10 | 2.069–2.147 |
| L2/Quiet/65536 | ParquetDeltaBrotli | 1.791 | 3.55 | 11.60 | 5.541–5.923 |
| L2/Quiet/65536 | ParquetDeltaLz4 | 2.468 | 9.39 | 44.37 | 1.453–1.592 |
| L2/Quiet/65536 | ParquetDeltaSnappy | 2.538 | 9.40 | 40.63 | 1.526–1.670 |
| L2/Quiet/65536 | ParquetDictionaryLz4 | 29.387 | 2.14 | 13.70 | 4.575–4.838 |
| L2/Quiet/65536 | ParquetPlainLz4 | 26.871 | 5.33 | 12.32 | 5.241–5.388 |
| L2/Volatile/262144 | AdaptiveSimd | 10.330 | 13.79 | 75.55 | 3.415–10.399 |
| L2/Volatile/262144 | AdaptiveSimdLz4 | 9.918 | 13.22 | 75.88 | 3.409–9.190 |
| L2/Volatile/262144 | ColumnLz4 | 29.905 | 5.37 | 13.27 | 19.650–21.237 |
| L2/Volatile/262144 | DeltaBitpackLz4 | 10.445 | 19.94 | 15.62 | 16.371–23.004 |
| L2/Volatile/262144 | DeltaVarintLz4 | 12.289 | 20.58 | 26.35 | 9.893–16.491 |
| L2/Volatile/262144 | ParquetDeltaBrotli | 10.205 | 3.42 | 10.29 | 24.139–27.580 |
| L2/Volatile/262144 | ParquetDeltaLz4 | 10.334 | 8.52 | 37.19 | 6.810–12.983 |
| L2/Volatile/262144 | ParquetDeltaSnappy | 10.343 | 8.52 | 37.15 | 6.827–7.800 |
| L2/Volatile/262144 | ParquetDictionaryLz4 | 24.672 | 2.56 | 15.44 | 15.287–19.832 |
| L2/Volatile/262144 | ParquetPlainLz4 | 29.927 | 4.51 | 11.62 | 22.371–22.734 |
| L2/Volatile/4096 | AdaptiveSimd | 10.153 | 10.16 | 90.55 | 0.045–0.047 |
| L2/Volatile/4096 | AdaptiveSimdLz4 | 10.040 | 9.77 | 87.87 | 0.045–0.047 |
| L2/Volatile/4096 | ColumnLz4 | 29.988 | 5.78 | 13.82 | 0.292–0.301 |
| L2/Volatile/4096 | DeltaBitpackLz4 | 10.500 | 19.30 | 15.92 | 0.254–0.265 |
| L2/Volatile/4096 | DeltaVarintLz4 | 12.423 | 21.73 | 26.52 | 0.154–0.159 |
| L2/Volatile/4096 | ParquetDeltaBrotli | 10.846 | 2.01 | 2.92 | 1.395–1.433 |
| L2/Volatile/4096 | ParquetDeltaLz4 | 10.981 | 7.66 | 27.10 | 0.149–0.152 |
| L2/Volatile/4096 | ParquetDeltaSnappy | 10.969 | 7.67 | 26.65 | 0.152–0.160 |
| L2/Volatile/4096 | ParquetDictionaryLz4 | 37.752 | 2.55 | 10.09 | 0.401–0.417 |
| L2/Volatile/4096 | ParquetPlainLz4 | 30.573 | 4.66 | 10.67 | 0.382–0.386 |
| L2/Volatile/65536 | AdaptiveSimd | 10.131 | 15.11 | 86.36 | 0.731–0.774 |
| L2/Volatile/65536 | AdaptiveSimdLz4 | 9.923 | 14.68 | 87.32 | 0.735–0.779 |
| L2/Volatile/65536 | ColumnLz4 | 29.914 | 5.39 | 13.60 | 4.774–5.126 |
| L2/Volatile/65536 | DeltaBitpackLz4 | 10.448 | 21.63 | 15.96 | 4.061–4.126 |
| L2/Volatile/65536 | DeltaVarintLz4 | 12.284 | 22.58 | 26.33 | 2.466–2.715 |
| L2/Volatile/65536 | ParquetDeltaBrotli | 10.228 | 3.52 | 10.94 | 5.903–6.317 |
| L2/Volatile/65536 | ParquetDeltaLz4 | 10.357 | 9.15 | 42.65 | 1.518–1.761 |
| L2/Volatile/65536 | ParquetDeltaSnappy | 10.366 | 9.25 | 41.82 | 1.502–1.776 |
| L2/Volatile/65536 | ParquetDictionaryLz4 | 36.774 | 1.81 | 12.51 | 5.180–5.570 |
| L2/Volatile/65536 | ParquetPlainLz4 | 29.951 | 4.79 | 12.17 | 5.353–5.714 |
| Quote/Quiet/262144 | AdaptiveSimd | 1.189 | 12.20 | 70.60 | 3.657–10.552 |
| Quote/Quiet/262144 | AdaptiveSimdLz4 | 1.026 | 12.05 | 25.55 | 3.686–10.426 |
| Quote/Quiet/262144 | ColumnLz4 | 29.342 | 4.60 | 10.61 | 22.965–25.174 |
| Quote/Quiet/262144 | DeltaBitpackLz4 | 3.278 | 21.33 | 17.54 | 14.933–21.845 |
| Quote/Quiet/262144 | DeltaVarintLz4 | 3.992 | 21.12 | 20.53 | 9.569–13.086 |
| Quote/Quiet/262144 | ParquetDeltaBrotli | 2.597 | 2.95 | 7.12 | 36.414–36.994 |
| Quote/Quiet/262144 | ParquetDeltaLz4 | 3.323 | 8.08 | 19.25 | 12.848–14.037 |
| Quote/Quiet/262144 | ParquetDeltaSnappy | 3.438 | 8.32 | 19.67 | 9.331–18.377 |
| Quote/Quiet/262144 | ParquetDictionaryLz4 | 17.311 | 2.51 | 13.94 | 18.060–25.996 |
| Quote/Quiet/262144 | ParquetPlainLz4 | 29.361 | 3.68 | 8.70 | 26.964–30.222 |
| Quote/Quiet/4096 | AdaptiveSimd | 1.278 | 7.42 | 42.12 | 0.050–0.118 |
| Quote/Quiet/4096 | AdaptiveSimdLz4 | 1.163 | 7.31 | 40.66 | 0.051–0.128 |
| Quote/Quiet/4096 | ColumnLz4 | 29.504 | 5.99 | 14.40 | 0.280–0.319 |
| Quote/Quiet/4096 | DeltaBitpackLz4 | 3.389 | 22.05 | 17.33 | 0.236–0.306 |
| Quote/Quiet/4096 | DeltaVarintLz4 | 4.602 | 21.51 | 27.44 | 0.142–0.205 |
| Quote/Quiet/4096 | ParquetDeltaBrotli | 3.562 | 1.91 | 1.95 | 2.095–3.560 |
| Quote/Quiet/4096 | ParquetDeltaLz4 | 3.994 | 6.66 | 17.68 | 0.230–0.237 |
| Quote/Quiet/4096 | ParquetDeltaSnappy | 4.143 | 7.53 | 25.04 | 0.147–0.235 |
| Quote/Quiet/4096 | ParquetDictionaryLz4 | 29.311 | 2.09 | 8.33 | 0.402–0.520 |
| Quote/Quiet/4096 | ParquetPlainLz4 | 30.096 | 4.04 | 7.91 | 0.469–0.542 |
| Quote/Quiet/65536 | AdaptiveSimd | 1.193 | 12.24 | 25.25 | 0.816–2.658 |
| Quote/Quiet/65536 | AdaptiveSimdLz4 | 1.033 | 12.10 | 26.22 | 0.816–2.590 |
| Quote/Quiet/65536 | ColumnLz4 | 29.352 | 4.73 | 12.94 | 4.737–5.135 |
| Quote/Quiet/65536 | DeltaBitpackLz4 | 3.287 | 22.87 | 17.67 | 3.707–5.940 |
| Quote/Quiet/65536 | DeltaVarintLz4 | 4.010 | 21.94 | 29.60 | 2.206–3.136 |
| Quote/Quiet/65536 | ParquetDeltaBrotli | 2.619 | 2.50 | 6.66 | 9.482–10.153 |
| Quote/Quiet/65536 | ParquetDeltaLz4 | 3.343 | 8.13 | 18.38 | 3.403–3.704 |
| Quote/Quiet/65536 | ParquetDeltaSnappy | 3.429 | 8.44 | 18.64 | 3.462–3.751 |
| Quote/Quiet/65536 | ParquetDictionaryLz4 | 30.700 | 1.70 | 10.72 | 5.721–7.242 |
| Quote/Quiet/65536 | ParquetPlainLz4 | 29.389 | 3.76 | 6.72 | 8.599–10.280 |
| Quote/Volatile/262144 | AdaptiveSimd | 10.955 | 13.64 | 68.27 | 3.781–3.874 |
| Quote/Volatile/262144 | AdaptiveSimdLz4 | 10.675 | 12.91 | 69.47 | 3.713–4.302 |
| Quote/Volatile/262144 | ColumnLz4 | 37.160 | 4.39 | 13.26 | 19.351–22.504 |
| Quote/Volatile/262144 | DeltaBitpackLz4 | 13.985 | 17.98 | 14.09 | 18.396–19.013 |
| Quote/Volatile/262144 | DeltaVarintLz4 | 16.704 | 18.09 | 24.93 | 10.372–16.223 |
| Quote/Volatile/262144 | ParquetDeltaBrotli | 13.851 | 3.11 | 12.25 | 20.670–28.377 |
| Quote/Volatile/262144 | ParquetDeltaLz4 | 13.879 | 7.73 | 21.28 | 11.267–13.407 |
| Quote/Volatile/262144 | ParquetDeltaSnappy | 13.881 | 8.31 | 36.98 | 6.930–11.841 |
| Quote/Volatile/262144 | ParquetDictionaryLz4 | 33.143 | 1.89 | 14.43 | 17.340–23.521 |
| Quote/Volatile/262144 | ParquetPlainLz4 | 37.187 | 3.51 | 11.52 | 22.438–28.417 |
| Quote/Volatile/4096 | AdaptiveSimd | 10.778 | 10.13 | 82.33 | 0.050–0.051 |
| Quote/Volatile/4096 | AdaptiveSimdLz4 | 10.769 | 9.66 | 80.80 | 0.050–0.052 |
| Quote/Volatile/4096 | ColumnLz4 | 37.379 | 4.87 | 14.42 | 0.283–0.296 |
| Quote/Volatile/4096 | DeltaBitpackLz4 | 14.010 | 17.68 | 14.35 | 0.285–0.296 |
| Quote/Volatile/4096 | DeltaVarintLz4 | 16.761 | 20.13 | 25.97 | 0.158–0.158 |
| Quote/Volatile/4096 | ParquetDeltaBrotli | 14.472 | 1.81 | 4.21 | 0.969–1.043 |
| Quote/Volatile/4096 | ParquetDeltaLz4 | 14.494 | 7.57 | 27.64 | 0.146–0.152 |
| Quote/Volatile/4096 | ParquetDeltaSnappy | 14.493 | 7.62 | 26.78 | 0.149–0.153 |
| Quote/Volatile/4096 | ParquetDictionaryLz4 | 49.751 | 2.20 | 8.83 | 0.463–0.465 |
| Quote/Volatile/4096 | ParquetPlainLz4 | 37.971 | 4.04 | 10.70 | 0.380–0.388 |
| Quote/Volatile/65536 | AdaptiveSimd | 10.756 | 15.02 | 77.53 | 0.840–0.878 |
| Quote/Volatile/65536 | AdaptiveSimdLz4 | 10.678 | 14.50 | 75.68 | 0.843–0.910 |
| Quote/Volatile/65536 | ColumnLz4 | 37.180 | 4.52 | 13.95 | 4.685–4.709 |
| Quote/Volatile/65536 | DeltaBitpackLz4 | 13.985 | 19.44 | 14.21 | 4.570–4.636 |
| Quote/Volatile/65536 | DeltaVarintLz4 | 16.704 | 20.67 | 25.49 | 2.564–2.585 |
| Quote/Volatile/65536 | ParquetDeltaBrotli | 13.872 | 3.21 | 12.43 | 5.169–5.364 |
| Quote/Volatile/65536 | ParquetDeltaLz4 | 13.902 | 8.92 | 42.48 | 1.519–1.574 |
| Quote/Volatile/65536 | ParquetDeltaSnappy | 13.904 | 8.98 | 42.90 | 1.512–1.615 |
| Quote/Volatile/65536 | ParquetDictionaryLz4 | 48.908 | 1.48 | 10.66 | 6.106–6.258 |
| Quote/Volatile/65536 | ParquetPlainLz4 | 37.218 | 4.11 | 12.08 | 5.369–5.497 |
| Trade/Quiet/262144 | AdaptiveSimd | 0.939 | 12.80 | 73.31 | 3.538–3.741 |
| Trade/Quiet/262144 | AdaptiveSimdLz4 | 0.776 | 12.69 | 72.57 | 3.523–3.694 |
| Trade/Quiet/262144 | ColumnLz4 | 29.928 | 5.44 | 11.80 | 22.080–22.666 |
| Trade/Quiet/262144 | DeltaBitpackLz4 | 1.892 | 22.34 | 18.36 | 14.100–14.743 |
| Trade/Quiet/262144 | DeltaVarintLz4 | 2.075 | 24.08 | 31.20 | 8.373–9.064 |
| Trade/Quiet/262144 | ParquetDeltaBrotli | 1.191 | 3.64 | 12.82 | 20.155–22.560 |
| Trade/Quiet/262144 | ParquetDeltaLz4 | 1.919 | 8.73 | 37.90 | 6.877–9.983 |
| Trade/Quiet/262144 | ParquetDeltaSnappy | 2.010 | 8.81 | 37.82 | 6.686–21.916 |
| Trade/Quiet/262144 | ParquetDictionaryLz4 | 18.744 | 2.70 | 16.19 | 14.779–18.010 |
| Trade/Quiet/262144 | ParquetPlainLz4 | 29.945 | 4.50 | 10.39 | 25.074–26.083 |
| Trade/Quiet/4096 | AdaptiveSimd | 1.028 | 7.36 | 81.44 | 0.049–0.052 |
| Trade/Quiet/4096 | AdaptiveSimdLz4 | 0.913 | 7.28 | 79.18 | 0.050–0.053 |
| Trade/Quiet/4096 | ColumnLz4 | 30.021 | 5.84 | 12.44 | 0.328–0.330 |
| Trade/Quiet/4096 | DeltaBitpackLz4 | 1.998 | 22.76 | 18.21 | 0.223–0.232 |
| Trade/Quiet/4096 | DeltaVarintLz4 | 2.687 | 25.33 | 30.14 | 0.135–0.139 |
| Trade/Quiet/4096 | ParquetDeltaBrotli | 2.140 | 2.55 | 2.11 | 1.923–1.963 |
| Trade/Quiet/4096 | ParquetDeltaLz4 | 2.600 | 7.87 | 28.32 | 0.144–0.150 |
| Trade/Quiet/4096 | ParquetDeltaSnappy | 2.740 | 7.92 | 27.66 | 0.144–0.150 |
| Trade/Quiet/4096 | ParquetDictionaryLz4 | 33.133 | 2.72 | 10.28 | 0.397–0.400 |
| Trade/Quiet/4096 | ParquetPlainLz4 | 30.610 | 4.72 | 9.72 | 0.419–0.424 |
| Trade/Quiet/65536 | AdaptiveSimd | 0.943 | 12.68 | 81.98 | 0.775–0.837 |
| Trade/Quiet/65536 | AdaptiveSimdLz4 | 0.783 | 12.63 | 84.11 | 0.773–0.816 |
| Trade/Quiet/65536 | ColumnLz4 | 29.933 | 5.58 | 12.18 | 5.371–5.391 |
| Trade/Quiet/65536 | DeltaBitpackLz4 | 1.904 | 24.14 | 18.09 | 3.532–3.810 |
| Trade/Quiet/65536 | DeltaVarintLz4 | 2.091 | 25.33 | 31.08 | 2.086–2.146 |
| Trade/Quiet/65536 | ParquetDeltaBrotli | 1.214 | 3.71 | 13.11 | 4.935–5.197 |
| Trade/Quiet/65536 | ParquetDeltaLz4 | 1.940 | 9.35 | 45.73 | 1.351–1.532 |
| Trade/Quiet/65536 | ParquetDeltaSnappy | 2.069 | 9.55 | 44.48 | 1.434–1.481 |
| Trade/Quiet/65536 | ParquetDictionaryLz4 | 34.782 | 1.87 | 12.25 | 5.325–5.467 |
| Trade/Quiet/65536 | ParquetPlainLz4 | 29.971 | 4.91 | 10.94 | 5.914–6.129 |
| Trade/Volatile/262144 | AdaptiveSimd | 9.955 | 13.53 | 69.44 | 3.678–10.737 |
| Trade/Volatile/262144 | AdaptiveSimdLz4 | 9.667 | 13.04 | 70.98 | 3.655–10.297 |
| Trade/Volatile/262144 | ColumnLz4 | 34.065 | 4.86 | 11.77 | 21.810–22.365 |
| Trade/Volatile/262144 | DeltaBitpackLz4 | 10.195 | 19.90 | 16.00 | 16.242–16.926 |
| Trade/Volatile/262144 | DeltaVarintLz4 | 12.274 | 20.69 | 26.57 | 9.746–10.231 |
| Trade/Volatile/262144 | ParquetDeltaBrotli | 9.960 | 3.41 | 10.89 | 23.492–25.494 |
| Trade/Volatile/262144 | ParquetDeltaLz4 | 10.087 | 8.60 | 37.48 | 6.621–12.551 |
| Trade/Volatile/262144 | ParquetDeltaSnappy | 10.087 | 8.34 | 37.19 | 6.628–11.293 |
| Trade/Volatile/262144 | ParquetDictionaryLz4 | 27.776 | 2.21 | 15.88 | 16.396–19.183 |
| Trade/Volatile/262144 | ParquetPlainLz4 | 34.087 | 4.16 | 10.47 | 24.814–25.952 |
| Trade/Volatile/4096 | AdaptiveSimd | 9.778 | 10.03 | 82.37 | 0.049–0.051 |
| Trade/Volatile/4096 | AdaptiveSimdLz4 | 9.764 | 9.66 | 82.12 | 0.049–0.050 |
| Trade/Volatile/4096 | ColumnLz4 | 34.115 | 5.25 | 12.62 | 0.324–0.326 |
| Trade/Volatile/4096 | DeltaBitpackLz4 | 10.230 | 19.52 | 16.30 | 0.251–0.260 |
| Trade/Volatile/4096 | DeltaVarintLz4 | 12.411 | 21.75 | 26.94 | 0.151–0.158 |
| Trade/Volatile/4096 | ParquetDeltaBrotli | 10.597 | 2.07 | 2.92 | 1.392–1.452 |
| Trade/Volatile/4096 | ParquetDeltaLz4 | 10.707 | 7.79 | 28.00 | 0.145–0.149 |
| Trade/Volatile/4096 | ParquetDeltaSnappy | 10.707 | 7.83 | 27.54 | 0.148–0.151 |
| Trade/Volatile/4096 | ParquetDictionaryLz4 | 43.447 | 2.37 | 9.08 | 0.447–0.454 |
| Trade/Volatile/4096 | ParquetPlainLz4 | 34.704 | 4.30 | 9.65 | 0.419–0.441 |
| Trade/Volatile/65536 | AdaptiveSimd | 9.756 | 14.96 | 80.58 | 0.793–0.863 |
| Trade/Volatile/65536 | AdaptiveSimdLz4 | 9.671 | 14.55 | 79.04 | 0.798–0.850 |
| Trade/Volatile/65536 | ColumnLz4 | 34.079 | 4.92 | 12.26 | 5.338–5.403 |
| Trade/Volatile/65536 | DeltaBitpackLz4 | 10.197 | 21.79 | 16.20 | 4.004–5.430 |
| Trade/Volatile/65536 | DeltaVarintLz4 | 12.281 | 22.57 | 26.87 | 2.419–2.451 |
| Trade/Volatile/65536 | ParquetDeltaBrotli | 9.977 | 3.41 | 11.12 | 5.731–5.975 |
| Trade/Volatile/65536 | ParquetDeltaLz4 | 10.110 | 9.19 | 43.61 | 1.443–1.531 |
| Trade/Volatile/65536 | ParquetDeltaSnappy | 10.110 | 9.22 | 43.82 | 1.466–2.654 |
| Trade/Volatile/65536 | ParquetDictionaryLz4 | 42.979 | 1.60 | 10.95 | 5.824–6.136 |
| Trade/Volatile/65536 | ParquetPlainLz4 | 34.117 | 4.36 | 10.95 | 5.859–8.720 |

## Parquet size/decode Pareto frontier

Frontier membership ignores encode speed; inspect encode results separately.

| Case | Nondominated Parquet variants |
| --- | --- |
| L2/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaSnappy |
