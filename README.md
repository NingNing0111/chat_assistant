# Chat Assistant

A voice-interactive AI assistant built with Rust, featuring wake word detection, streaming ASR, LLM-powered conversations, and real-time TTS synthesis.

## Features

- **Wake Word Detection** - Custom wake word activation using sherpa-onnx
- **Streaming ASR** - Real-time speech recognition with VAD (Voice Activity Detection)
- **LLM Integration** - Multi-turn conversations via rig framework, supports OpenAI-compatible APIs
- **Real-time TTS** - Streaming synthesis with Kokoro, segmented playback via `<stop/>` markers
- **Tool System** - Built-in tools (time, weather, calculator, reminders) + MCP tool loading
- **Memory System** - Short-term session memory + long-term persistent storage
- **Barge-in Support** - Interrupt ongoing playback with new voice input

## Tech Stack

| Component | Technology |
|-----------|------------|
| LLM/Agent | [rig](https://github.com/0xPlaygrounds/rig) |
| ASR/TTS/WakeWord | [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) |
| Audio Capture | [cpal](https://github.com/rustaudio/cpal) |
| Audio Playback | [rodio](https://github.com/RustAudio/rodio) |

## Prerequisites

- Rust 1.70+
- Git LFS (for model files)
- Audio input/output device

## Quick Start

1. **Clone and build**
   ```bash
   git clone https://github.com/NingNing0111/chat_assistant.git
   cd chat_assistant
   cargo build --release
   ```

2. **Configure**
   ```bash
   cp .env.example .env  # Edit with your API keys
   ```

   Edit `.env`:
   ```env
   LLM_PROVIDER=openai
   LLM_MODEL=your-model-name
   OPENAI_API_KEY=your-api-key
   LLM_BASE_URL=https://api.openai.com/v1  # or your custom endpoint
   WAKE_WORD=玉米糊
   ```

3. **First run** - Models download automatically on first launch
   ```bash
   cargo run --release
   ```

## Project Structure

```
chat_assistant/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # State machine orchestration
│   ├── config.rs            # Configuration loading
│   ├── audio/
│   │   ├── capture.rs       # Microphone capture (cpal)
│   │   └── player.rs        # Audio playback queue (rodio)
│   ├── asr/
│   │   └── recognizer.rs    # Streaming ASR (sherpa-onnx)
│   ├── wakeword/
│   │   └── detector.rs      # Wake word detection
│   ├── tts/
│   │   └── synthesizer.rs   # TTS synthesis (Kokoro)
│   ├── llm/
│   │   ├── agent.rs         # LLM agent wrapper
│   │   └── response.rs      # <stop/> segment parsing
│   ├── session/
│   │   └── manager.rs       # Multi-turn conversation context
│   ├── memory/
│   │   └── store.rs         # Short-term + long-term memory
│   ├── tools/
│   │   ├── builtin.rs       # Built-in tools
│   │   └── mcp.rs           # MCP JSON config loader
│   └── model_manager.rs     # Auto-download models
├── examples/
│   ├── tts_example.rs       # Test TTS synthesis
│   ├── keyword_spotter_example.rs  # Test wake word
│   └── audio_playback.rs    # Test audio playback
├── config/
│   └── mcp_config.json      # MCP tools configuration
└── .env                     # Environment variables
```

## Usage Modes

### Wake Word Mode (Default)
1. Say the wake word (e.g., "玉米糊")
2. Speak your query
3. Assistant responds with synthesized speech

### Continuous Mode
```env
CONTINUOUS_MODE=true
```
- No wake word needed
- Always listening after startup

## Tools

### Built-in Tools
- `get_time` - Current time
- `get_weather` - Weather query
- `calculate` - Math expressions
- `set_reminder` - Set reminders

### MCP Tools
Add tools via JSON config:
```json
{
  "tools": [
    {
      "name": "custom_tool",
      "description": "Custom tool description",
      "params": {}
    }
  ]
}
```

## TTS Segmentation

The assistant uses `<stop/>` markers to segment responses for streaming TTS:

```
Sure, I can help you with that.<stop/>The weather today is sunny.<stop/>
```

Each segment is synthesized and played immediately for low-latency responses.

## Examples

Test individual components:

```bash
# Test TTS synthesis
cargo run --example tts_example --release

# Test wake word detection
cargo run --example keyword_spotter_example --release

# Test audio playback
cargo run --example audio_playback --release
```

## License

MIT
