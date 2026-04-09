
/// Response segmenter that splits LLM output by <stop/> markers
pub struct ResponseSegmenter {
    stop_marker: String,
}

impl ResponseSegmenter {
    pub fn new() -> Self {
        Self {
            stop_marker: "<stop/>".to_string(),
        }
    }

    /// Split text by <stop/> markers, returning segments
    pub fn segment(&self, text: &str) -> Vec<String> {
        text.split(&self.stop_marker)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Check if text contains a stop marker
    pub fn has_stop(&self, text: &str) -> bool {
        text.contains(&self.stop_marker)
    }

    /// Extract the next segment from text (up to first stop marker)
    pub fn extract_next(&self, text: &str) -> Option<(String, String)> {
        if let Some(pos) = text.find(&self.stop_marker) {
            let segment = text[..pos].trim().to_string();
            let remaining = text[pos + self.stop_marker.len()..].to_string();
            if !segment.is_empty() {
                return Some((segment, remaining));
            }
        }
        None
    }

    /// Create a prompt that instructs the LLM to use stop markers
    pub fn system_prompt() -> &'static str {
        r#"你是语音助手。

要求：
1. 输出适合语音播放
2. 每句话后加 <stop/>
3. 尽量简短
4. 如需调用工具，输出JSON

格式：
普通回答：
文本 + <stop/>

工具调用：
{
  "tool": "",
  "params": {}
}"#
    }
}

impl Default for ResponseSegmenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment() {
        let seg = ResponseSegmenter::new();
        let text = "你好，今天天气不错。<stop/>适合出门散步。<stop/>";
        let segments = seg.segment(text);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], "你好，今天天气不错。");
        assert_eq!(segments[1], "适合出门散步。");
    }

    #[test]
    fn test_extract_next() {
        let seg = ResponseSegmenter::new();
        let text = "你好<stop/>再见";
        let (segment, remaining) = seg.extract_next(text).unwrap();
        assert_eq!(segment, "你好");
        assert_eq!(remaining.trim(), "再见");
    }
}
