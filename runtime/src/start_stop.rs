use core::cell::RefCell;

use critical_section::Mutex;
use embedded_perfmon_transport::{Event, EventKind, GlobalEvent, TaskEvent, TaskEventKind};
use heapless::index_map::FnvIndexMap;

use crate::get_trace_event_timestamp;

const TASK_COUNT: usize = {
    let input = if let Some(string) = option_env!("PERFMON_TASK_COUNT") {
        string
    } else {
        "16"
    };
    match usize::from_str_radix(input, 10) {
        Ok(val @ 2..) => val,
        Ok(_) => panic!("PERFMON_TASK_COUNT must be 2 or higher"),
        Err(_e) => panic!("Could not parse `PERFMON_TASK_COUNT`"),
    }
};

static STATE: Mutex<RefCell<State>> = Mutex::new(RefCell::new(State::new()));

/// Start the tracing.
///
/// This will emit the cached data so it's present in the trace.
/// It will also ungate the writing of trace events.
pub fn start_tracing() {
    critical_section::with(|cs| {
        let mut state = STATE.borrow_ref_mut(cs);

        state.started = true;
        state.send_data();
    });
}

/// Stop the tracing.
///
/// This will gate the writing of trace events.
pub fn stop_tracing() {
    critical_section::with(|cs| {
        let mut state = STATE.borrow_ref_mut(cs);
        state.started = false;
    });
}

pub(crate) fn write_trace_event(event: Event<'static>) {
    critical_section::with(|cs| {
        let mut state = STATE.borrow_ref_mut(cs);

        match &event.kind {
            EventKind::Global(GlobalEvent::TickRate { rate }) => state.tickrate = Some(*rate),
            EventKind::Task(TaskEvent {
                task_id,
                kind: TaskEventKind::TaskNamed { name },
            }) => {
                let Ok(task_data) = state.task_data.entry(*task_id).or_default() else {
                    panic!("task cache full");
                };

                task_data.name = Some(*name);
            }
            EventKind::Task(TaskEvent {
                task_id,
                kind: TaskEventKind::TaskNew { executor_id },
            }) => {
                let Ok(task_data) = state.task_data.entry(*task_id).or_default() else {
                    panic!("task cache full");
                };

                task_data.executor_id = Some(*executor_id);
            }
            EventKind::Task(TaskEvent {
                task_id,
                kind:
                    kind @ (TaskEventKind::TaskEnd
                    | TaskEventKind::TaskExecBegin
                    | TaskEventKind::TaskExecEnd
                    | TaskEventKind::TaskReadyBegin),
            }) => {
                let Ok(task_data) = state.task_data.entry(*task_id).or_default() else {
                    panic!("task cache full");
                };

                task_data.last_state_event = Some(kind.clone());
            }
            #[cfg(feature = "priority")]
            EventKind::Task(TaskEvent {
                task_id,
                kind: TaskEventKind::PrioritySet { priority },
            }) => {
                let Ok(task_data) = state.task_data.entry(*task_id).or_default() else {
                    panic!("task cache full");
                };

                task_data.priority = Some(*priority);
            }
            #[cfg(feature = "deadline")]
            EventKind::Task(TaskEvent {
                task_id,
                kind: TaskEventKind::DeadlineSet { deadline },
            }) => {
                let Ok(task_data) = state.task_data.entry(*task_id).or_default() else {
                    panic!("task cache full");
                };

                task_data.deadline = Some(*deadline);
            }
            _ => {}
        }

        if state.started {
            crate::external::_write_trace_event(event);
        }
    });
}

struct State {
    started: bool,
    tickrate: Option<u64>,
    task_data: FnvIndexMap<u32, TaskData, TASK_COUNT>,
}

impl State {
    const fn new() -> Self {
        Self {
            started: false,
            tickrate: None,
            task_data: FnvIndexMap::new(),
        }
    }

    fn send_data(&self) {
        let timestamp = get_trace_event_timestamp();

        if let Some(rate) = self.tickrate {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Global(GlobalEvent::TickRate { rate }),
            });
        }

        for (task_id, task_data) in &self.task_data {
            task_data.send_data(*task_id, timestamp);
        }
    }
}

#[derive(Default)]
struct TaskData {
    name: Option<&'static str>,
    executor_id: Option<u32>,
    last_state_event: Option<TaskEventKind<'static>>,
    #[cfg(feature = "priority")]
    priority: Option<u8>,
    #[cfg(feature = "deadline")]
    deadline: Option<u64>,
}

impl TaskData {
    fn send_data(&self, task_id: u32, timestamp: u64) {
        if let Some(name) = self.name {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Task(TaskEvent {
                    task_id,
                    kind: TaskEventKind::TaskNamed { name },
                }),
            });
        }

        if let Some(executor_id) = self.executor_id {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Task(TaskEvent {
                    task_id,
                    kind: TaskEventKind::TaskNew { executor_id },
                }),
            });
        }

        if let Some(last_state_event) = self.last_state_event.as_ref() {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Task(TaskEvent {
                    task_id,
                    kind: last_state_event.clone(),
                }),
            });
        }

        #[cfg(feature = "priority")]
        if let Some(priority) = self.priority {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Task(TaskEvent {
                    task_id,
                    kind: TaskEventKind::PrioritySet { priority },
                }),
            });
        }

        #[cfg(feature = "deadline")]
        if let Some(deadline) = self.deadline {
            crate::external::_write_trace_event(Event {
                timestamp,
                kind: EventKind::Task(TaskEvent {
                    task_id,
                    kind: TaskEventKind::DeadlineSet { deadline },
                }),
            });
        }
    }
}
