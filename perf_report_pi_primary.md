# Benchmark comparison

aarch64, 4 logical CPUs, governor performance, kernel 6.12.87+rpt-rpi-2712, clocksource arch_sys_counter, rustc 1.99.0 (b940084d7 2026-09-28); report made at commit 87ec045 with uncommitted changes.

## push_pop

push one record and pop it back, one thread. Median, time per operation, lower is better.

| |  |
|---|---:|
| spsc | 19.6 ns |
| mpsc | 27.7 ns |
| mpsc-primary | 25.7 ns |
| rtrb | **13.6 ns** |
| sync_channel | 44.2 ns |
| arrayqueue | 28.0 ns |
| mutex | 38.1 ns |

## push_full

push into a full queue, one thread: the price of a refusal. Median, time per operation, lower is better.

| |  |
|---|---:|
| spsc | **1.6 ns** |
| mpsc | 5.7 ns |
| mpsc-primary | 5.1 ns |
| rtrb | 2.9 ns |
| sync_channel | 11.9 ns |
| arrayqueue | 11.9 ns |
| mutex | 16.3 ns |

## pop_empty

pop from an empty queue, one thread: the price of one look. Median, time per operation, lower is better.

| |  |
|---|---:|
| spsc/pop1 | 6.7 ns |
| spsc/popN | 6.8 ns |
| mpsc/pop1 | 7.3 ns |
| mpsc/popN | 7.2 ns |
| mpsc-primary/pop1 | 7.3 ns |
| mpsc-primary/popN | 7.2 ns |
| rtrb/pop1 | 1.8 ns |
| rtrb/popN | **1.7 ns** |
| sync_channel/pop1 | 12.7 ns |
| sync_channel/popN | 12.5 ns |
| arrayqueue/pop1 | 10.3 ns |
| arrayqueue/popN | 5.8 ns |
| mutex/pop1 | 14.6 ns |
| mutex/popN | 20.4 ns |

## fill_drain

fill to capacity, then drain, one thread; columns are capacities. Median, records per second, higher is better.

| | 64 | 1024 | 16384 |
|---|---:|---:|---:|
| spsc/push1_pop1 | 52.6 M/s | 53.0 M/s | 50.8 M/s |
| spsc/push1_popN | 92.8 M/s | 93.9 M/s | 88.9 M/s |
| mpsc/push1_pop1 | 35.9 M/s | 36.1 M/s | 20.5 M/s |
| mpsc/push1_popN | 46.2 M/s | 46.6 M/s | 21.0 M/s |
| mpsc/push32_pop1 | 73.1 M/s | 74.0 M/s | 72.7 M/s |
| mpsc/push32_popN | 134.2 M/s | 137.2 M/s | 131.6 M/s |
| mpsc-primary/push1_pop1 | 46.1 M/s | 46.5 M/s | 23.5 M/s |
| mpsc-primary/push1_popN | 64.2 M/s | 65.4 M/s | 27.5 M/s |
| rtrb/push1_pop1 | 151.5 M/s | 163.1 M/s | 159.2 M/s |
| rtrb/push1_popN | 311.9 M/s | 345.8 M/s | 320.3 M/s |
| rtrb/push32_pop1 | 247.0 M/s | 260.4 M/s | 258.6 M/s |
| rtrb/push32_popN | **1.17 G/s** | **1.54 G/s** | **1.41 G/s** |
| sync_channel/push1_pop1 | 25.4 M/s | 25.6 M/s | 24.7 M/s |
| sync_channel/push1_popN | 25.5 M/s | 25.7 M/s | 25.1 M/s |
| arrayqueue/push1_pop1 | 35.2 M/s | 35.2 M/s | 35.2 M/s |
| arrayqueue/push1_popN | 35.4 M/s | 35.7 M/s | 34.8 M/s |
| mutex/push1_pop1 | 26.9 M/s | 26.9 M/s | 26.9 M/s |
| mutex/push1_popN | 46.5 M/s | 47.3 M/s | 47.1 M/s |
| mutex/push32_pop1 | 52.0 M/s | 52.1 M/s | 52.1 M/s |
| mutex/push32_popN | 286.6 M/s | 321.6 M/s | 311.9 M/s |

## spsc

one producer thread, one consumer thread; columns are capacities. Median, records per second, higher is better.

| | 64 | 1024 | 16384 |
|---|---:|---:|---:|
| spsc/push1_pop1 | 11.6 M/s | 11.8 M/s | 12.0 M/s |
| spsc/push1_popN | 12.4 M/s | 12.4 M/s | 12.6 M/s |
| mpsc-primary/push1_pop1 | 14.6 M/s | 20.6 M/s | 16.8 M/s |
| mpsc-primary/push1_popN | 17.0 M/s | 16.9 M/s | 17.0 M/s |
| rtrb/push1_pop1 | 53.3 M/s | 55.5 M/s | 57.0 M/s |
| rtrb/push1_popN | 140.1 M/s | 190.6 M/s | 244.6 M/s |
| rtrb/push32_pop1 | 53.3 M/s | 55.5 M/s | 57.1 M/s |
| rtrb/push32_popN | **181.5 M/s** | **208.9 M/s** | **578.9 M/s** |
| sync_channel/push1_pop1 | 8.3 M/s | 8.4 M/s | 9.3 M/s |
| sync_channel/push1_popN | 8.7 M/s | 20.2 M/s | 22.9 M/s |
| arrayqueue/push1_pop1 | 11.0 M/s | 19.8 M/s | 30.7 M/s |
| arrayqueue/push1_popN | 12.1 M/s | 9.8 M/s | 22.4 M/s |
| mutex/push1_pop1 | 3.0 M/s | 3.1 M/s | 3.2 M/s |
| mutex/push1_popN | 2.6 M/s | 2.7 M/s | 2.7 M/s |
| mutex/push32_pop1 | 2.9 M/s | 3.0 M/s | 3.0 M/s |
| mutex/push32_popN | 35.5 M/s | 40.4 M/s | 37.7 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| spsc/arrayqueue/push1_pop1/1024 | 0.010 | 0.011 |
| spsc/arrayqueue/push1_pop1/16384 | 0.001 | 0.007 |
| spsc/arrayqueue/push1_pop1/64 | 0.012 | 0.034 |
| spsc/arrayqueue/push1_popN/1024 | 0.023 | 0.012 |
| spsc/arrayqueue/push1_popN/16384 | 0.005 | 0.002 |
| spsc/arrayqueue/push1_popN/64 | 0.016 | 0.013 |
| spsc/mpsc-primary/push1_pop1/1024 | 0.021 | 0.044 |
| spsc/mpsc-primary/push1_pop1/16384 | 0.014 | 0.137 |
| spsc/mpsc-primary/push1_pop1/64 | 0.079 | 0.041 |
| spsc/mpsc-primary/push1_popN/1024 | 0.019 | 0.245 |
| spsc/mpsc-primary/push1_popN/16384 | 0.006 | 0.259 |
| spsc/mpsc-primary/push1_popN/64 | 0.023 | 0.185 |
| spsc/mutex/push1_pop1/1024 | 0.113 | 0.034 |
| spsc/mutex/push1_pop1/16384 | 0.079 | 0.036 |
| spsc/mutex/push1_pop1/64 | 0.149 | 0.077 |
| spsc/mutex/push1_popN/1024 | 0.081 | 0.591 |
| spsc/mutex/push1_popN/16384 | 0.026 | 0.523 |
| spsc/mutex/push1_popN/64 | 0.041 | 0.597 |
| spsc/mutex/push32_pop1/1024 | 0.535 | 0.029 |
| spsc/mutex/push32_pop1/16384 | 0.511 | 0.028 |
| spsc/mutex/push32_pop1/64 | 0.573 | 0.056 |
| spsc/mutex/push32_popN/1024 | 0.004 | 0.111 |
| spsc/mutex/push32_popN/16384 | 0.003 | 0.123 |
| spsc/mutex/push32_popN/64 | 0.012 | 0.131 |
| spsc/rtrb/push1_pop1/1024 | 0.017 | 0.005 |
| spsc/rtrb/push1_pop1/16384 | 0.022 | 0.002 |
| spsc/rtrb/push1_pop1/64 | 0.020 | 0.005 |
| spsc/rtrb/push1_popN/1024 | 0.108 | 0.014 |
| spsc/rtrb/push1_popN/16384 | 0.001 | 0.021 |
| spsc/rtrb/push1_popN/64 | 0.052 | 0.005 |
| spsc/rtrb/push32_pop1/1024 | 0.006 | 0.004 |
| spsc/rtrb/push32_pop1/16384 | 0.021 | 0.003 |
| spsc/rtrb/push32_pop1/64 | 0.024 | 0.005 |
| spsc/rtrb/push32_popN/1024 | 0.106 | 0.001 |
| spsc/rtrb/push32_popN/16384 | 0.002 | 0.000 |
| spsc/rtrb/push32_popN/64 | 0.093 | 0.001 |
| spsc/spsc/push1_pop1/1024 | 0.040 | 0.018 |
| spsc/spsc/push1_pop1/16384 | 0.011 | 0.022 |
| spsc/spsc/push1_pop1/64 | 0.119 | 0.031 |
| spsc/spsc/push1_popN/1024 | 0.033 | 0.059 |
| spsc/spsc/push1_popN/16384 | 0.016 | 0.068 |
| spsc/spsc/push1_popN/64 | 0.042 | 0.056 |
| spsc/sync_channel/push1_pop1/1024 | 0.027 | 0.010 |
| spsc/sync_channel/push1_pop1/16384 | 0.011 | 0.008 |
| spsc/sync_channel/push1_pop1/64 | 0.025 | 0.018 |
| spsc/sync_channel/push1_popN/1024 | 0.008 | 0.005 |
| spsc/sync_channel/push1_popN/16384 | 0.004 | 0.004 |
| spsc/sync_channel/push1_popN/64 | 0.025 | 0.022 |

</details>

## spsc_payload

one producer, one consumer, 1024 slots; columns are record sizes in bytes. Median, records per second, higher is better.

| | 8 | 64 | 256 |
|---|---:|---:|---:|
| spsc/push1_popN | 12.4 M/s | 10.7 M/s | 9.5 M/s |
| rtrb/push1_popN | **191.8 M/s** | **125.1 M/s** | **28.0 M/s** |
| sync_channel/push1_popN | 10.9 M/s | 7.8 M/s | 11.9 M/s |
| arrayqueue/push1_popN | 18.6 M/s | 9.9 M/s | 9.9 M/s |
| mutex/push1_popN | 6.4 M/s | 2.7 M/s | 1.9 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| spsc_payload/arrayqueue/push1_popN/256 | 0.009 | 0.015 |
| spsc_payload/arrayqueue/push1_popN/64 | 0.010 | 0.009 |
| spsc_payload/arrayqueue/push1_popN/8 | 0.010 | 0.011 |
| spsc_payload/mutex/push1_popN/256 | 0.066 | 1.612 |
| spsc_payload/mutex/push1_popN/64 | 0.062 | 0.641 |
| spsc_payload/mutex/push1_popN/8 | 0.043 | 0.402 |
| spsc_payload/rtrb/push1_popN/256 | 0.011 | 0.133 |
| spsc_payload/rtrb/push1_popN/64 | 0.006 | 0.103 |
| spsc_payload/rtrb/push1_popN/8 | 0.107 | 0.013 |
| spsc_payload/spsc/push1_popN/256 | 0.046 | 0.159 |
| spsc_payload/spsc/push1_popN/64 | 0.017 | 0.038 |
| spsc_payload/spsc/push1_popN/8 | 0.031 | 0.052 |
| spsc_payload/sync_channel/push1_popN/256 | 0.011 | 0.017 |
| spsc_payload/sync_channel/push1_popN/64 | 0.018 | 0.007 |
| spsc_payload/sync_channel/push1_popN/8 | 0.014 | 0.004 |

</details>

## spsc_pinned

one producer, one consumer, 1024 slots, on SMT siblings of one core and on two cores. Median, records per second, higher is better.

| | cores |
|---|---:|
| spsc/push1_pop1 | 11.9 M/s |
| rtrb/push1_pop1 | **55.9 M/s** |
| sync_channel/push1_pop1 | 8.4 M/s |
| arrayqueue/push1_pop1 | 21.7 M/s |
| mutex/push1_pop1 | 2.9 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| spsc_pinned/arrayqueue/push1_pop1/cores | 0.001 | 0.003 |
| spsc_pinned/mutex/push1_pop1/cores | 0.162 | 0.001 |
| spsc_pinned/rtrb/push1_pop1/cores | 0.012 | 0.002 |
| spsc_pinned/spsc/push1_pop1/cores | 0.036 | 0.011 |
| spsc_pinned/sync_channel/push1_pop1/cores | 0.001 | 0.005 |

</details>

## batch

one producer pushing that many records per operation, 1024 slots. Median, records per second, higher is better.

| | 1 | 8 | 32 | 128 |
|---|---:|---:|---:|---:|
| spsc | 12.7 M/s | — | — | — |
| mpsc | 10.9 M/s | 105.0 M/s | 131.0 M/s | 131.7 M/s |
| rtrb | **206.5 M/s** | **193.7 M/s** | **208.3 M/s** | **209.1 M/s** |
| sync_channel | 17.8 M/s | — | — | — |
| arrayqueue | 18.5 M/s | — | — | — |
| mutex | 6.0 M/s | 16.1 M/s | 27.7 M/s | 99.6 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| batch/arrayqueue/1 | 0.006 | 0.006 |
| batch/mpsc/1 | 0.013 | 0.042 |
| batch/mpsc/128 | 0.208 | 0.001 |
| batch/mpsc/32 | 0.144 | 0.001 |
| batch/mpsc/8 | 0.004 | 0.001 |
| batch/mutex/1 | 0.023 | 0.420 |
| batch/mutex/128 | 0.001 | 0.017 |
| batch/mutex/32 | 0.003 | 0.190 |
| batch/mutex/8 | 0.010 | 0.126 |
| batch/rtrb/1 | 0.080 | 0.002 |
| batch/rtrb/128 | 0.125 | 0.001 |
| batch/rtrb/32 | 0.108 | 0.001 |
| batch/rtrb/8 | 0.092 | 0.008 |
| batch/spsc/1 | 0.020 | 0.052 |
| batch/sync_channel/1 | 0.009 | 0.003 |

</details>

## mpsc_producers

records split over that many producers, 1024 slots. Median, records per second, higher is better.

| | 1 | 2 | 3 |
|---|---:|---:|---:|
| mpsc/push1_popN | 10.9 M/s | 7.7 M/s | 6.9 M/s |
| mpsc/push32_popN | **130.3 M/s** | **127.7 M/s** | **124.9 M/s** |
| sync_channel/push1_popN | 20.2 M/s | 8.9 M/s | 7.5 M/s |
| arrayqueue/push1_popN | 21.6 M/s | 10.1 M/s | 9.7 M/s |
| mutex/push1_popN | 3.5 M/s | 7.7 M/s | 5.1 M/s |
| mutex/push32_popN | 36.0 M/s | 62.6 M/s | 61.5 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| mpsc_producers/arrayqueue/push1_popN/1 | 0.009 | 0.010 |
| mpsc_producers/arrayqueue/push1_popN/2 | 0.013 | 0.006 |
| mpsc_producers/arrayqueue/push1_popN/3 | 0.107 | 0.010 |
| mpsc_producers/mpsc/push1_popN/1 | 0.190 | 0.066 |
| mpsc_producers/mpsc/push1_popN/2 | 0.024 | 0.390 |
| mpsc_producers/mpsc/push1_popN/3 | 0.510 | 0.672 |
| mpsc_producers/mpsc/push32_popN/1 | 0.157 | 0.001 |
| mpsc_producers/mpsc/push32_popN/2 | 0.454 | 0.002 |
| mpsc_producers/mpsc/push32_popN/3 | 0.723 | 0.004 |
| mpsc_producers/mutex/push1_popN/1 | 0.014 | 0.326 |
| mpsc_producers/mutex/push1_popN/2 | 0.002 | 0.184 |
| mpsc_producers/mutex/push1_popN/3 | 0.020 | 0.107 |
| mpsc_producers/mutex/push32_popN/1 | 0.003 | 0.190 |
| mpsc_producers/mutex/push32_popN/2 | 0.003 | 0.043 |
| mpsc_producers/mutex/push32_popN/3 | 0.005 | 0.029 |
| mpsc_producers/sync_channel/push1_popN/1 | 0.011 | 0.010 |
| mpsc_producers/sync_channel/push1_popN/2 | 0.042 | 0.013 |
| mpsc_producers/sync_channel/push1_popN/3 | 0.146 | 0.123 |

</details>

## mpsc_capacity

four producers; columns are capacities. Median, records per second, higher is better.

| | 64 | 16384 |
|---|---:|---:|
| mpsc/push1_popN | 6.9 M/s | 7.1 M/s |
| mpsc/push32_popN | **65.4 M/s** | **137.8 M/s** |
| sync_channel/push1_popN | 7.1 M/s | 7.6 M/s |
| arrayqueue/push1_popN | 9.3 M/s | 10.1 M/s |
| mutex/push1_popN | 4.8 M/s | 5.0 M/s |
| mutex/push32_popN | 30.4 M/s | 63.0 M/s |

<details><summary>Spins per record on a full and an empty queue</summary>

| benchmark | full | empty |
|---|---:|---:|
| mpsc_capacity/arrayqueue/push1_popN/16384 | 0.039 | 0.011 |
| mpsc_capacity/arrayqueue/push1_popN/64 | 0.121 | 0.015 |
| mpsc_capacity/mpsc/push1_popN/16384 | 0.270 | 0.616 |
| mpsc_capacity/mpsc/push1_popN/64 | 0.337 | 0.664 |
| mpsc_capacity/mpsc/push32_popN/16384 | 0.619 | 0.004 |
| mpsc_capacity/mpsc/push32_popN/64 | 1.243 | 0.003 |
| mpsc_capacity/mutex/push1_popN/16384 | 0.014 | 0.101 |
| mpsc_capacity/mutex/push1_popN/64 | 0.052 | 0.108 |
| mpsc_capacity/mutex/push32_popN/16384 | 0.004 | 0.026 |
| mpsc_capacity/mutex/push32_popN/64 | 0.094 | 0.017 |
| mpsc_capacity/sync_channel/push1_popN/16384 | 0.085 | 0.117 |
| mpsc_capacity/sync_channel/push1_popN/64 | 0.152 | 0.139 |

</details>

## mpsc_oversubscribed

twice as many producers as logical CPUs, 1024 slots. Median, records per second, higher is better.

| | 8 |
|---|---:|
| mpsc/push1_popN | 4.4 M/s |
| mpsc/push32_popN | **79.9 M/s** |
| sync_channel/push1_popN | 6.1 M/s |
| arrayqueue/push1_popN | 6.4 M/s |
| mutex/push1_popN | 5.4 M/s |
| mutex/push32_popN | 32.0 M/s |

<details><summary>Spins per record on a full and an empty queue, longest wait between two receives</summary>

| benchmark | full | empty | max gap over samples |
|---|---:|---:|---:|
| mpsc_oversubscribed/arrayqueue/push1_popN/8 | 7.392 | 0.057 | 24.41 ms |
| mpsc_oversubscribed/mpsc/push1_popN/8 | 23.018 | 2.295 | 44.01 ms |
| mpsc_oversubscribed/mpsc/push32_popN/8 | 1.710 | 0.149 | 11.28 ms |
| mpsc_oversubscribed/mutex/push1_popN/8 | 0.182 | 0.141 | 9.44 ms |
| mpsc_oversubscribed/mutex/push32_popN/8 | 0.238 | 0.138 | 12.66 ms |
| mpsc_oversubscribed/sync_channel/push1_popN/8 | 8.267 | 0.075 | 24.06 ms |

</details>

