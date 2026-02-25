# elara-voice

Voice processing engine for the ELARA Protocol - featuring parametric encoding, synthesis, and graceful degradation for real-time voice communication.

## Features

- **Parametric Encoding**: Voice parameters instead of raw audio samples
- **Voice Activity Detection**: Intelligent speech/silence classification
- **Packet Loss Concealment**: Smooth degradation under network stress
- **Speech Synthesis**: Text-to-speech with emotional modeling
- **Degradation Control**: Quality scaling based on network conditions

## Quick Start

```rust
use elara_voice::{VoiceEncoder, VoiceDecoder, VoiceState};
use elara_core::StateTime;

// Create voice encoder
let mut encoder = VoiceEncoder::new(config);

// Create voice decoder  
let mut decoder = VoiceDecoder::new(config);

// Encode voice frame
let voice_state = encoder.encode_frame(audio_data)?;

// Decode to audio
let audio_data = decoder.decode_state(voice_state)?;
```

## Voice State Model

### Parametric Representation
```rust
pub struct VoiceState {
    pub activity: VoiceActivity,     // Speech/silence classification
    pub parameters: VoiceParams,   // Pitch, energy, spectral features
    pub emotion: EmotionParams,    // Emotional characteristics
    pub degradation: DegradationLevel, // Quality level
    pub timestamp: StateTime,        // Temporal reference
}
```

### Voice Parameters
```rust
pub struct VoiceParams {
    pub pitch: f32,              // Fundamental frequency
    pub energy: f32,             // Signal energy
    pub spectral_centroid: f32, // Spectral center of mass
    pub spectral_rolloff: f32,   // Spectral rolloff point
    pub mfcc: [f32; 13],         // Mel-frequency cepstral coefficients
}
```

## Encoding Process

### Voice Activity Detection
```rust
// Classify speech vs silence
let activity = encoder.detect_activity(audio_frame)?;

// Handle different activity types
match activity {
    VoiceActivity::Speech => encoder.encode_speech(audio_frame),
    VoiceActivity::Silence => encoder.encode_silence(),
    VoiceActivity::Transition => encoder.encode_transition(audio_frame),
}
```

### Degradation Levels
```
Full Quality → Reduced Quality → Essential → Symbolic → Presence
     ↓              ↓              ↓           ↓         ↓
  All Params    Core Params     Basic       Text     Existence
  Preserved     Preserved       Features    Only     Proof
```

## Synthesis Features

### Text-to-Speech
```rust
// Synthesize speech from text
let voice_state = synthesizer.synthesize(
    "Hello, ELARA!",
    Emotion::Neutral,
    VoiceStyle::Conversational
)?;
```

### Emotion Modeling
```rust
// Apply emotional characteristics
let emotional_state = encoder.apply_emotion(
    base_state,
    EmotionParams {
        arousal: 0.7,      // Energy level
        valence: 0.5,      // Positivity
        dominance: 0.6,    // Control level
    }
)?;
```

## Network Adaptation

### Quality Scaling
```rust
// Adapt to network conditions
encoder.adapt_to_network(network_quality);

// Manual quality setting
encoder.set_quality_level(QualityLevel::Medium);

// Automatic degradation
encoder.enable_auto_degradation(true);
```

### Packet Loss Concealment
```rust
// Handle missing packets gracefully
if packet_lost {
    let concealed = decoder.conceal_loss(last_state, context)?;
    decoder.decode_state(concealed)?;
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.