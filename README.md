# Systems Programming Final Project - Worker Pools
This project explores two types of task scheduling methods. The project constraints are simplified insofar that task quanta are fixed (to 200ms) so advanced task scheduling algorithms (which, in general, assign quanta to tasks dynamically) are not necessary to implement to optimize scheduling.

This project implements:
- A FIFO pool: Tasks are unqueued as soon as (possible as) they're queued 
- A round robin queue scheduler: Tasks are executed for some quantum of time per iteration and re-queued until they are complete.

# Building and Running 
To test a specific implementation:
```sh
cargo run --features fifo_pool # runs the basic fifo tests 
cargo run --features dynamic_pool # runs the round robin scheduler tests
```

To summarize the data:
```sh
cargo run --features <type> > out.txt
python3 analyze.py out.txt > results_<type>.txt
```

To automate this process for both tests:
```sh
python3 run_tests.py
```

> The command usage provided is unix-specific, and is only guaranteed to work on Mac or Linux. If you want to do this on Windows it's probably the same, though.

# Dependencies
- Cargo
- Python 3.11 or later 

