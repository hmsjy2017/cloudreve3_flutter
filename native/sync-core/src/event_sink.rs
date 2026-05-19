use crate::api::ffi_types::SyncEventFfi;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Rust→Dart 事件推送 — 封装 FRB StreamSink
///
/// **重要**: StreamSink.add() 需要 SyncEventFfi 实现 SseEncode trait，
/// 该 trait 由 flutter_rust_bridge_codegen 自动生成。
/// 在运行 codegen 之前，emit() 仅写日志不实际推送。
/// 运行 codegen 后，StreamSink 可用，emit() 将实际推送到 Dart。
pub struct EventSink {
    sink: Arc<Mutex<Option<crate::frb_generated::StreamSink<SyncEventFfi>>>>,
    available: std::sync::atomic::AtomicBool,
}

impl Default for EventSink {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSink {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
            available: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 注册 StreamSink
    pub async fn register(&self, sink: crate::frb_generated::StreamSink<SyncEventFfi>) {
        self.available.store(true, std::sync::atomic::Ordering::Relaxed);
        *self.sink.lock().await = Some(sink);
    }

    /// 推送事件到 Dart
    pub async fn emit(&self, event: SyncEventFfi) {
        if self.available.load(std::sync::atomic::Ordering::Relaxed) {
            self.emit_inner(event).await;
        }
    }

    /// 实际推送 — 仅在 StreamSink 可用时调用
    /// 此方法在 FRB codegen 生成 SseEncode 实现后才编译通过
    #[cfg(feature = "event_sink_enabled")]
    async fn emit_inner(&self, event: SyncEventFfi) {
        if let Some(sink) = self.sink.lock().await.as_ref() {
            let _ = sink.add(event);
        }
    }

    #[cfg(not(feature = "event_sink_enabled"))]
    async fn emit_inner(&self, _event: SyncEventFfi) {
        // codegen 生成前为空操作，仅写日志
        tracing::debug!("EventSink: 事件未推送（FRB codegen 未运行）");
    }
}
