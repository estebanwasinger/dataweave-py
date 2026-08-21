# Rust Bridge vs Python DataWeave Performance

Generated at: `2026-06-01T11:05:03+00:00`

## Environment

- Python: `3.12.0`
- Platform: `macOS-26.1-arm64-arm-64bit`
- Executable: `/Users/estebanwasinger/dataweave-py/.venv/bin/python3`
- Package version: `0.4.0`
- Rust timing mode: `bridge`
- Rust cargo profile: `n/a`
- Warmup iterations per backend/scenario: `20`
- Measured iterations per backend/scenario: `200`

## Results

| Scenario | Iterations | Rust median ms | Rust mean ms | Rust p95 ms | Python median ms | Python mean ms | Python p95 ms | Rust speedup |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| basic_projection | 200 | 0.0888 | 0.1012 | 0.1590 | 0.1205 | 0.1304 | 0.1817 | 1.36x |
| array_map_filter_reduce | 200 | 3.0906 | 3.2494 | 4.2326 | 10.9647 | 11.3318 | 13.3765 | 3.55x |
| object_helpers | 200 | 0.7096 | 0.7217 | 0.8269 | 4.5816 | 4.6988 | 5.0731 | 6.46x |
| xml_selectors | 200 | 0.9439 | 0.9739 | 1.1176 | 1.2534 | 1.2947 | 1.4511 | 1.33x |
| json_render | 200 | 2.0735 | 2.1077 | 2.2704 | 4.2100 | 4.2459 | 4.5777 | 2.03x |
| csv_render | 200 | 0.4501 | 0.4775 | 0.5825 | 1.0008 | 1.0338 | 1.2038 | 2.22x |
| crypto_hash | 200 | 0.0362 | 0.0381 | 0.0380 | 1.2566 | 1.3069 | 1.5571 | 34.70x |
| period_arithmetic | 200 | 0.0747 | 0.0772 | 0.0849 | 0.2405 | 0.2572 | 0.3501 | 3.22x |
| nested_selectors_aggregation | 200 | 4.0545 | 4.0724 | 4.2953 | 19.8956 | 20.0193 | 20.6491 | 4.91x |
| group_order_distinct | 200 | 4.2506 | 4.2541 | 4.4299 | 7.9904 | 8.0019 | 8.3175 | 1.88x |
| object_map_filter | 200 | 0.5294 | 0.8265 | 0.9647 | 3.8189 | 3.8653 | 4.1945 | 7.21x |
| zip_unzip | 200 | 1.2070 | 1.2437 | 1.4161 | 2.2222 | 2.2614 | 2.4447 | 1.84x |
| string_functions | 200 | 0.3896 | 0.4079 | 0.4926 | 0.5653 | 0.5871 | 0.6961 | 1.45x |

## Interpretation

- Rust is faster in `13` of `13` scenarios in this run.
- Median Rust speedup across scenarios: `2.22x`.
- Geometric mean Rust speedup across scenarios: `3.20x`.

## Notes

- Results compare the current in-process Python bridge API, not raw Rust-only execution.
- Lower time is better.
- Speedup greater than `1.0x` means Rust is faster.
- Results are local-machine measurements, not CI performance guarantees.
- Each scenario is validated for equal normalized Rust and Python output before timing.

## Lazy Range/Reduce Microbenchmark

Measured on `2026-08-20` using the release-mode native CLI on macOS arm64:

```dataweave
%dw 2.0
output application/json
---
1 to 1000000 reduce ((item, accum = 0) -> item + accum)
```

| Engine path | Wall time | Peak RSS | Result |
|---|---:|---:|---:|
| Previous eager range and interpreted reducer | ~8.3 s | ~180 MB | `500000500000` |
| Compiled lazy range and slot-based reducer | ~0.04 s | ~2.4 MB | `500000500000` |

The pure-Rust benchmark harness reports approximately `35 ms` for the one
million-item scenario after warmup. The sequence is iterated rather than
replaced with a closed-form sum, so reducer evaluation order and errors remain
observable. The range itself and numeric accumulator are not materialized as
`serde_json::Value` instances.
