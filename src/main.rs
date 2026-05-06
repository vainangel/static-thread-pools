use rand::rngs::StdRng;
use rand::SeedableRng;
use rand::prelude::*;
use rand::distr::weighted::WeightedIndex;
use regex::Regex;
use std::env;

use std::time::{Duration, SystemTime};

use crate::td::task::{Task, TaskKind};
use crate::td::pool::*;

pub mod td;

// NOTE: Using a priority queue for tasks yields the same net runtime as
// using a regular queue (i.e., dispatching tasks as theyre enqueued). 
// Starvation over time is also the same. 
//
// NOTE: This is also the case when using fixed durations for all task kinds.

#[cfg(feature = "fifo_pool")]
macro_rules! query_type {
    () => { WorkerPool };
}

#[cfg(feature = "dynamic_pool")]
macro_rules! query_type {
    () => { DynamicWorkerPool };
}

#[cfg(not(any(feature = "fifo_pool", feature = "dynamic_pool")))]
macro_rules! query_type {
    () => {
        compile_error!("Must specify --features fifo_pool or --features pqueue_pool !");
    }
}

#[cfg(all(feature = "fifo_pool", feature = "dynamic_pool"))]
compile_error!("Features 'fifo_pool' and 'dynamic_pool' are mutually exclusive.");

fn main() {
    let seed: u64 = 123456;
    let mut rng = StdRng::seed_from_u64(seed);
    
    let mut tasks = vec![];
    let tasks_total = i32::from_str_radix(env::var("TASKS").unwrap_or_else(|_| "1000".to_string()).as_str(), 10).unwrap(); 
    // make sure to set this to 1,000 for final tests (required by rubric in
    // amendments)

    let arrival_time_0 = SystemTime::now() + Duration::new(5, 0);

    let dist_taskkind: WeightedIndex<i32> = match env::var("DIST") {
        Ok(dist_string) => { 
            match Regex::new(r"([0-9]+)\:([0-9]+)").unwrap().captures(dist_string.as_str()) {
                Some(captures) => {
                    let ratio_cpu = i32::from_str_radix(&captures[1], 10).unwrap();
                    let ratio_io = i32::from_str_radix(&captures[2], 10).unwrap();

                    Ok(WeightedIndex::new(&[ratio_cpu, ratio_io]).unwrap())
                },
                _ => Err("Format of parameter DIST is invalid (should be x:y)")
            }
        },
        _ => Ok(WeightedIndex::new(&[7, 3]).unwrap()),
    }.unwrap();

    let use_uniform_durations = match env::var("UNIFORM_DURATION") {
        Ok(value) => value != "0",
        _ => true,
    };

    let mut arrival_time_offset_20ms = 0;

    for id in 1..=tasks_total {
        let kind = vec![TaskKind::CPU, TaskKind::IO][dist_taskkind.sample(&mut rng)].clone();

        let duration = match use_uniform_durations {
            true => Duration::from_millis(200),
            false => Duration::from_millis(rng.random_range(100..1000))
        };

        let arrival_time = arrival_time_0 + Duration::from_millis(20 * arrival_time_offset_20ms);
        arrival_time_offset_20ms += 1;

        tasks.push(Task{
            id,
            arrival_time,
            kind,
            duration, 
            time_queued: SystemTime::now(),
        });
    }

    tasks.reverse();

    let mut pool = <query_type!()>::new(8);

    println!("Worker pool opened. Waiting for tasks...");

    let mut load_at_intervals = Vec::with_capacity(tasks_total as usize);

    while !tasks.is_empty() {
        if let Some(task) = tasks.last() && task.arrival_time <= SystemTime::now() {
            println!("Dispatching task {:?}, which will take about {:.3?}s...", 
                task.id, task.duration.as_secs_f32());

            let mut task = task.clone();
            task.time_queued = SystemTime::now();

            pool.execute_task(task);

            load_at_intervals.push(pool.load.lock().unwrap().clone());
            tasks.pop();
        }
    }
    pool.await_remaining_tasks();

    let sum: usize = load_at_intervals.iter().sum();
    let avg_load = sum / load_at_intervals.len();
    println!("Avg. system load: {:?}", avg_load);
}

