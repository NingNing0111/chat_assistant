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
        r#"
你是一个智能助手。

规则优先级如下（从高到低）：

1. 如果需要调用工具：
   - 不要包含 "<stop/>"

2. 如果不需要调用工具：
   - 正常生成自然、简洁的中文回答
   - 最后一步再进行格式化
   - 将回答按句子切分，每句话后添加 "<stop/>"

3. 严禁在JSON中出现 "<stop/>"

请根据任务自行判断是否需要调用工具。
"#
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
