use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::{self, Write};

use dwpy_core::{compile, DwError, ExecutionOptions, DEFAULT_MAX_MATERIALIZED_BYTES};
use serde_json::{json, Value};

struct ThreadTrackingAllocator;

thread_local! {
    static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static LIVE_BYTES: Cell<usize> = const { Cell::new(0) };
    static PEAK_BYTES: Cell<usize> = const { Cell::new(0) };
}

#[global_allocator]
static TEST_ALLOCATOR: ThreadTrackingAllocator = ThreadTrackingAllocator;

unsafe impl GlobalAlloc for ThreadTrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            track_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        track_deallocation(layout.size());
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, old, new_size) };
        if !new_pointer.is_null() {
            if new_size >= old.size() {
                track_allocation(new_size - old.size());
            } else {
                track_deallocation(old.size() - new_size);
            }
        }
        new_pointer
    }
}

fn track_allocation(bytes: usize) {
    let tracking = TRACK_ALLOCATIONS.try_with(Cell::get).unwrap_or(false);
    if !tracking {
        return;
    }
    let _ = LIVE_BYTES.try_with(|live| {
        let next = live.get().saturating_add(bytes);
        live.set(next);
        let _ = PEAK_BYTES.try_with(|peak| peak.set(peak.get().max(next)));
    });
}

fn track_deallocation(bytes: usize) {
    let tracking = TRACK_ALLOCATIONS.try_with(Cell::get).unwrap_or(false);
    if tracking {
        let _ = LIVE_BYTES.try_with(|live| live.set(live.get().saturating_sub(bytes)));
    }
}

fn peak_allocated_bytes<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    TRACK_ALLOCATIONS.with(|tracking| tracking.set(false));
    LIVE_BYTES.with(|live| live.set(0));
    PEAK_BYTES.with(|peak| peak.set(0));
    TRACK_ALLOCATIONS.with(|tracking| tracking.set(true));
    let result = operation();
    TRACK_ALLOCATIONS.with(|tracking| tracking.set(false));
    let peak = PEAK_BYTES.with(Cell::get);
    (result, peak)
}

fn options(render_output: bool) -> ExecutionOptions {
    ExecutionOptions {
        render_output,
        ..ExecutionOptions::default()
    }
}

#[test]
fn compiled_range_reduce_is_constant_memory_and_replayable() {
    let script = compile(
        "%dw 2.0\noutput application/json\n---\n1 to 1000000 reduce ((item, accum = 0) -> item + accum)",
    )
    .unwrap();
    let constrained = ExecutionOptions {
        render_output: true,
        max_materialized_bytes: 1,
        lazy_sequences: true,
    };

    assert_eq!(
        script.execute(Value::Null, None, &constrained).unwrap(),
        json!("500000500000")
    );
    assert_eq!(
        script.execute(Value::Null, None, &constrained).unwrap(),
        json!("500000500000")
    );
}

#[test]
fn range_reduce_peak_heap_does_not_scale_with_range_length() {
    let small = compile(
        "%dw 2.0\noutput application/python\n---\n1 to 10000 reduce ((item, accum = 0) -> item + accum)",
    )
    .unwrap();
    let large = compile(
        "%dw 2.0\noutput application/python\n---\n1 to 1000000 reduce ((item, accum = 0) -> item + accum)",
    )
    .unwrap();
    let execution_options = options(false);

    let (small_result, small_peak) =
        peak_allocated_bytes(|| small.execute(Value::Null, None, &execution_options));
    let (large_result, large_peak) =
        peak_allocated_bytes(|| large.execute(Value::Null, None, &execution_options));

    assert_eq!(small_result.unwrap(), json!(50005000));
    assert_eq!(large_result.unwrap(), json!(500000500000_i64));
    assert!(
        large_peak <= small_peak.saturating_add(64 * 1024),
        "peak heap grew with range length: 10k={small_peak} bytes, 1m={large_peak} bytes"
    );
}

#[test]
fn compiled_reduce_preserves_defaults_empty_results_and_descending_ranges() {
    let descending = compile(
        "%dw 2.0\noutput application/python\n---\n5 to 1 reduce ((item, accum) -> item + accum)",
    )
    .unwrap();
    assert_eq!(
        descending
            .execute(Value::Null, None, &options(false))
            .unwrap(),
        json!(15)
    );

    let with_default = compile(
        "%dw 2.0\noutput application/python\n---\n1 to 5 filter ((item) -> item > 10) reduce ((item, accum = 7) -> item + accum)",
    )
    .unwrap();
    assert_eq!(
        with_default
            .execute(Value::Null, None, &options(false))
            .unwrap(),
        json!(7)
    );

    let without_default = compile(
        "%dw 2.0\noutput application/python\n---\n1 to 5 filter ((item) -> item > 10) reduce ((item, accum) -> item + accum)",
    )
    .unwrap();
    assert_eq!(
        without_default
            .execute(Value::Null, None, &options(false))
            .unwrap(),
        Value::Null
    );

    let payload_reduce = compile(
        "%dw 2.0\noutput application/python\n---\npayload.items reduce ((item, accum = 0) -> accum + item.price)",
    )
    .unwrap();
    assert_eq!(
        payload_reduce
            .execute(
                json!({"items": [{"price": 4}, {"price": 7}]}),
                None,
                &options(false),
            )
            .unwrap(),
        json!(11)
    );
}

#[test]
fn lazy_map_filter_flat_map_and_distinct_match_eager_execution() {
    for source in [
        "1 to 8 map ((item, index) -> item + index) filter ((item) -> item > 5)",
        "1 to 4 flatMap ((item) -> [item, item + 1])",
        "1 to 6 map ((item) -> item / 2) distinctBy ((item) -> item)",
        "1 to 10 takeWhile ((item) -> item < 6) dropWhile ((item) -> item < 3)",
    ] {
        let script = compile(&format!(
            "%dw 2.0\noutput application/python\n---\n{source}"
        ))
        .unwrap();
        let lazy = script.execute(Value::Null, None, &options(false)).unwrap();
        let eager = script
            .execute(
                Value::Null,
                None,
                &ExecutionOptions {
                    render_output: false,
                    lazy_sequences: false,
                    ..ExecutionOptions::default()
                },
            )
            .unwrap();
        assert_eq!(lazy, eager, "parity failed for {source}");
    }
}

#[test]
fn lazy_terminals_match_eager_execution_and_short_circuit() {
    for source in [
        "1 to 20 some ((item) -> item > 10)",
        "1 to 20 every ((item) -> item > 0)",
        "1 to 20 firstWith ((item) -> item > 10)",
        "1 to 20 indexWhere ((item) -> item > 10)",
        "1 to 20 countBy ((item) -> item > 10)",
        "1 to 20 sumBy ((item) -> item * 2)",
    ] {
        let script = compile(&format!(
            "%dw 2.0\noutput application/python\n---\n{source}"
        ))
        .unwrap();
        let lazy = script.execute(Value::Null, None, &options(false)).unwrap();
        let eager = script
            .execute(
                Value::Null,
                None,
                &ExecutionOptions {
                    render_output: false,
                    lazy_sequences: false,
                    ..ExecutionOptions::default()
                },
            )
            .unwrap();
        assert_eq!(lazy, eager, "terminal parity failed for {source}");
    }

    let empty_every = compile(
        "%dw 2.0\noutput application/python\n---\n1 to 3 filter ((item) -> item > 10) every ((item) -> item > 0)",
    )
    .unwrap();
    assert_eq!(
        empty_every
            .execute(Value::Null, None, &options(false))
            .unwrap(),
        Value::Bool(false)
    );

    let payload_sum = compile(
        "%dw 2.0\noutput application/python\n---\npayload.items filter ((item) -> item.active == true) sumBy ((item) -> item.price)",
    )
    .unwrap();
    let payload = json!({
        "items": [
            {"active": true, "price": 4},
            {"active": false, "price": 100},
            {"active": true, "price": 7}
        ]
    });
    let lazy = payload_sum
        .execute(payload.clone(), None, &options(false))
        .unwrap();
    let eager = payload_sum
        .execute(
            payload,
            None,
            &ExecutionOptions {
                render_output: false,
                lazy_sequences: false,
                ..ExecutionOptions::default()
            },
        )
        .unwrap();
    assert_eq!(lazy, eager);
    assert_eq!(lazy, json!(11));
}

#[test]
fn materialization_is_bounded_and_reports_the_item() {
    let script = compile("%dw 2.0\noutput application/python\n---\n1 to 100").unwrap();
    let error = script
        .execute(
            Value::Null,
            None,
            &ExecutionOptions {
                render_output: false,
                max_materialized_bytes: 128,
                lazy_sequences: true,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        DwError::ResourceLimit {
            operation,
            limit_bytes: 128,
            estimated_bytes,
            item_index,
        } if operation == "materializing lazy sequence" && estimated_bytes > 128 && item_index < 100
    ));
    assert_eq!(DEFAULT_MAX_MATERIALIZED_BYTES, 256 * 1024 * 1024);
}

#[test]
fn sink_streams_json_ndjson_and_csv() {
    let cases = [
        (
            "%dw 2.0\noutput application/json\n---\n1 to 3 map ((item) -> [item, item + 1])",
            "[[1,2],[2,3],[3,4]]",
        ),
        (
            "%dw 2.0\noutput application/x-ndjson\n---\n1 to 3 map ((item) -> [item, item + 1])",
            "[1,2]\n[2,3]\n[3,4]\n",
        ),
        (
            "%dw 2.0\noutput application/json indent=2\n---\n1 to 2 map ((item) -> [item, item + 1])",
            "[\n  [\n    1,\n    2\n  ],\n  [\n    2,\n    3\n  ]\n]",
        ),
        (
            "%dw 2.0\noutput application/csv header=false\n---\n1 to 3 map ((item) -> [item, item + 1])",
            "1,2\n2,3\n3,4\n",
        ),
    ];
    for (source, expected) in cases {
        let script = compile(source).unwrap();
        let mut output = Vec::new();
        script
            .execute_to_writer(Value::Null, None, &ExecutionOptions::default(), &mut output)
            .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }
}

#[test]
fn sink_errors_are_non_atomic() {
    struct FailingWriter {
        bytes: Vec<u8>,
        remaining: usize,
    }

    impl Write for FailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::new(io::ErrorKind::Other, "sink full"));
            }
            let count = buffer.len().min(self.remaining);
            self.bytes.extend_from_slice(&buffer[..count]);
            self.remaining -= count;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let script = compile("%dw 2.0\noutput application/json\n---\n1 to 100").unwrap();
    let mut writer = FailingWriter {
        bytes: Vec::new(),
        remaining: 8,
    };
    let error = script
        .execute_to_writer(Value::Null, None, &ExecutionOptions::default(), &mut writer)
        .unwrap_err();
    assert!(matches!(error, DwError::Output(message) if message.contains("sink full")));
    assert!(!writer.bytes.is_empty());
    assert!(writer.bytes.starts_with(b"["));
}
