use core::task::Poll;

use embedded_perfmon_transport::{Event, EventKind, GlobalEvent, TaskEvent, TaskEventKind};

use crate::{get_trace_event_timestamp, write_trace_event};

pub fn emit_global_marker(name: &'static str) {
    write_trace_event(Event {
        timestamp: get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::Marker { name }),
    });
}

pub async fn emit_task_marker(name: &'static str) {
    let task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    write_trace_event(Event {
        timestamp: get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: task_ref.id().get() as u32,
            kind: TaskEventKind::Marker { name },
        }),
    });
}
