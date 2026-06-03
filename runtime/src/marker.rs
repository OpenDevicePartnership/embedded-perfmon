use core::task::Poll;

use embedded_perfmon_transport::{Event, EventKind, GlobalEvent, TaskEvent, TaskEventKind};

use crate::{_get_trace_event_timestamp, _write_trace_event};

pub fn emit_global_marker(name: &str) {
    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Global(GlobalEvent::Marker { name }),
    });
}

pub async fn emit_task_marker(name: &str) {
    let task_ref =
        core::future::poll_fn(|cx| Poll::Ready(embassy_executor::raw::task_from_waker(cx.waker())))
            .await;

    _write_trace_event(Event {
        timestamp: _get_trace_event_timestamp(),
        kind: EventKind::Task(TaskEvent {
            task_id: task_ref.id(),
            kind: TaskEventKind::Marker { name },
        }),
    });
}
