# Representation Profiles

Representation Profiles are **configurations** that adapt ELARA's universal laws to specific communication modalities without changing the underlying protocol.

## Philosophy

> "One protocol, many faces."

A Representation Profile defines:
- Which state atoms are used
- How deltas are encoded
- What timing constraints apply
- How degradation occurs

## Profile Registry

| Profile ID | Name | Primary Use |
|------------|------|-------------|
| 0x00 | Raw | Direct state access |
| 0x01 | Textual | Chat, messages |
| 0x02 | VoiceMinimal | Voice calls |
| 0x03 | VoiceRich | HD voice |
| 0x04 | VideoPerceptual | Video calls |
| 0x05 | GroupSwarm | Group communication |
| 0x06 | LivestreamAsymmetric | Broadcasting |
| 0x07 | Agent | AI/Bot integration |

## Profile: Textual (0x01)

For chat, system messages, presence, and typing indicators.

### State Atoms

```rust
// Message stream
ω:text:stream_id = {
    type: Core,
    delta_law: AppendOnly,
    authority: stream_participants
}

// User presence
ω:presence:user_id = {
    type: Perceptual,
    delta_law: LastWriteWins,
    authority: user_only
}

// Typing indicator
ω:typing:user_id = {
    type: Cosmetic,
    delta_law: Ephemeral(ttl: 5s),
    authority: user_only
}
```

### Delta Encoding

```rust
TextDelta = {
    // Append operation
    Append { 
        content: String,
        format: Option<Format>
    },
    // Edit operation (for editable messages)
    Edit {
        message_id: MessageId,
        range: Range<usize>,
        replacement: String
    },
    // React operation
    React {
        message_id: MessageId,
        reaction: Reaction
    }
}
```

### Timing

- **Hp**: 100-500ms (typing prediction)
- **Hc**: 5-30s (edit window)
- Delivery: Best-effort with ordering guarantee

## Profile: VoiceMinimal (0x02)

For voice calls on constrained devices/networks.

### State Atoms

```rust
// Voice state per user
ω:voice:user_id = {
    type: Perceptual,
    delta_law: FrameBased(interval: 20ms),
    authority: user_only
}
```

### Delta Encoding

**NOT audio PCM!** Voice is encoded as **speech state**:

```rust
VoiceDelta = {
    // Frame timing
    frame_seq: u16,
    timestamp: u32,
    
    // Speech parameters (NOT audio samples)
    voiced: bool,           // Voiced or unvoiced
    pitch: u8,              // Fundamental frequency index
    energy: u8,             // Volume/energy level
    spectral_env: [u8; 10], // Spectral envelope (LPC or similar)
    residual_seed: u16,     // For noise regeneration
    
    // Optional enhancements
    formants: Option<[u8; 4]>,
    prosody: Option<ProsodyHint>
}
```

### Timing

- **Hp**: 40-100ms
- **Hc**: 80-200ms
- Frame interval: 10-20ms
- Jitter buffer: Adaptive

### Degradation Path

```
Full Voice Parameters
       │
       ▼ (pressure)
Reduced Parameters (pitch + energy only)
       │
       ▼ (more pressure)
Symbolic (speaking/silent indicator)
       │
       ▼ (critical)
Presence Only
```

## Profile: VideoPerceptual (0x04)

For video calls with perceptual optimization.

### State Atoms

```rust
// Video state per user
ω:video:user_id = {
    type: Perceptual,
    delta_law: KeyframePlusDelta,
    authority: user_only
}

// Video quality hints
ω:video_quality:user_id = {
    type: Enhancement,
    delta_law: LastWriteWins,
    authority: user_only
}
```

### Delta Encoding

```rust
VideoDelta = {
    // Frame type
    frame_type: KeyFrame | PFrame | BFrame,
    
    // Perceptual state (NOT raw pixels)
    face_region: Option<FaceState>,
    background_hash: u64,
    motion_vectors: Vec<MotionVector>,
    
    // Compressed visual data
    visual_data: CompressedRegions
}

FaceState = {
    position: (f32, f32),
    size: f32,
    expression_params: [f32; 16],
    gaze_direction: (f32, f32)
}
```

### Timing

- **Hp**: 50-150ms
- **Hc**: 100-500ms
- Keyframe interval: 1-5s
- Target framerate: 15-30fps

## Profile: GroupSwarm (0x05)

For group communication with swarm dynamics.

### State Atoms

```rust
// Group membership
ω:group:group_id = {
    type: Core,
    delta_law: SetCRDT,
    authority: group_admins
}

// Per-member state
ω:member:group_id:user_id = {
    type: Perceptual,
    delta_law: Composite,
    authority: user_only
}

// Shared group state
ω:shared:group_id:state_name = {
    type: varies,
    delta_law: varies,
    authority: group_participants
}
```

### Swarm Dynamics

```rust
SwarmConfig = {
    // Topology
    max_direct_peers: 8,
    relay_threshold: 16,
    
    // Diffusion
    gossip_fanout: 3,
    priority_routing: true,
    
    // Load balancing
    speaker_priority: true,
    active_speaker_boost: 2.0
}
```

## Profile: Agent (0x07)

For AI agents and bots.

### State Atoms

```rust
// Agent identity
ω:agent:agent_id = {
    type: Core,
    delta_law: Immutable,
    authority: agent_owner
}

// Agent state
ω:agent_state:agent_id = {
    type: Core,
    delta_law: EventSourced,
    authority: agent_only
}

// Agent outputs
ω:agent_output:agent_id:stream_id = {
    type: varies,
    delta_law: AppendOnly,
    authority: agent_only
}
```

### Agent Constraints

```rust
AgentConstraints = {
    // Rate limiting
    max_events_per_second: 10,
    max_state_size: 1MB,
    
    // Authority bounds
    can_create_state: bool,
    can_modify_others: bool,
    
    // Identification
    must_identify_as_agent: true
}
```

## Creating Custom Profiles

Profiles can be extended for specific applications:

```rust
CustomProfile = {
    base: ProfileId,  // Inherit from existing
    
    // Override specific aspects
    state_atoms: Vec<StateAtomDef>,
    delta_laws: Map<StatePattern, DeltaLaw>,
    timing: TimingConfig,
    degradation: DegradationPath,
    
    // Profile metadata
    id: ProfileId,  // Must be >= 0x80 for custom
    name: String,
    version: Version
}
```

## Profile Negotiation

When nodes connect, they negotiate compatible profiles:

```rust
fn negotiate_profile(local: &[ProfileId], remote: &[ProfileId]) -> Option<ProfileId> {
    // Find highest common profile
    local.iter()
        .filter(|p| remote.contains(p))
        .max_by_key(|p| p.capability_level())
}
```

## Key Insight

> Profiles don't change ELARA's laws. They configure how those laws manifest for specific use cases.

This is why a single ELARA implementation can handle chat, voice, video, and AI agents - they're all just different profile configurations.
