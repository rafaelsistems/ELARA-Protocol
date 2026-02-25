# elara-msp

Minimal Survival Profile (MSP) for the ELARA Protocol - providing text streams, voice processing, and degradation management for real-time communication applications.

## Features

- **Text Streams**: Real-time text messaging with typing indicators
- **Voice Processing**: Parametric voice encoding and synthesis
- **Degradation Management**: Graceful quality scaling under network stress
- **Feed Processing**: Stream-based message handling
- **Cross-Platform**: Works on any device from $30 phones to servers

## Quick Start

```rust
use elara_msp::text::{TextStream, TextMessage};
use elara_msp::voice::{VoiceStream, VoiceFrame};

// Create text stream
let mut text_stream = TextStream::new(stream_id);

// Create voice stream
let mut voice_stream = VoiceStream::new(stream_id);

// Process text message
let message = TextMessage::new(
    author_id,
    "Hello from ELARA!",
    timestamp
);
text_stream.append(message)?;

// Process voice frame
let voice_frame = VoiceFrame::from_audio(audio_data)?;
voice_stream.append(voice_frame)?;
```

## Text Processing

### Text Streams
```rust
pub struct TextStream {
    pub id: StreamId,
    pub messages: Vec<TextMessage>,
    pub typing_indicators: HashMap<NodeId, TypingIndicator>,
}
```

### Message Types
- **TextMessage**: Plain text with metadata
- **TypingIndicator**: Real-time typing status
- **FeedItem**: Stream-based message container
- **StreamMarker**: Conversation boundaries

### Features
- Real-time message delivery
- Typing indicators with expiration
- Message threading support
- Unicode and emoji support
- Automatic text normalization

## Voice Processing

### Voice Streams
```rust
pub struct VoiceStream {
    pub id: StreamId,
    pub frames: Vec<VoiceFrame>,
    pub state: VoiceStreamState,
    pub degradation: DegradationLevel,
}
```

### Voice Frames
- **Voiced Frames**: Active speech with full parameters
- **Silent Frames**: Background noise modeling
- **Transition Frames**: Speech/silence boundaries

### Degradation Levels
```
Full Quality → Reduced Quality → Essential → Symbolic → Presence
     ↓              ↓              ↓           ↓         ↓
  All Params    Core Params     Basic       Text     Existence
  Preserved     Preserved       Features    Only     Proof
```

## Degradation Management

### Automatic Adaptation
```rust
// Network quality detection
let quality = network_monitor.quality();

// Automatic degradation
stream.adapt_to_network(quality);

// Manual quality setting
stream.set_quality_level(QualityLevel::Medium);
```

### Cross-Platform Support
- **Tier 0**: $30 phones - Text only
- **Tier 1**: Basic smartphones - Text + compressed voice
- **Tier 2**: Modern devices - Full features
- **Tier 3**: High-end devices - Maximum quality

## Integration

### With ELARA Core
```rust
use elara_core::{Event, EventType};

// Convert to ELARA event
let event = text_stream.to_event()?;

// Process through ELARA pipeline
node.process_event(event)?;
```

### Standalone Usage
```rust
// Use without full ELARA stack
let mut stream = TextStream::standalone();
stream.append_message("Hello World")?;
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.