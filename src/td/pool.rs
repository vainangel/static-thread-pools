use priority_queue::PriorityQueue;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread; 
use std::time::Duration;

use crate::td::task::{Task, TaskKind};

impl TaskKind {
    pub fn get_load(&self) -> usize {
        match self {
            TaskKind::CPU => 35,
            TaskKind::IO => 10,
        }
    }
}

// Since tasks are simulated and do not use input/output, we do not need to worry about data I/O,
// though it is worth researching https://doc.rust-lang.org/nomicon/send-and-sync.html 
pub struct Worker {
    id: usize, 
    pub thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn get_id(&self) -> usize { self.id }

    // NOTE What is the point in wrapping the receiver in an Arc<Mutex<_>> ? 
    pub fn new(id: usize, load: Arc<Mutex<usize>>, 
        receiver: Arc<Mutex<mpsc::Receiver<Option<Task>>>>) -> Self {
        let listen_for_tasks = move || loop {
            let task = receiver.lock().unwrap().recv().unwrap().clone();

            match task {
                Some(task) => {
                    let last_load = load.lock().unwrap().clone();
                    println!("Dispatching {:?} task with new load {:?}", task.kind, last_load);

                    thread::sleep(task.duration);

                    // TODO: Not relevant here, but handle premature callback termination?
                    // Unhandled premature task termination can lead to false `load` reporting.

                    // send task kind update (subtract from load) 
                    *load.lock().unwrap() -= task.kind.get_load();
                    println!("Finished {:?} task {:?} in {:.3?}s (About {:?}s after arrival time)", 
                        task.kind, task.id, 
                        SystemTime::now().duration_since(task.time_queued).unwrap_or(Duration::ZERO).as_secs_f32(),
                        SystemTime::now().duration_since(task.arrival_time).unwrap_or(Duration::ZERO).as_secs_f32());
                }
                None => break
            }
        };

        let thread = thread::spawn(listen_for_tasks);
        Worker{ id, thread: Some(thread) }
    }
}

use std::time::{SystemTime};

pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Option<Task>>,

    pub start: SystemTime,
    pub load: Arc<Mutex<usize>>,
}

impl WorkerPool {
    pub fn new(n_workers: usize) -> Self {
        assert!(n_workers > 0);

        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));

        let load = Arc::new(Mutex::new(0 as usize));

        let mut workers = Vec::<Worker>::with_capacity(n_workers);

        for id in 1..=n_workers {
            workers.push(Worker::new(id, load.clone(), Arc::clone(&rx)));
        }

        WorkerPool{ workers, sender: tx, start: SystemTime::now(), load }
    }

    pub fn execute_task(&self, task: Task) {
        // wait until resource is available 
        while *self.load.lock().unwrap() + task.kind.get_load() > 100 {
            // ...
        }

        // update load state and dispatch worker 
        *self.load.lock().unwrap() += task.kind.get_load();
        self.sender.send(Some(task)).unwrap();
    }

    pub fn await_remaining_tasks(&mut self) {
        println!("Finishing remaining tasks...");

        while *self.load.lock().unwrap() > 0 {
            // ...
        }

        let end = SystemTime::now();
        let elapsed = end.duration_since(self.start).unwrap();
        println!("Finished all jobs in {:.3?}s", elapsed.as_secs_f32());
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(None).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

pub struct DynamicWorker {
    id: usize,
    pub thread: Option<thread::JoinHandle<()>>,
}

impl DynamicWorker {
    pub fn get_id(&self) -> usize { self.id }

    // TODO: replace with AtomicUsize ? 
    pub fn new(id: usize, load: Arc<Mutex<usize>>, 
        rx: Arc<Mutex<mpsc::Receiver<Option<Task>>>>) -> Self {

        let listen_for_tasks = move || loop {
            let task = rx.lock().unwrap().recv().unwrap();

            match task {
                Some(task) => {
                    println!("Dispatching {:?} task {:?}", task.kind, task.id);
                    thread::sleep(task.duration);
                    *load.lock().unwrap() -= task.kind.get_load();

                    println!("Finished {:?} task {:?} in {:.3?}s (About {:?}s after arrival time)", 
                        task.kind, task.id, 
                        SystemTime::now().duration_since(task.time_queued).unwrap_or(Duration::ZERO).as_secs_f32(),
                        SystemTime::now().duration_since(task.arrival_time).unwrap_or(Duration::ZERO).as_secs_f32());
                },
                None => break,
            };

        };

        let thread = thread::spawn(listen_for_tasks);
        DynamicWorker{ id, thread: Some(thread) }
    }
}

pub struct DynamicWorkerPool {
    workers: Vec<DynamicWorker>,
    tx: mpsc::Sender<Option<Task>>,
    pub load: Arc<Mutex<usize>>,
    start: SystemTime,
    task_queue: PriorityQueue<Task, TaskKind>,
}

impl DynamicWorkerPool {
    pub fn new(n_workers: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));

        let load = Arc::new(Mutex::new(0 as usize));

        let mut workers = Vec::with_capacity(n_workers);
        for id in 1..=workers.capacity() {
            workers.push(DynamicWorker::new(id, load.clone(), rx.clone()));
        }

        let start = SystemTime::now();

        DynamicWorkerPool{ workers, tx, load, start, task_queue: PriorityQueue::new() }
    }

    pub fn execute_task(&mut self, task: Task) {
        self.task_queue.push(task.clone(), task.kind);

        let (task, kind) = self.task_queue.pop().unwrap();
        while *self.load.lock().unwrap() + kind.get_load() > 100 {
            // ...
        }

        *self.load.lock().unwrap() += kind.get_load();
        println!("Load: {:?}", self.load.lock().unwrap().clone());
        self.tx.send(Some(task)).unwrap();
    }

    pub fn await_remaining_tasks(&mut self) {
        println!("Finishing remaining tasks...");

        while *self.load.lock().unwrap() > 0 {
            // ...
        }

        let end = SystemTime::now();
        let elapsed = end.duration_since(self.start).unwrap();
        println!("Finished all jobs in {:.3?}s", elapsed.as_secs_f32());
    }

}

impl Drop for DynamicWorkerPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.tx.send(None).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
