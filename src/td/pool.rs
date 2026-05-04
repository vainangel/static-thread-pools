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

// NOTE: Worker task scheduling:
// 1. Receive a task
// 2. If the total duration of the task exceeds the quanta, burst (halt the thread) for quanta ms (i
//    can just loop until the duration has elapsed)
// 3. If the task is not done (i.e., the total duration has not elapsed), update the work done
//    duration and send the task back to the scheduler.
// 4. If the task is done, terminate. 

// TODO: Full-duplex IO between worker and pool
// 1. Receive tasks from pool
// 2. Send unfinished tasks back to pool 

// TODO: Rename DynamicWorker to RoundRobinWorker? Not really necessary.

#[derive(Clone)]
struct TaskMediator {
    quanta: Duration,
    load: Arc<Mutex<usize>>,
    rx: Arc<Mutex<mpsc::Receiver<Option<Task>>>>,
    rq_tx: mpsc::Sender<Option<Task>>,
}

impl TaskMediator {
    pub fn get_pending_task(&self) -> Option<TaskHandle> {
        let task = self.rx.lock().unwrap().recv().unwrap();

        Some(TaskHandle::new(task?, self.clone())) 
    }

    pub fn return_task(&self, task: Task) {
        self.rq_tx.send(Some(task.clone())).unwrap();
    }

    pub fn update_load(&self, task: Task) {
        *self.load.lock().unwrap() -= task.kind.get_load();
    }

    pub fn quanta(&self) -> Duration { self.quanta }
}

struct TaskHandle {
    task: Task,
    mediator: TaskMediator,
}

impl TaskHandle {
    pub fn new(task: Task, mediator: TaskMediator) -> Self {
        TaskHandle{ task, mediator }
    }

    pub fn do_work(&mut self) -> bool {
        let is_last_burst = self.task.duration <= self.mediator.quanta(); 

        if !is_last_burst {
            thread::sleep(self.mediator.quanta());
            self.task.duration -= self.mediator.quanta();
            self.mediator.return_task(self.task.clone());
        } else {
            thread::sleep(self.task.duration);
        }

        self.mediator.update_load(self.task.clone());
        is_last_burst
    }
}

pub struct DynamicWorker {
    id: usize,
    pub thread: Option<thread::JoinHandle<()>>,
}

impl DynamicWorker {
    pub fn get_id(&self) -> usize { self.id }

    // TODO: replace load mutex with AtomicUsize ? 
    pub fn new(id: usize, mediator: TaskMediator) -> Self {

        let listen_for_tasks = move || loop {
            let task_handle = mediator.get_pending_task(); 

            match task_handle {
                Some(mut task_handle) => {
                    // NOTE: `task.duration` acts as a stand-in for job state, which would typically
                    // be used to describe whether a task has been completed, is pending, or is
                    // preemptively terminated. Since we do not have such state, but instead use
                    // duration to describe how long a task "works" for, we simply use the duration
                    // to tell us how much "work" needs to be done.

                    let task_is_done = task_handle.do_work();
                    if task_is_done {
                        println!("Finished {:?} task {:?} in {:.3?}s (About {:?}s after arrival time)",
                            task_handle.task.kind, task_handle.task.id,
                            SystemTime::now().duration_since(task_handle.task.time_queued).unwrap_or(Duration::ZERO).as_secs_f32(),
                            SystemTime::now().duration_since(task_handle.task.arrival_time).unwrap_or(Duration::ZERO).as_secs_f32()
                        );
                    }
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
    rq_tx: mpsc::Sender<Option<Task>>,
    pub load: Arc<Mutex<usize>>,
    start: SystemTime,
    task_queue: Arc<Mutex<PriorityQueue<Task, TaskKind>>>,
}

impl DynamicWorkerPool {
    pub fn new(n_workers: usize) -> Self {
        println!("Building dynamic worker pool.");

        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        let task_queue = Arc::new(Mutex::new(PriorityQueue::new()));

        let (rq_tx, rq_rx) = mpsc::channel();
        let queue = task_queue.clone();

        thread::spawn(move || loop {
            let returned_task: Option<Task> = rq_rx.recv().unwrap();

            if let Some(returned_task) = returned_task {
                println!("Unfinished task {:?} returned to task pool.", returned_task.id);
                queue.lock().unwrap().push(returned_task.clone(), returned_task.kind);
            } else {
                break;
            }
        });

        let load = Arc::new(Mutex::new(0 as usize));

        const QUANTA: Duration = Duration::from_millis(50);

        let mut workers = Vec::with_capacity(n_workers);
        for id in 1..=workers.capacity() {
            let mediator = TaskMediator{ quanta: QUANTA, load: load.clone(), rx: rx.clone(), 
                rq_tx: rq_tx.clone() };
            workers.push(DynamicWorker::new(id, mediator));
        }

        let start = SystemTime::now();

        DynamicWorkerPool{ workers, tx, rq_tx, load, start, task_queue }
    }

    pub fn execute_task(&mut self, task: Task) {
        self.task_queue.lock().unwrap().push(task.clone(), task.kind);

        let (task, kind) = self.task_queue.lock().unwrap().pop().unwrap();
        while *self.load.lock().unwrap() + kind.get_load() > 100 {
            // ...
        }

        *self.load.lock().unwrap() += kind.get_load();
        println!("Load: {:?}", self.load.lock().unwrap().clone());
        self.tx.send(Some(task)).unwrap();
    }

    pub fn await_remaining_tasks(&mut self) {
        println!("Finishing remaining tasks...");

        loop {
            // NOTE: anti-pattern, should use messages between threads to signal completion of work
            if self.task_queue.lock().unwrap().is_empty() && *self.load.lock().unwrap() == 0 {
                break; 
            }

            while let Some((task, kind)) = self.task_queue.lock().unwrap().pop() {
                while *self.load.lock().unwrap() + kind.get_load() > 100 {
                    // ...
                }

                *self.load.lock().unwrap() += kind.get_load();
                self.tx.send(Some(task)).unwrap();
            }
        }
        self.rq_tx.send(None).unwrap();

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
