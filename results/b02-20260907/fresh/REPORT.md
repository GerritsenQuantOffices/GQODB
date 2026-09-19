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
| L2/Quiet/262144 | AdaptiveSimd | 2.060 | 12.93 | 81.12 | 0.843 | 1.46x | 2.11x | NO |
| L2/Quiet/262144 | AdaptiveSimdLz4 | 1.773 | 12.77 | 80.63 | 0.726 | 1.45x | 2.10x | NO |
| L2/Quiet/4096 | AdaptiveSimd | 1.841 | 7.58 | 90.37 | 0.584 | 0.97x | 3.29x | NO |
| L2/Quiet/4096 | AdaptiveSimdLz4 | 1.627 | 7.39 | 89.36 | 0.517 | 0.94x | 3.26x | NO |
| L2/Quiet/65536 | AdaptiveSimd | 1.885 | 12.90 | 92.72 | 0.764 | 1.37x | 2.07x | NO |
| L2/Quiet/65536 | AdaptiveSimdLz4 | 1.601 | 12.67 | 87.08 | 0.649 | 1.34x | 1.95x | YES |
| L2/Volatile/262144 | AdaptiveSimd | 10.330 | 13.70 | 76.13 | 1.000 | 1.60x | 2.59x | NO |
| L2/Volatile/262144 | AdaptiveSimdLz4 | 9.918 | 13.31 | 76.44 | 0.960 | 1.56x | 2.60x | YES |
| L2/Volatile/4096 | AdaptiveSimd | 10.153 | 10.14 | 87.92 | 0.924 | 1.31x | 3.20x | YES |
| L2/Volatile/4096 | AdaptiveSimdLz4 | 10.040 | 9.82 | 88.72 | 0.914 | 1.27x | 3.23x | YES |
| L2/Volatile/65536 | AdaptiveSimd | 10.131 | 15.15 | 86.99 | 0.978 | 1.65x | 2.03x | YES |
| L2/Volatile/65536 | AdaptiveSimdLz4 | 9.923 | 14.56 | 86.08 | 0.958 | 1.59x | 2.01x | YES |
| Quote/Quiet/262144 | AdaptiveSimd | 1.189 | 12.55 | 70.76 | 0.358 | 1.55x | 3.69x | YES |
| Quote/Quiet/262144 | AdaptiveSimdLz4 | 1.026 | 11.96 | 25.61 | 0.309 | 1.47x | 1.34x | NO |
| Quote/Quiet/4096 | AdaptiveSimd | 1.278 | 7.45 | 42.24 | 0.319 | 1.12x | 2.38x | YES |
| Quote/Quiet/4096 | AdaptiveSimdLz4 | 1.163 | 7.34 | 41.35 | 0.290 | 1.10x | 2.33x | YES |
| Quote/Quiet/65536 | AdaptiveSimd | 1.193 | 12.02 | 26.99 | 0.357 | 1.43x | 1.43x | YES |
| Quote/Quiet/65536 | AdaptiveSimdLz4 | 1.033 | 12.04 | 26.02 | 0.309 | 1.43x | 1.37x | YES |
| Quote/Volatile/262144 | AdaptiveSimd | 10.955 | 13.56 | 67.97 | 0.789 | 1.75x | 2.85x | YES |
| Quote/Volatile/262144 | AdaptiveSimdLz4 | 10.675 | 13.12 | 67.72 | 0.769 | 1.69x | 2.84x | YES |
| Quote/Volatile/4096 | AdaptiveSimd | 10.778 | 10.02 | 82.46 | 0.744 | 1.32x | 2.97x | YES |
| Quote/Volatile/4096 | AdaptiveSimdLz4 | 10.769 | 9.65 | 81.55 | 0.743 | 1.27x | 2.94x | YES |
| Quote/Volatile/65536 | AdaptiveSimd | 10.756 | 14.91 | 77.16 | 0.774 | 1.68x | 1.80x | YES |
| Quote/Volatile/65536 | AdaptiveSimdLz4 | 10.678 | 14.60 | 77.70 | 0.768 | 1.64x | 1.81x | YES |
| Trade/Quiet/262144 | AdaptiveSimd | 0.939 | 12.69 | 72.82 | 0.490 | 1.45x | 2.13x | YES |
| Trade/Quiet/262144 | AdaptiveSimdLz4 | 0.776 | 12.65 | 72.79 | 0.405 | 1.45x | 2.12x | YES |
| Trade/Quiet/4096 | AdaptiveSimd | 1.028 | 7.36 | 81.88 | 0.396 | 0.94x | 2.94x | NO |
| Trade/Quiet/4096 | AdaptiveSimdLz4 | 0.913 | 7.21 | 81.41 | 0.352 | 0.92x | 2.92x | NO |
| Trade/Quiet/65536 | AdaptiveSimd | 0.943 | 12.57 | 78.72 | 0.487 | 1.34x | 1.81x | YES |
| Trade/Quiet/65536 | AdaptiveSimdLz4 | 0.783 | 12.58 | 82.97 | 0.404 | 1.35x | 1.90x | YES |
| Trade/Volatile/262144 | AdaptiveSimd | 9.955 | 13.71 | 72.51 | 0.987 | 1.60x | 3.03x | YES |
| Trade/Volatile/262144 | AdaptiveSimdLz4 | 9.667 | 13.24 | 70.89 | 0.958 | 1.54x | 2.96x | YES |
| Trade/Volatile/4096 | AdaptiveSimd | 9.778 | 10.01 | 83.33 | 0.913 | 1.28x | 2.94x | YES |
| Trade/Volatile/4096 | AdaptiveSimdLz4 | 9.764 | 9.72 | 83.32 | 0.912 | 1.24x | 2.94x | YES |
| Trade/Volatile/65536 | AdaptiveSimd | 9.756 | 14.85 | 78.07 | 0.965 | 1.61x | 1.74x | YES |
| Trade/Volatile/65536 | AdaptiveSimdLz4 | 9.671 | 14.48 | 77.79 | 0.957 | 1.57x | 1.73x | YES |

Matched delta-LZ4 ambitious gate: 11/36.
All-Parquet ambitious gate: 7/36.
Matched delta-LZ4 three-axis wins: 32/36.
All-Parquet three-axis wins: 27/36.
Matched delta-LZ4 practical gate: 17/36.
All-Parquet practical gate: 13/36.

## All variants

Rates are medians across process repetitions. Min/max and individual timings are
in results.json and trial files. Rates include format conversion and allocations.

| Case | Codec | Bytes/row | Encode Mrow/s | Decode Mrow/s | Decode min–max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| L2/Quiet/262144 | AdaptiveSimd | 2.060 | 12.93 | 81.12 | 3.171–3.517 |
| L2/Quiet/262144 | AdaptiveSimdLz4 | 1.773 | 12.77 | 80.63 | 3.219–3.358 |
| L2/Quiet/262144 | ColumnLz4 | 26.830 | 6.10 | 13.25 | 19.574–19.893 |
| L2/Quiet/262144 | DeltaBitpackLz4 | 2.415 | 22.35 | 17.86 | 14.628–14.762 |
| L2/Quiet/262144 | DeltaVarintLz4 | 2.569 | 24.65 | 31.28 | 8.337–8.492 |
| L2/Quiet/262144 | ParquetDeltaBrotli | 1.765 | 3.50 | 11.31 | 22.866–23.621 |
| L2/Quiet/262144 | ParquetDeltaLz4 | 2.443 | 8.83 | 38.48 | 6.707–9.054 |
| L2/Quiet/262144 | ParquetDeltaSnappy | 2.529 | 8.81 | 37.15 | 6.947–7.588 |
| L2/Quiet/262144 | ParquetDictionaryLz4 | 16.101 | 2.97 | 15.71 | 16.029–17.311 |
| L2/Quiet/262144 | ParquetPlainLz4 | 26.847 | 4.75 | 11.12 | 22.629–25.547 |
| L2/Quiet/4096 | AdaptiveSimd | 1.841 | 7.58 | 90.37 | 0.045–0.048 |
| L2/Quiet/4096 | AdaptiveSimdLz4 | 1.627 | 7.39 | 89.36 | 0.045–0.050 |
| L2/Quiet/4096 | ColumnLz4 | 26.887 | 6.42 | 14.24 | 0.287–0.291 |
| L2/Quiet/4096 | DeltaBitpackLz4 | 2.562 | 23.30 | 17.89 | 0.227–0.232 |
| L2/Quiet/4096 | DeltaVarintLz4 | 3.051 | 26.89 | 30.63 | 0.133–0.135 |
| L2/Quiet/4096 | ParquetDeltaBrotli | 2.730 | 2.40 | 2.08 | 1.967–2.002 |
| L2/Quiet/4096 | ParquetDeltaLz4 | 3.149 | 7.83 | 27.43 | 0.149–0.150 |
| L2/Quiet/4096 | ParquetDeltaSnappy | 3.264 | 8.08 | 27.55 | 0.147–0.151 |
| L2/Quiet/4096 | ParquetDictionaryLz4 | 27.949 | 2.96 | 11.45 | 0.354–0.364 |
| L2/Quiet/4096 | ParquetPlainLz4 | 27.472 | 5.15 | 10.70 | 0.380–0.388 |
| L2/Quiet/65536 | AdaptiveSimd | 1.885 | 12.90 | 92.72 | 0.695–0.803 |
| L2/Quiet/65536 | AdaptiveSimdLz4 | 1.601 | 12.67 | 87.08 | 0.707–0.780 |
| L2/Quiet/65536 | ColumnLz4 | 26.833 | 6.16 | 13.68 | 4.778–5.023 |
| L2/Quiet/65536 | DeltaBitpackLz4 | 2.426 | 24.37 | 17.77 | 3.580–3.712 |
| L2/Quiet/65536 | DeltaVarintLz4 | 2.589 | 26.63 | 31.45 | 2.073–2.122 |
| L2/Quiet/65536 | ParquetDeltaBrotli | 1.785 | 3.56 | 11.49 | 5.583–5.929 |
| L2/Quiet/65536 | ParquetDeltaLz4 | 2.467 | 9.45 | 44.76 | 1.431–1.578 |
| L2/Quiet/65536 | ParquetDeltaSnappy | 2.609 | 9.58 | 43.74 | 1.486–1.648 |
| L2/Quiet/65536 | ParquetDictionaryLz4 | 29.387 | 2.17 | 13.82 | 4.736–4.892 |
| L2/Quiet/65536 | ParquetPlainLz4 | 26.870 | 5.37 | 12.18 | 5.337–5.570 |
| L2/Volatile/262144 | AdaptiveSimd | 10.330 | 13.70 | 76.13 | 3.364–3.598 |
| L2/Volatile/262144 | AdaptiveSimdLz4 | 9.918 | 13.31 | 76.44 | 3.376–3.583 |
| L2/Volatile/262144 | ColumnLz4 | 29.917 | 5.34 | 13.29 | 19.554–19.858 |
| L2/Volatile/262144 | DeltaBitpackLz4 | 10.445 | 19.71 | 15.96 | 16.234–16.658 |
| L2/Volatile/262144 | DeltaVarintLz4 | 12.289 | 20.98 | 26.11 | 9.864–10.149 |
| L2/Volatile/262144 | ParquetDeltaBrotli | 10.205 | 3.41 | 10.49 | 23.707–26.232 |
| L2/Volatile/262144 | ParquetDeltaLz4 | 10.334 | 8.55 | 29.42 | 7.004–13.046 |
| L2/Volatile/262144 | ParquetDeltaSnappy | 10.343 | 8.54 | 32.81 | 6.925–12.608 |
| L2/Volatile/262144 | ParquetDictionaryLz4 | 24.722 | 2.48 | 14.10 | 15.527–20.619 |
| L2/Volatile/262144 | ParquetPlainLz4 | 29.939 | 4.41 | 11.50 | 22.460–23.614 |
| L2/Volatile/4096 | AdaptiveSimd | 10.153 | 10.14 | 87.92 | 0.045–0.048 |
| L2/Volatile/4096 | AdaptiveSimdLz4 | 10.040 | 9.82 | 88.72 | 0.046–0.049 |
| L2/Volatile/4096 | ColumnLz4 | 29.976 | 5.78 | 14.09 | 0.289–0.294 |
| L2/Volatile/4096 | DeltaBitpackLz4 | 10.500 | 19.24 | 16.11 | 0.254–0.255 |
| L2/Volatile/4096 | DeltaVarintLz4 | 12.427 | 22.05 | 26.37 | 0.153–0.157 |
| L2/Volatile/4096 | ParquetDeltaBrotli | 10.851 | 2.02 | 2.91 | 1.401–1.488 |
| L2/Volatile/4096 | ParquetDeltaLz4 | 10.986 | 7.75 | 27.48 | 0.148–0.154 |
| L2/Volatile/4096 | ParquetDeltaSnappy | 10.976 | 7.71 | 26.91 | 0.151–0.154 |
| L2/Volatile/4096 | ParquetDictionaryLz4 | 37.779 | 2.55 | 10.07 | 0.402–0.427 |
| L2/Volatile/4096 | ParquetPlainLz4 | 30.561 | 4.68 | 10.68 | 0.383–0.387 |
| L2/Volatile/65536 | AdaptiveSimd | 10.131 | 15.15 | 86.99 | 0.744–0.818 |
| L2/Volatile/65536 | AdaptiveSimdLz4 | 9.923 | 14.56 | 86.08 | 0.751–0.789 |
| L2/Volatile/65536 | ColumnLz4 | 29.924 | 5.44 | 13.67 | 4.770–4.986 |
| L2/Volatile/65536 | DeltaBitpackLz4 | 10.447 | 21.11 | 16.03 | 4.027–4.134 |
| L2/Volatile/65536 | DeltaVarintLz4 | 12.295 | 22.66 | 26.51 | 2.467–2.557 |
| L2/Volatile/65536 | ParquetDeltaBrotli | 10.224 | 3.40 | 11.02 | 5.880–6.072 |
| L2/Volatile/65536 | ParquetDeltaLz4 | 10.357 | 9.18 | 42.76 | 1.507–1.625 |
| L2/Volatile/65536 | ParquetDeltaSnappy | 10.366 | 9.18 | 41.61 | 1.497–1.712 |
| L2/Volatile/65536 | ParquetDictionaryLz4 | 36.718 | 1.82 | 12.57 | 5.186–5.483 |
| L2/Volatile/65536 | ParquetPlainLz4 | 29.962 | 4.77 | 12.11 | 5.361–5.647 |
| Quote/Quiet/262144 | AdaptiveSimd | 1.189 | 12.55 | 70.76 | 3.677–10.420 |
| Quote/Quiet/262144 | AdaptiveSimdLz4 | 1.026 | 11.96 | 25.61 | 3.700–10.410 |
| Quote/Quiet/262144 | ColumnLz4 | 29.338 | 5.66 | 13.25 | 19.588–24.993 |
| Quote/Quiet/262144 | DeltaBitpackLz4 | 3.278 | 21.29 | 17.26 | 14.940–21.985 |
| Quote/Quiet/262144 | DeltaVarintLz4 | 3.992 | 21.04 | 29.13 | 8.838–12.903 |
| Quote/Quiet/262144 | ParquetDeltaBrotli | 2.599 | 3.13 | 9.96 | 25.526–36.334 |
| Quote/Quiet/262144 | ParquetDeltaLz4 | 3.322 | 8.12 | 19.17 | 13.063–14.276 |
| Quote/Quiet/262144 | ParquetDeltaSnappy | 3.397 | 8.47 | 31.67 | 7.945–13.857 |
| Quote/Quiet/262144 | ParquetDictionaryLz4 | 17.193 | 2.73 | 14.70 | 17.484–18.242 |
| Quote/Quiet/262144 | ParquetPlainLz4 | 29.358 | 3.68 | 8.89 | 26.809–29.653 |
| Quote/Quiet/4096 | AdaptiveSimd | 1.278 | 7.45 | 42.24 | 0.051–0.114 |
| Quote/Quiet/4096 | AdaptiveSimdLz4 | 1.163 | 7.34 | 41.35 | 0.053–0.118 |
| Quote/Quiet/4096 | ColumnLz4 | 29.473 | 6.02 | 14.27 | 0.281–0.321 |
| Quote/Quiet/4096 | DeltaBitpackLz4 | 3.376 | 21.68 | 17.09 | 0.233–0.298 |
| Quote/Quiet/4096 | DeltaVarintLz4 | 4.608 | 21.83 | 28.27 | 0.144–0.204 |
| Quote/Quiet/4096 | ParquetDeltaBrotli | 3.565 | 1.78 | 1.23 | 2.012–3.804 |
| Quote/Quiet/4096 | ParquetDeltaLz4 | 4.008 | 6.68 | 17.76 | 0.227–0.236 |
| Quote/Quiet/4096 | ParquetDeltaSnappy | 4.143 | 6.61 | 18.18 | 0.154–0.232 |
| Quote/Quiet/4096 | ParquetDictionaryLz4 | 29.395 | 2.12 | 8.10 | 0.390–0.519 |
| Quote/Quiet/4096 | ParquetPlainLz4 | 30.065 | 4.01 | 8.01 | 0.477–0.531 |
| Quote/Quiet/65536 | AdaptiveSimd | 1.193 | 12.02 | 26.99 | 0.813–2.623 |
| Quote/Quiet/65536 | AdaptiveSimdLz4 | 1.033 | 12.04 | 26.02 | 0.822–2.657 |
| Quote/Quiet/65536 | ColumnLz4 | 29.330 | 4.67 | 12.95 | 4.757–5.253 |
| Quote/Quiet/65536 | DeltaBitpackLz4 | 3.286 | 23.19 | 17.60 | 3.698–5.496 |
| Quote/Quiet/65536 | DeltaVarintLz4 | 4.011 | 22.64 | 28.53 | 2.207–3.070 |
| Quote/Quiet/65536 | ParquetDeltaBrotli | 2.624 | 2.51 | 6.81 | 9.573–9.701 |
| Quote/Quiet/65536 | ParquetDeltaLz4 | 3.346 | 8.42 | 18.92 | 3.373–3.520 |
| Quote/Quiet/65536 | ParquetDeltaSnappy | 3.380 | 8.20 | 18.59 | 3.469–3.831 |
| Quote/Quiet/65536 | ParquetDictionaryLz4 | 30.095 | 1.72 | 11.84 | 5.156–6.145 |
| Quote/Quiet/65536 | ParquetPlainLz4 | 29.368 | 3.78 | 7.49 | 8.516–9.993 |
| Quote/Volatile/262144 | AdaptiveSimd | 10.955 | 13.56 | 67.97 | 3.739–3.956 |
| Quote/Volatile/262144 | AdaptiveSimdLz4 | 10.675 | 13.12 | 67.72 | 3.750–3.904 |
| Quote/Volatile/262144 | ColumnLz4 | 37.169 | 4.41 | 13.35 | 19.328–19.675 |
| Quote/Volatile/262144 | DeltaBitpackLz4 | 13.986 | 17.71 | 14.17 | 18.319–18.729 |
| Quote/Volatile/262144 | DeltaVarintLz4 | 16.711 | 18.63 | 25.36 | 10.273–10.576 |
| Quote/Volatile/262144 | ParquetDeltaBrotli | 13.847 | 3.08 | 12.52 | 20.616–21.428 |
| Quote/Volatile/262144 | ParquetDeltaLz4 | 13.878 | 7.75 | 23.86 | 7.034–12.194 |
| Quote/Volatile/262144 | ParquetDeltaSnappy | 13.880 | 8.35 | 26.15 | 6.996–12.094 |
| Quote/Volatile/262144 | ParquetDictionaryLz4 | 33.039 | 1.99 | 13.00 | 17.443–23.785 |
| Quote/Volatile/262144 | ParquetPlainLz4 | 37.197 | 3.51 | 11.53 | 22.583–23.603 |
| Quote/Volatile/4096 | AdaptiveSimd | 10.778 | 10.02 | 82.46 | 0.049–0.052 |
| Quote/Volatile/4096 | AdaptiveSimdLz4 | 10.769 | 9.65 | 81.55 | 0.050–0.052 |
| Quote/Volatile/4096 | ColumnLz4 | 37.288 | 4.89 | 14.41 | 0.284–0.286 |
| Quote/Volatile/4096 | DeltaBitpackLz4 | 14.031 | 17.48 | 14.39 | 0.284–0.291 |
| Quote/Volatile/4096 | DeltaVarintLz4 | 16.721 | 19.95 | 25.79 | 0.157–0.160 |
| Quote/Volatile/4096 | ParquetDeltaBrotli | 14.473 | 1.78 | 4.25 | 0.962–0.968 |
| Quote/Volatile/4096 | ParquetDeltaLz4 | 14.494 | 7.60 | 27.73 | 0.147–0.148 |
| Quote/Volatile/4096 | ParquetDeltaSnappy | 14.492 | 7.65 | 27.32 | 0.149–0.155 |
| Quote/Volatile/4096 | ParquetDictionaryLz4 | 49.565 | 2.19 | 8.84 | 0.462–0.483 |
| Quote/Volatile/4096 | ParquetPlainLz4 | 37.880 | 4.10 | 10.77 | 0.379–0.389 |
| Quote/Volatile/65536 | AdaptiveSimd | 10.756 | 14.91 | 77.16 | 0.833–0.904 |
| Quote/Volatile/65536 | AdaptiveSimdLz4 | 10.678 | 14.60 | 77.70 | 0.837–0.930 |
| Quote/Volatile/65536 | ColumnLz4 | 37.167 | 4.51 | 13.92 | 4.676–4.822 |
| Quote/Volatile/65536 | DeltaBitpackLz4 | 13.991 | 19.39 | 14.18 | 4.573–4.647 |
| Quote/Volatile/65536 | DeltaVarintLz4 | 16.711 | 20.03 | 25.46 | 2.560–2.583 |
| Quote/Volatile/65536 | ParquetDeltaBrotli | 13.874 | 3.15 | 12.47 | 5.163–5.289 |
| Quote/Volatile/65536 | ParquetDeltaLz4 | 13.903 | 8.89 | 42.90 | 1.510–1.561 |
| Quote/Volatile/65536 | ParquetDeltaSnappy | 13.904 | 8.86 | 41.40 | 1.517–1.746 |
| Quote/Volatile/65536 | ParquetDictionaryLz4 | 48.533 | 1.49 | 10.64 | 6.108–6.288 |
| Quote/Volatile/65536 | ParquetPlainLz4 | 37.205 | 4.13 | 12.15 | 5.332–5.486 |
| Trade/Quiet/262144 | AdaptiveSimd | 0.939 | 12.69 | 72.82 | 3.539–3.681 |
| Trade/Quiet/262144 | AdaptiveSimdLz4 | 0.776 | 12.65 | 72.79 | 3.545–3.880 |
| Trade/Quiet/262144 | ColumnLz4 | 29.924 | 5.45 | 11.77 | 22.142–25.516 |
| Trade/Quiet/262144 | DeltaBitpackLz4 | 1.891 | 21.95 | 18.38 | 14.236–14.595 |
| Trade/Quiet/262144 | DeltaVarintLz4 | 2.074 | 23.85 | 31.02 | 8.350–8.581 |
| Trade/Quiet/262144 | ParquetDeltaBrotli | 1.192 | 3.64 | 12.81 | 20.270–22.362 |
| Trade/Quiet/262144 | ParquetDeltaLz4 | 1.918 | 8.75 | 34.26 | 6.599–10.522 |
| Trade/Quiet/262144 | ParquetDeltaSnappy | 1.978 | 8.77 | 36.69 | 6.995–13.712 |
| Trade/Quiet/262144 | ParquetDictionaryLz4 | 18.727 | 2.57 | 15.47 | 15.244–17.512 |
| Trade/Quiet/262144 | ParquetPlainLz4 | 29.941 | 4.15 | 10.11 | 24.965–30.465 |
| Trade/Quiet/4096 | AdaptiveSimd | 1.028 | 7.36 | 81.88 | 0.050–0.053 |
| Trade/Quiet/4096 | AdaptiveSimdLz4 | 0.913 | 7.21 | 81.41 | 0.050–0.053 |
| Trade/Quiet/4096 | ColumnLz4 | 30.065 | 5.88 | 12.51 | 0.326–0.334 |
| Trade/Quiet/4096 | DeltaBitpackLz4 | 1.994 | 21.79 | 17.92 | 0.224–0.232 |
| Trade/Quiet/4096 | DeltaVarintLz4 | 2.695 | 24.76 | 29.74 | 0.135–0.143 |
| Trade/Quiet/4096 | ParquetDeltaBrotli | 2.141 | 2.52 | 2.11 | 1.930–1.961 |
| Trade/Quiet/4096 | ParquetDeltaLz4 | 2.594 | 7.84 | 27.84 | 0.144–0.149 |
| Trade/Quiet/4096 | ParquetDeltaSnappy | 2.742 | 8.07 | 28.06 | 0.144–0.146 |
| Trade/Quiet/4096 | ParquetDictionaryLz4 | 33.281 | 2.71 | 10.33 | 0.392–0.411 |
| Trade/Quiet/4096 | ParquetPlainLz4 | 30.654 | 4.76 | 9.79 | 0.417–0.425 |
| Trade/Quiet/65536 | AdaptiveSimd | 0.943 | 12.57 | 78.72 | 0.777–0.895 |
| Trade/Quiet/65536 | AdaptiveSimdLz4 | 0.783 | 12.58 | 82.97 | 0.779–0.888 |
| Trade/Quiet/65536 | ColumnLz4 | 29.935 | 5.56 | 12.00 | 5.363–5.539 |
| Trade/Quiet/65536 | DeltaBitpackLz4 | 1.901 | 23.39 | 18.45 | 3.516–3.665 |
| Trade/Quiet/65536 | DeltaVarintLz4 | 2.091 | 25.87 | 31.43 | 2.079–2.152 |
| Trade/Quiet/65536 | ParquetDeltaBrotli | 1.216 | 3.71 | 13.02 | 4.947–5.122 |
| Trade/Quiet/65536 | ParquetDeltaLz4 | 1.936 | 9.35 | 43.57 | 1.416–1.532 |
| Trade/Quiet/65536 | ParquetDeltaSnappy | 1.970 | 9.23 | 42.10 | 1.494–1.577 |
| Trade/Quiet/65536 | ParquetDictionaryLz4 | 35.093 | 1.85 | 12.13 | 5.364–5.432 |
| Trade/Quiet/65536 | ParquetPlainLz4 | 29.972 | 4.89 | 10.90 | 5.992–6.215 |
| Trade/Volatile/262144 | AdaptiveSimd | 9.955 | 13.71 | 72.51 | 3.580–3.750 |
| Trade/Volatile/262144 | AdaptiveSimdLz4 | 9.667 | 13.24 | 70.89 | 3.582–3.774 |
| Trade/Volatile/262144 | ColumnLz4 | 34.066 | 4.86 | 11.84 | 22.056–22.183 |
| Trade/Volatile/262144 | DeltaBitpackLz4 | 10.195 | 19.91 | 16.16 | 16.092–16.324 |
| Trade/Volatile/262144 | DeltaVarintLz4 | 12.269 | 20.82 | 26.99 | 9.680–9.853 |
| Trade/Volatile/262144 | ParquetDeltaBrotli | 9.957 | 3.42 | 10.33 | 23.853–26.116 |
| Trade/Volatile/262144 | ParquetDeltaLz4 | 10.087 | 8.57 | 23.97 | 7.008–12.504 |
| Trade/Volatile/262144 | ParquetDeltaSnappy | 10.087 | 8.63 | 38.62 | 6.727–7.974 |
| Trade/Volatile/262144 | ParquetDictionaryLz4 | 27.686 | 2.17 | 15.86 | 16.414–16.862 |
| Trade/Volatile/262144 | ParquetPlainLz4 | 34.088 | 4.08 | 10.13 | 25.067–28.890 |
| Trade/Volatile/4096 | AdaptiveSimd | 9.778 | 10.01 | 83.33 | 0.048–0.052 |
| Trade/Volatile/4096 | AdaptiveSimdLz4 | 9.764 | 9.72 | 83.32 | 0.049–0.052 |
| Trade/Volatile/4096 | ColumnLz4 | 34.148 | 5.28 | 12.60 | 0.323–0.329 |
| Trade/Volatile/4096 | DeltaBitpackLz4 | 10.229 | 19.56 | 16.27 | 0.251–0.257 |
| Trade/Volatile/4096 | DeltaVarintLz4 | 12.442 | 21.92 | 27.01 | 0.151–0.153 |
| Trade/Volatile/4096 | ParquetDeltaBrotli | 10.594 | 2.10 | 2.94 | 1.386–1.416 |
| Trade/Volatile/4096 | ParquetDeltaLz4 | 10.707 | 7.85 | 28.31 | 0.144–0.146 |
| Trade/Volatile/4096 | ParquetDeltaSnappy | 10.708 | 7.81 | 27.85 | 0.146–0.149 |
| Trade/Volatile/4096 | ParquetDictionaryLz4 | 43.559 | 2.40 | 9.11 | 0.445–0.451 |
| Trade/Volatile/4096 | ParquetPlainLz4 | 34.737 | 4.33 | 9.80 | 0.416–0.429 |
| Trade/Volatile/65536 | AdaptiveSimd | 9.756 | 14.85 | 78.07 | 0.800–0.868 |
| Trade/Volatile/65536 | AdaptiveSimdLz4 | 9.671 | 14.48 | 77.79 | 0.808–0.873 |
| Trade/Volatile/65536 | ColumnLz4 | 34.061 | 4.92 | 12.31 | 5.313–5.354 |
| Trade/Volatile/65536 | DeltaBitpackLz4 | 10.196 | 21.80 | 16.25 | 4.017–4.095 |
| Trade/Volatile/65536 | DeltaVarintLz4 | 12.276 | 22.56 | 27.01 | 2.416–2.481 |
| Trade/Volatile/65536 | ParquetDeltaBrotli | 9.976 | 3.36 | 11.19 | 5.828–5.920 |
| Trade/Volatile/65536 | ParquetDeltaLz4 | 10.110 | 9.21 | 44.90 | 1.451–1.523 |
| Trade/Volatile/65536 | ParquetDeltaSnappy | 10.110 | 9.24 | 44.21 | 1.450–1.532 |
| Trade/Volatile/65536 | ParquetDictionaryLz4 | 42.567 | 1.62 | 11.12 | 5.815–5.946 |
| Trade/Volatile/65536 | ParquetPlainLz4 | 34.098 | 4.42 | 10.97 | 5.936–6.125 |

## Parquet size/decode Pareto frontier

Frontier membership ignores encode speed; inspect encode results separately.

| Case | Nondominated Parquet variants |
| --- | --- |
| L2/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaSnappy |
| Trade/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
