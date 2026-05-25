use std::time::Duration;

/// SSE 事件缓冲区
#[derive(Default)]
pub(crate) struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

impl SseEvent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.event_type.is_none() && self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.event_type = None;
        self.data.clear();
        self.id = None;
    }
}

pub(crate) enum SseParseResult {
    Next,
    Dispatch,
    SetRetry(Duration),
}

pub(crate) fn sse_parse_line(line: &str, event: &mut SseEvent) -> SseParseResult {
    let line = line.trim_end_matches(['\r', '\n']);

    if line.is_empty() {
        if event.is_empty() {
            return SseParseResult::Next;
        }
        return SseParseResult::Dispatch;
    }

    if line.starts_with(':') {
        return SseParseResult::Next;
    }

    let (field, value) = if let Some(colon_pos) = line.find(':') {
        let field = &line[..colon_pos];
        let value = line[colon_pos + 1..].strip_prefix(' ').unwrap_or(&line[colon_pos + 1..]);
        (field, value)
    } else {
        (line, "")
    };

    match field {
        "event" => event.event_type = Some(value.to_string()),
        "data" => {
            if !event.data.is_empty() {
                event.data.push('\n');
            }
            event.data.push_str(value);
        }
        "id" => event.id = Some(value.to_string()),
        "retry" => {
            if let Ok(ms) = value.parse::<u64>() {
                return SseParseResult::SetRetry(Duration::from_millis(ms));
            }
        }
        _ => {}
    }

    SseParseResult::Next
}

/// SSE 事件中的文件变更条目
#[derive(Debug, serde::Deserialize)]
pub(crate) struct SseFileEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub file_id: String,
    pub from: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub to: String,
}
