# B01 block feasibility results

Synthetic warm in-memory blocks; owned integer columns in and out.
Five process repetitions are intended; actual counts are in results.json.
Complete encoded blocks and checksums counted; full database metadata/WAL absent.
No Zstd, live bus, real exchange data, filesystem, partial queries or book reconstruction tested.
No market-wide or production-readiness verdict follows from these results.

## Candidate screen

The threshold is >=20% fewer bytes AND >=2x faster full decode.
All-Parquet means meeting both thresholds against each tested Parquet variant.
This is stricter than beating one chosen weak variant, but still a limited baseline set.

| Case | Candidate | Bytes/row | Decode Mrow/s | Size / delta-LZ4 | Decode speedup / delta-LZ4 | All-Parquet gate |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| L2/Quiet/262144 | DeltaBitpackLz4 | 2.415 | 18.09 | 0.988 | 0.49x | FAIL |
| L2/Quiet/262144 | DeltaVarintLz4 | 2.568 | 31.34 | 1.051 | 0.86x | FAIL |
| L2/Quiet/4096 | DeltaBitpackLz4 | 2.560 | 18.05 | 0.808 | 0.66x | FAIL |
| L2/Quiet/4096 | DeltaVarintLz4 | 3.061 | 30.69 | 0.966 | 1.12x | FAIL |
| L2/Quiet/65536 | DeltaBitpackLz4 | 2.427 | 18.20 | 0.983 | 0.40x | FAIL |
| L2/Quiet/65536 | DeltaVarintLz4 | 2.588 | 31.57 | 1.048 | 0.70x | FAIL |
| L2/Volatile/262144 | DeltaBitpackLz4 | 10.445 | 16.16 | 1.011 | 0.46x | FAIL |
| L2/Volatile/262144 | DeltaVarintLz4 | 12.289 | 26.23 | 1.189 | 0.75x | FAIL |
| L2/Volatile/4096 | DeltaBitpackLz4 | 10.500 | 16.11 | 0.956 | 0.59x | FAIL |
| L2/Volatile/4096 | DeltaVarintLz4 | 12.423 | 26.61 | 1.131 | 0.98x | FAIL |
| L2/Volatile/65536 | DeltaBitpackLz4 | 10.448 | 16.05 | 1.009 | 0.36x | FAIL |
| L2/Volatile/65536 | DeltaVarintLz4 | 12.284 | 26.53 | 1.186 | 0.60x | FAIL |
| Quote/Quiet/262144 | DeltaBitpackLz4 | 3.278 | 12.09 | 0.986 | 0.60x | FAIL |
| Quote/Quiet/262144 | DeltaVarintLz4 | 3.992 | 19.08 | 1.201 | 0.95x | FAIL |
| Quote/Quiet/4096 | DeltaBitpackLz4 | 3.389 | 17.57 | 0.848 | 0.99x | FAIL |
| Quote/Quiet/4096 | DeltaVarintLz4 | 4.602 | 28.64 | 1.152 | 1.61x | FAIL |
| Quote/Quiet/65536 | DeltaBitpackLz4 | 3.287 | 17.55 | 0.983 | 0.92x | FAIL |
| Quote/Quiet/65536 | DeltaVarintLz4 | 4.010 | 29.67 | 1.200 | 1.56x | FAIL |
| Quote/Volatile/262144 | DeltaBitpackLz4 | 13.985 | 14.31 | 1.008 | 0.65x | FAIL |
| Quote/Volatile/262144 | DeltaVarintLz4 | 16.704 | 25.41 | 1.204 | 1.16x | FAIL |
| Quote/Volatile/4096 | DeltaBitpackLz4 | 14.010 | 14.37 | 0.967 | 0.52x | FAIL |
| Quote/Volatile/4096 | DeltaVarintLz4 | 16.761 | 26.02 | 1.156 | 0.94x | FAIL |
| Quote/Volatile/65536 | DeltaBitpackLz4 | 13.985 | 14.31 | 1.006 | 0.33x | FAIL |
| Quote/Volatile/65536 | DeltaVarintLz4 | 16.704 | 25.48 | 1.202 | 0.59x | FAIL |
| Trade/Quiet/262144 | DeltaBitpackLz4 | 1.892 | 18.38 | 0.986 | 0.46x | FAIL |
| Trade/Quiet/262144 | DeltaVarintLz4 | 2.075 | 31.06 | 1.081 | 0.78x | FAIL |
| Trade/Quiet/4096 | DeltaBitpackLz4 | 1.998 | 18.36 | 0.768 | 0.65x | FAIL |
| Trade/Quiet/4096 | DeltaVarintLz4 | 2.687 | 30.34 | 1.034 | 1.08x | FAIL |
| Trade/Quiet/65536 | DeltaBitpackLz4 | 1.904 | 18.59 | 0.981 | 0.40x | FAIL |
| Trade/Quiet/65536 | DeltaVarintLz4 | 2.091 | 31.44 | 1.078 | 0.67x | FAIL |
| Trade/Volatile/262144 | DeltaBitpackLz4 | 10.195 | 16.30 | 1.011 | 0.42x | FAIL |
| Trade/Volatile/262144 | DeltaVarintLz4 | 12.274 | 27.01 | 1.217 | 0.69x | FAIL |
| Trade/Volatile/4096 | DeltaBitpackLz4 | 10.230 | 16.34 | 0.955 | 0.58x | FAIL |
| Trade/Volatile/4096 | DeltaVarintLz4 | 12.411 | 27.21 | 1.159 | 0.96x | FAIL |
| Trade/Volatile/65536 | DeltaBitpackLz4 | 10.197 | 16.27 | 1.009 | 0.36x | FAIL |
| Trade/Volatile/65536 | DeltaVarintLz4 | 12.281 | 27.06 | 1.215 | 0.60x | FAIL |

Matched delta-LZ4 gate: 0/36 candidate/case pairs.
All-Parquet gate: 0/36 candidate/case pairs.

## All variants

Rates are medians across process repetitions. Min/max and individual timings are
in results.json and trial files. Rates include format conversion and allocations.

| Case | Codec | Bytes/row | Encode Mrow/s | Decode Mrow/s | Decode min–max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| L2/Quiet/262144 | ColumnLz4 | 26.826 | 6.10 | 13.34 | 19.542–19.851 |
| L2/Quiet/262144 | DeltaBitpackLz4 | 2.415 | 22.54 | 18.09 | 14.348–14.787 |
| L2/Quiet/262144 | DeltaVarintLz4 | 2.568 | 24.95 | 31.34 | 8.311–8.500 |
| L2/Quiet/262144 | ParquetDeltaBrotli | 1.765 | 3.52 | 11.33 | 22.579–23.854 |
| L2/Quiet/262144 | ParquetDeltaLz4 | 2.444 | 8.74 | 36.56 | 6.823–9.598 |
| L2/Quiet/262144 | ParquetDeltaSnappy | 2.524 | 8.78 | 37.98 | 6.803–9.712 |
| L2/Quiet/262144 | ParquetDictionaryLz4 | 16.035 | 3.05 | 16.31 | 13.672–16.415 |
| L2/Quiet/262144 | ParquetPlainLz4 | 26.844 | 4.71 | 11.51 | 22.519–24.947 |
| L2/Quiet/4096 | ColumnLz4 | 27.006 | 6.51 | 14.13 | 0.289–0.297 |
| L2/Quiet/4096 | DeltaBitpackLz4 | 2.560 | 23.33 | 18.05 | 0.226–0.233 |
| L2/Quiet/4096 | DeltaVarintLz4 | 3.061 | 27.22 | 30.69 | 0.133–0.138 |
| L2/Quiet/4096 | ParquetDeltaBrotli | 2.753 | 2.45 | 2.08 | 1.957–2.014 |
| L2/Quiet/4096 | ParquetDeltaLz4 | 3.168 | 7.85 | 27.33 | 0.149–0.155 |
| L2/Quiet/4096 | ParquetDeltaSnappy | 3.266 | 8.05 | 27.53 | 0.147–0.156 |
| L2/Quiet/4096 | ParquetDictionaryLz4 | 27.932 | 2.99 | 11.51 | 0.353–0.359 |
| L2/Quiet/4096 | ParquetPlainLz4 | 27.591 | 5.14 | 10.73 | 0.381–0.387 |
| L2/Quiet/65536 | ColumnLz4 | 26.834 | 6.17 | 13.48 | 4.766–4.931 |
| L2/Quiet/65536 | DeltaBitpackLz4 | 2.427 | 24.57 | 18.20 | 3.566–3.700 |
| L2/Quiet/65536 | DeltaVarintLz4 | 2.588 | 27.18 | 31.57 | 2.071–2.116 |
| L2/Quiet/65536 | ParquetDeltaBrotli | 1.791 | 3.57 | 11.46 | 5.534–5.807 |
| L2/Quiet/65536 | ParquetDeltaLz4 | 2.468 | 9.47 | 45.26 | 1.385–1.496 |
| L2/Quiet/65536 | ParquetDeltaSnappy | 2.538 | 9.43 | 41.84 | 1.495–1.591 |
| L2/Quiet/65536 | ParquetDictionaryLz4 | 29.387 | 2.18 | 13.94 | 4.646–4.848 |
| L2/Quiet/65536 | ParquetPlainLz4 | 26.871 | 5.44 | 12.17 | 5.304–5.475 |
| L2/Volatile/262144 | ColumnLz4 | 29.905 | 5.37 | 13.28 | 19.616–20.065 |
| L2/Volatile/262144 | DeltaBitpackLz4 | 10.445 | 19.90 | 16.16 | 16.127–16.518 |
| L2/Volatile/262144 | DeltaVarintLz4 | 12.289 | 20.88 | 26.23 | 9.904–10.043 |
| L2/Volatile/262144 | ParquetDeltaBrotli | 10.205 | 3.44 | 10.48 | 23.594–25.646 |
| L2/Volatile/262144 | ParquetDeltaLz4 | 10.334 | 8.66 | 34.79 | 6.735–10.922 |
| L2/Volatile/262144 | ParquetDeltaSnappy | 10.343 | 8.52 | 32.66 | 6.777–12.238 |
| L2/Volatile/262144 | ParquetDictionaryLz4 | 24.672 | 2.50 | 15.46 | 15.296–19.789 |
| L2/Volatile/262144 | ParquetPlainLz4 | 29.927 | 4.59 | 11.14 | 22.301–25.746 |
| L2/Volatile/4096 | ColumnLz4 | 29.988 | 5.82 | 13.94 | 0.290–0.295 |
| L2/Volatile/4096 | DeltaBitpackLz4 | 10.500 | 19.41 | 16.11 | 0.254–0.260 |
| L2/Volatile/4096 | DeltaVarintLz4 | 12.423 | 21.98 | 26.61 | 0.154–0.154 |
| L2/Volatile/4096 | ParquetDeltaBrotli | 10.846 | 2.03 | 2.91 | 1.395–1.430 |
| L2/Volatile/4096 | ParquetDeltaLz4 | 10.981 | 7.71 | 27.29 | 0.148–0.151 |
| L2/Volatile/4096 | ParquetDeltaSnappy | 10.969 | 7.75 | 26.99 | 0.151–0.154 |
| L2/Volatile/4096 | ParquetDictionaryLz4 | 37.752 | 2.55 | 10.07 | 0.406–0.421 |
| L2/Volatile/4096 | ParquetPlainLz4 | 30.573 | 4.69 | 10.65 | 0.382–0.387 |
| L2/Volatile/65536 | ColumnLz4 | 29.914 | 5.47 | 13.59 | 4.799–4.920 |
| L2/Volatile/65536 | DeltaBitpackLz4 | 10.448 | 21.55 | 16.05 | 4.069–4.152 |
| L2/Volatile/65536 | DeltaVarintLz4 | 12.284 | 23.02 | 26.53 | 2.467–2.489 |
| L2/Volatile/65536 | ParquetDeltaBrotli | 10.228 | 3.55 | 11.09 | 5.781–6.024 |
| L2/Volatile/65536 | ParquetDeltaLz4 | 10.357 | 9.29 | 44.50 | 1.457–1.507 |
| L2/Volatile/65536 | ParquetDeltaSnappy | 10.366 | 9.29 | 43.62 | 1.463–1.514 |
| L2/Volatile/65536 | ParquetDictionaryLz4 | 36.774 | 1.83 | 12.55 | 5.190–5.261 |
| L2/Volatile/65536 | ParquetPlainLz4 | 29.951 | 4.81 | 12.22 | 5.279–5.480 |
| Quote/Quiet/262144 | ColumnLz4 | 29.342 | 4.67 | 10.56 | 19.336–25.166 |
| Quote/Quiet/262144 | DeltaBitpackLz4 | 3.278 | 19.51 | 12.09 | 14.875–21.977 |
| Quote/Quiet/262144 | DeltaVarintLz4 | 3.992 | 21.53 | 19.08 | 8.809–15.996 |
| Quote/Quiet/262144 | ParquetDeltaBrotli | 2.597 | 2.96 | 7.24 | 26.032–36.722 |
| Quote/Quiet/262144 | ParquetDeltaLz4 | 3.323 | 8.18 | 20.05 | 9.970–13.271 |
| Quote/Quiet/262144 | ParquetDeltaSnappy | 3.438 | 8.32 | 19.73 | 7.264–18.198 |
| Quote/Quiet/262144 | ParquetDictionaryLz4 | 17.311 | 2.70 | 14.04 | 17.297–18.782 |
| Quote/Quiet/262144 | ParquetPlainLz4 | 29.361 | 3.93 | 9.09 | 23.343–29.712 |
| Quote/Quiet/4096 | ColumnLz4 | 29.504 | 6.03 | 14.52 | 0.281–0.322 |
| Quote/Quiet/4096 | DeltaBitpackLz4 | 3.389 | 22.24 | 17.57 | 0.233–0.299 |
| Quote/Quiet/4096 | DeltaVarintLz4 | 4.602 | 21.73 | 28.64 | 0.143–0.204 |
| Quote/Quiet/4096 | ParquetDeltaBrotli | 3.562 | 1.78 | 1.23 | 2.075–3.786 |
| Quote/Quiet/4096 | ParquetDeltaLz4 | 3.994 | 6.63 | 17.74 | 0.226–0.232 |
| Quote/Quiet/4096 | ParquetDeltaSnappy | 4.143 | 6.79 | 18.24 | 0.145–0.225 |
| Quote/Quiet/4096 | ParquetDictionaryLz4 | 29.311 | 2.50 | 10.04 | 0.386–0.511 |
| Quote/Quiet/4096 | ParquetPlainLz4 | 30.096 | 4.10 | 8.21 | 0.376–0.530 |
| Quote/Quiet/65536 | ColumnLz4 | 29.352 | 5.69 | 13.51 | 4.747–5.086 |
| Quote/Quiet/65536 | DeltaBitpackLz4 | 3.287 | 22.91 | 17.55 | 3.667–5.403 |
| Quote/Quiet/65536 | DeltaVarintLz4 | 4.010 | 22.63 | 29.67 | 2.195–3.084 |
| Quote/Quiet/65536 | ParquetDeltaBrotli | 2.619 | 2.50 | 6.80 | 6.579–9.988 |
| Quote/Quiet/65536 | ParquetDeltaLz4 | 3.343 | 8.22 | 19.07 | 3.264–3.653 |
| Quote/Quiet/65536 | ParquetDeltaSnappy | 3.429 | 8.44 | 18.59 | 1.508–3.746 |
| Quote/Quiet/65536 | ParquetDictionaryLz4 | 30.700 | 1.86 | 11.85 | 4.863–7.526 |
| Quote/Quiet/65536 | ParquetPlainLz4 | 29.389 | 4.56 | 7.60 | 6.841–10.160 |
| Quote/Volatile/262144 | ColumnLz4 | 37.160 | 4.43 | 13.31 | 19.296–25.827 |
| Quote/Volatile/262144 | DeltaBitpackLz4 | 13.985 | 17.84 | 14.31 | 18.127–22.841 |
| Quote/Volatile/262144 | DeltaVarintLz4 | 16.704 | 17.65 | 25.41 | 10.242–16.206 |
| Quote/Volatile/262144 | ParquetDeltaBrotli | 13.851 | 3.17 | 12.46 | 20.673–22.016 |
| Quote/Volatile/262144 | ParquetDeltaLz4 | 13.879 | 8.18 | 21.96 | 6.876–12.320 |
| Quote/Volatile/262144 | ParquetDeltaSnappy | 13.881 | 8.42 | 37.49 | 6.732–12.219 |
| Quote/Volatile/262144 | ParquetDictionaryLz4 | 33.143 | 1.96 | 15.00 | 17.188–18.786 |
| Quote/Volatile/262144 | ParquetPlainLz4 | 37.187 | 3.55 | 11.28 | 22.795–29.121 |
| Quote/Volatile/4096 | ColumnLz4 | 37.379 | 4.89 | 14.44 | 0.283–0.288 |
| Quote/Volatile/4096 | DeltaBitpackLz4 | 14.010 | 17.62 | 14.37 | 0.284–0.286 |
| Quote/Volatile/4096 | DeltaVarintLz4 | 16.761 | 20.21 | 26.02 | 0.157–0.158 |
| Quote/Volatile/4096 | ParquetDeltaBrotli | 14.472 | 1.82 | 4.23 | 0.963–0.993 |
| Quote/Volatile/4096 | ParquetDeltaLz4 | 14.494 | 7.59 | 27.69 | 0.147–0.153 |
| Quote/Volatile/4096 | ParquetDeltaSnappy | 14.493 | 7.59 | 27.33 | 0.149–0.151 |
| Quote/Volatile/4096 | ParquetDictionaryLz4 | 49.751 | 2.22 | 8.88 | 0.460–0.467 |
| Quote/Volatile/4096 | ParquetPlainLz4 | 37.971 | 4.08 | 10.81 | 0.378–0.380 |
| Quote/Volatile/65536 | ColumnLz4 | 37.180 | 4.54 | 13.87 | 4.706–4.934 |
| Quote/Volatile/65536 | DeltaBitpackLz4 | 13.985 | 19.12 | 14.31 | 4.551–4.651 |
| Quote/Volatile/65536 | DeltaVarintLz4 | 16.704 | 20.45 | 25.48 | 2.556–2.576 |
| Quote/Volatile/65536 | ParquetDeltaBrotli | 13.872 | 3.18 | 12.62 | 5.170–5.358 |
| Quote/Volatile/65536 | ParquetDeltaLz4 | 13.902 | 8.97 | 43.33 | 1.462–1.685 |
| Quote/Volatile/65536 | ParquetDeltaSnappy | 13.904 | 8.90 | 42.03 | 1.499–2.880 |
| Quote/Volatile/65536 | ParquetDictionaryLz4 | 48.908 | 1.49 | 10.64 | 6.077–6.303 |
| Quote/Volatile/65536 | ParquetPlainLz4 | 37.218 | 4.06 | 12.12 | 5.286–6.855 |
| Trade/Quiet/262144 | ColumnLz4 | 29.928 | 5.54 | 11.85 | 22.033–22.296 |
| Trade/Quiet/262144 | DeltaBitpackLz4 | 1.892 | 22.36 | 18.38 | 14.199–14.395 |
| Trade/Quiet/262144 | DeltaVarintLz4 | 2.075 | 24.28 | 31.06 | 8.414–8.608 |
| Trade/Quiet/262144 | ParquetDeltaBrotli | 1.191 | 3.67 | 12.97 | 19.995–21.902 |
| Trade/Quiet/262144 | ParquetDeltaLz4 | 1.919 | 8.89 | 39.77 | 6.383–9.778 |
| Trade/Quiet/262144 | ParquetDeltaSnappy | 2.010 | 8.94 | 39.71 | 6.503–6.874 |
| Trade/Quiet/262144 | ParquetDictionaryLz4 | 18.744 | 2.75 | 17.30 | 14.986–16.721 |
| Trade/Quiet/262144 | ParquetPlainLz4 | 29.945 | 4.51 | 10.28 | 24.824–27.096 |
| Trade/Quiet/4096 | ColumnLz4 | 30.021 | 5.86 | 12.50 | 0.326–0.331 |
| Trade/Quiet/4096 | DeltaBitpackLz4 | 1.998 | 22.93 | 18.36 | 0.223–0.223 |
| Trade/Quiet/4096 | DeltaVarintLz4 | 2.687 | 25.45 | 30.34 | 0.135–0.135 |
| Trade/Quiet/4096 | ParquetDeltaBrotli | 2.140 | 2.56 | 2.13 | 1.920–1.943 |
| Trade/Quiet/4096 | ParquetDeltaLz4 | 2.600 | 7.91 | 28.04 | 0.144–0.149 |
| Trade/Quiet/4096 | ParquetDeltaSnappy | 2.740 | 8.10 | 28.39 | 0.143–0.149 |
| Trade/Quiet/4096 | ParquetDictionaryLz4 | 33.133 | 2.72 | 10.33 | 0.393–0.405 |
| Trade/Quiet/4096 | ParquetPlainLz4 | 30.610 | 4.79 | 9.79 | 0.418–0.421 |
| Trade/Quiet/65536 | ColumnLz4 | 29.933 | 5.58 | 12.11 | 5.373–5.427 |
| Trade/Quiet/65536 | DeltaBitpackLz4 | 1.904 | 24.21 | 18.59 | 3.508–3.637 |
| Trade/Quiet/65536 | DeltaVarintLz4 | 2.091 | 25.97 | 31.44 | 2.079–2.125 |
| Trade/Quiet/65536 | ParquetDeltaBrotli | 1.214 | 3.75 | 12.96 | 4.981–5.169 |
| Trade/Quiet/65536 | ParquetDeltaLz4 | 1.940 | 9.43 | 46.83 | 1.342–1.494 |
| Trade/Quiet/65536 | ParquetDeltaSnappy | 2.069 | 9.52 | 45.35 | 1.424–1.486 |
| Trade/Quiet/65536 | ParquetDictionaryLz4 | 34.782 | 1.86 | 12.17 | 5.241–5.500 |
| Trade/Quiet/65536 | ParquetPlainLz4 | 29.971 | 4.92 | 11.02 | 5.849–6.105 |
| Trade/Volatile/262144 | ColumnLz4 | 34.065 | 4.86 | 11.89 | 21.830–22.124 |
| Trade/Volatile/262144 | DeltaBitpackLz4 | 10.195 | 20.05 | 16.30 | 16.016–16.320 |
| Trade/Volatile/262144 | DeltaVarintLz4 | 12.274 | 20.79 | 27.01 | 9.670–9.731 |
| Trade/Volatile/262144 | ParquetDeltaBrotli | 9.960 | 3.49 | 10.98 | 23.519–28.293 |
| Trade/Volatile/262144 | ParquetDeltaLz4 | 10.087 | 8.56 | 39.26 | 6.476–12.590 |
| Trade/Volatile/262144 | ParquetDeltaSnappy | 10.087 | 8.68 | 38.95 | 6.669–7.105 |
| Trade/Volatile/262144 | ParquetDictionaryLz4 | 27.776 | 2.23 | 15.74 | 16.114–25.300 |
| Trade/Volatile/262144 | ParquetPlainLz4 | 34.087 | 4.05 | 10.29 | 24.827–26.747 |
| Trade/Volatile/4096 | ColumnLz4 | 34.115 | 5.27 | 12.61 | 0.323–0.342 |
| Trade/Volatile/4096 | DeltaBitpackLz4 | 10.230 | 19.63 | 16.34 | 0.250–0.251 |
| Trade/Volatile/4096 | DeltaVarintLz4 | 12.411 | 21.94 | 27.21 | 0.150–0.151 |
| Trade/Volatile/4096 | ParquetDeltaBrotli | 10.597 | 2.06 | 2.95 | 1.383–1.433 |
| Trade/Volatile/4096 | ParquetDeltaLz4 | 10.707 | 7.85 | 28.24 | 0.143–0.147 |
| Trade/Volatile/4096 | ParquetDeltaSnappy | 10.707 | 7.81 | 27.59 | 0.147–0.152 |
| Trade/Volatile/4096 | ParquetDictionaryLz4 | 43.447 | 2.38 | 9.08 | 0.449–0.458 |
| Trade/Volatile/4096 | ParquetPlainLz4 | 34.704 | 4.36 | 9.81 | 0.417–0.421 |
| Trade/Volatile/65536 | ColumnLz4 | 34.079 | 4.95 | 12.26 | 5.327–5.461 |
| Trade/Volatile/65536 | DeltaBitpackLz4 | 10.197 | 21.71 | 16.27 | 4.008–4.037 |
| Trade/Volatile/65536 | DeltaVarintLz4 | 12.281 | 22.77 | 27.06 | 2.412–2.446 |
| Trade/Volatile/65536 | ParquetDeltaBrotli | 9.977 | 3.44 | 11.33 | 5.709–5.837 |
| Trade/Volatile/65536 | ParquetDeltaLz4 | 10.110 | 9.25 | 45.27 | 1.412–1.614 |
| Trade/Volatile/65536 | ParquetDeltaSnappy | 10.110 | 9.24 | 44.05 | 1.452–1.534 |
| Trade/Volatile/65536 | ParquetDictionaryLz4 | 42.979 | 1.61 | 11.00 | 5.809–6.134 |
| Trade/Volatile/65536 | ParquetPlainLz4 | 34.117 | 4.44 | 10.99 | 5.927–6.203 |

## Parquet size/decode Pareto frontier

Frontier membership ignores encode speed; inspect encode results separately.

| Case | Nondominated Parquet variants |
| --- | --- |
| L2/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| L2/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| L2/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Quote/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Quote/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Quiet/262144 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Quiet/4096 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Quiet/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/262144 | ParquetDeltaBrotli, ParquetDeltaLz4, ParquetDeltaSnappy |
| Trade/Volatile/4096 | ParquetDeltaBrotli, ParquetDeltaLz4 |
| Trade/Volatile/65536 | ParquetDeltaBrotli, ParquetDeltaLz4 |
