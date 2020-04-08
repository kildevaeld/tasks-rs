

perf:
	perf record -F99 --call-graph dwarf target/release/examples/simple

perf-report:
	perf report