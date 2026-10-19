# oulu

OS: Linux 7.2.8 x86_64
CPU: 13th Gen Intel(R) Core(TM) i7-1355U
Compiler: rustc 1.99.1 (58cf8f9c3 2026-10-13)
Language: Rust 2024

Work counts user-space instructions of the whole process; paired startup subtraction retains noise.
Pinned to allowed CPU 0.
Wall microseconds include process launch, redirected IO and GNU time; no baseline subtraction.
CPU microseconds come from GNU time user+system at 10 ms resolution; small samples are coarse.
RSS KiB is GNU time maximum resident set per fresh process, peak across timing runs.
Solutions are interleaved per phase; work uses 5 full/empty pairs when available. Timing uses 10 runs, or 3 after a pilot exceeds 1 s.
