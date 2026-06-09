#![no_std]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event<'a> {
    pub timestamp: u64,
    #[serde(borrow)]
    pub kind: EventKind<'a>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventKind<'a> {
    Global(GlobalEvent<'a>),
    Executor(ExecutorEvent),
    #[serde(borrow)]
    Task(TaskEvent<'a>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GlobalEvent<'a> {
    /// The timestamp tickrate per second. Should appear at least once in every trace.
    /// When emitted multiple times, only one of them is used.
    ///
    /// This is a global value and is not bound to a particular executor
    TickRate { rate: u64 },
    /// An interrupt has started
    IrqStart { irq: u16 },
    /// An interrupt has stopped
    IrqEnd { irq: u16 },
    /// The user emitted a custom marker
    Marker {
        #[serde(borrow)]
        name: &'a str,
    },
    /// The user started a custom span
    SpanStart {
        #[serde(borrow)]
        name: &'a str,
        id: u32,
    },
    /// The user ended the custom span with the specified id
    SpanEnd { id: u32 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutorEvent {
    pub executor_id: u32,
    pub kind: ExecutorEventKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExecutorEventKind {
    /// Executor exited the idle state
    ExecutorPollStart,
    /// Executor entered the idle state
    ExecutorIdle,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEvent<'a> {
    pub task_id: u32,
    #[serde(borrow)]
    pub kind: TaskEventKind<'a>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskEventKind<'a> {
    /// A new task is created
    TaskNew { executor_id: u32 },
    /// A task has been stopped
    TaskEnd,
    /// A task started being executed
    TaskExecBegin,
    /// A task finished execution
    TaskExecEnd,
    /// A task is now ready to be executed
    TaskReadyBegin,
    /// A task got assigned a human readable name
    TaskNamed { name: &'a str },
    /// A task got its priority changed
    PrioritySet { priority: u8 },
    /// A task got its deadline changed
    DeadlineSet { deadline: u64 },
    /// The user emitted a custom marker
    Marker {
        #[serde(borrow)]
        name: &'a str,
    },
    /// The user started a custom span
    SpanStart {
        #[serde(borrow)]
        name: &'a str,
        id: u32,
    },
    /// The user ended the custom span with the specified id
    SpanEnd { id: u32 },
}

impl<'a> Event<'a> {
    pub fn serialize<'buf>(&self, buf: &'buf mut [u8]) -> Result<&'buf mut [u8], postcard::Error> {
        postcard::to_slice_cobs(self, buf)
    }

    pub fn deserialize(buf: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), postcard::Error> {
        postcard::take_from_bytes_cobs(buf)
    }
}
