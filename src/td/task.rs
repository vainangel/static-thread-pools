use std::time::{Duration, SystemTime};

pub type TaskId = i32;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TaskKind {
    CPU = 2,
    IO = 1, 
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Task {
    pub id: TaskId,
    pub arrival_time: SystemTime,
    pub kind: TaskKind,
    pub duration: Duration, 
    pub time_queued: SystemTime,
}



