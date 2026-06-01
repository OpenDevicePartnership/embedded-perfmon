#![no_std]

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event<'a> {
    pub timestamp: u64,
    #[serde(borrow)]
    pub kind: EventKind<'a>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EventKind<'a> {
    Global(GlobalEvent),
    #[serde(borrow)]
    Executor(ExecutorEvent<'a>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GlobalEvent {
    /// The timestamp tickrate per second. Should appear at least once in every trace.
    /// When emitted multiple times, only one of them is used.
    ///
    /// This is a global value and is not bound to a particular executor
    TickRate { rate: u64 },
    /// An interrupt has started
    IrqStart { irq: u16 },
    /// An interrupt has stopped
    IrqEnd { irq: u16 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutorEvent<'a> {
    pub executor_id: u32,
    #[serde(borrow)]
    pub kind: ExecutorEventKind<'a>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExecutorEventKind<'a> {
    /// Executor exited the idle state
    ExecutorPollStart,
    /// Executor entered the idle state
    ExecutorIdle,
    /// A new task is created
    TaskNew { task_id: u32 },
    /// A task has been stopped
    TaskEnd { task_id: u32 },
    /// A task started being executed
    TaskExecBegin { task_id: u32 },
    /// A task finished execution
    TaskExecEnd { task_id: u32 },
    /// A task is now ready to be executed
    TaskReadyBegin { task_id: u32 },
    /// A task got assigned a human readable name
    TaskNamed { task_id: u32, name: &'a str },
}

impl<'a> Event<'a> {
    pub fn serialize<'buf>(&self, buf: &'buf mut [u8]) -> Result<&'buf mut [u8], postcard::Error> {
        postcard::to_slice_cobs(self, buf)
    }

    pub fn deserialize(buf: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), postcard::Error> {
        postcard::take_from_bytes_cobs(buf)
    }
}
