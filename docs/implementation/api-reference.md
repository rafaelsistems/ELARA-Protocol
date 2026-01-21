# ELARA API Reference

Quick reference for the public APIs of each ELARA crate.

## elara-core

### Identifiers

```rust
// Node identifier (8 bytes, derived from public key)
pub struct NodeId(pub u64);
impl NodeId {
    pub fn new(id: u64) -> Self;
    pub fn generate() -> Self;  // Random for testing
    pub fn to_bytes(&self) -> [u8; 8];
    pub fn from_bytes(bytes: &[u8; 8]) -> Self;
}

// Session identifier (8 bytes)
pub struct SessionId(pub u64);
impl SessionId {
    pub fn new(id: u64) -> Self;
    pub fn to_bytes(&self) -> [u8; 8];
}

// State identifier (type + instance)
pub struct StateId {
    pub state_type: u32,
    pub instance: u32,
}

// Event identifier (source + sequence)
pub struct EventId {
    pub source: NodeId,
    pub seq: u64,
}
```

### Packet Classification

```rust
#[repr(u8)]
pub enum PacketClass {
    Core = 0x00,
    Perceptual = 0x01,
    Enhancement = 0x02,
    Cosmetic = 0x03,
    Repair = 0x04,
}

#[repr(u8)]
pub enum RepresentationProfile {
    Raw = 0x00,
    Textual = 0x01,
    VoiceMinimal = 0x02,
    VoiceRich = 0x03,
    VideoPerceptual = 0x04,
    GroupSwarm = 0x05,
    LivestreamAsymmetric = 0x06,
    Agent = 0x07,
}
```

### Time Types

```rust
// Perceptual time (monotonic, local)
pub struct PerceptualTime(pub Duration);

// State time (elastic, convergent)
pub struct StateTime(pub Duration);
impl StateTime {
    pub fn now() -> Self;
    pub fn from_millis(ms: u64) -> Self;
    pub fn as_millis(&self) -> u64;
    pub fn to_wire(&self) -> i32;  // Relative offset
    pub fn from_wire(base: StateTime, offset: i32) -> Self;
}

// Reality window
pub struct RealityWindow {
    pub τs: StateTime,
    pub Hc: Duration,
    pub Hp: Duration,
}
impl RealityWindow {
    pub fn contains(&self, time: StateTime) -> bool;
    pub fn classify(&self, time: StateTime) -> TimePosition;
}

pub enum TimePosition {
    TooOld,
    Correctable,
    Current,
    Predicted,
    TooFuture,
}
```

### Version Vectors

```rust
pub struct VersionVector {
    entries: HashMap<NodeId, u64>,
}
impl VersionVector {
    pub fn new() -> Self;
    pub fn increment(&mut self, node: NodeId);
    pub fn get(&self, node: NodeId) -> u64;
    pub fn merge(&self, other: &Self) -> Self;
    pub fn happens_before(&self, other: &Self) -> bool;
    pub fn concurrent_with(&self, other: &Self) -> bool;
    pub fn dominates(&self, other: &Self) -> bool;
}
```

### Events

```rust
pub struct Event {
    pub id: EventId,
    pub event_type: EventType,
    pub source: NodeId,
    pub target_state: StateId,
    pub version_ref: VersionVector,
    pub mutation: MutationOp,
    pub time_intent: TimeIntent,
    pub authority_proof: AuthorityProof,
    pub entropy_hint: EntropyHint,
}

pub enum MutationOp {
    Set(Vec<u8>),
    Increment(i64),
    Append(Vec<u8>),
    Merge(Vec<u8>),
    Delete,
    Blend { value: Vec<u8>, weight: f32 },
}
```

### Errors

```rust
pub enum ElaraError {
    InvalidFrame,
    InvalidHeader,
    CryptoError(String),
    ReplayDetected,
    AuthorityViolation,
    CausalityViolation,
    SessionMismatch,
    ResourceExhausted,
    NetworkError(String),
    // ...
}

pub type ElaraResult<T> = Result<T, ElaraError>;
```

## elara-wire

### Frame Header

```rust
pub struct FixedHeader {
    pub version: u8,
    pub crypto_suite: u8,
    pub flags: FrameFlags,
    pub header_len: u16,
    pub session_id: SessionId,
    pub node_id: NodeId,
    pub class: PacketClass,
    pub profile: RepresentationProfile,
    pub seq_window: u32,
    pub time_hint: i32,
}

impl FixedHeader {
    pub fn new(session: SessionId, node: NodeId, class: PacketClass) -> Self;
    pub fn parse(data: &[u8]) -> ElaraResult<Self>;
    pub fn serialize(&self, buf: &mut Vec<u8>) -> ElaraResult<()>;
    pub fn seq(&self) -> u16;
    pub fn set_seq(&mut self, seq: u16);
}

pub struct FrameFlags(pub u8);
impl FrameFlags {
    pub fn multipath(&self) -> bool;
    pub fn relay(&self) -> bool;
    pub fn fragment(&self) -> bool;
    pub fn repair(&self) -> bool;
    pub fn priority(&self) -> bool;
    pub fn has_extensions(&self) -> bool;
}
```

### Frame Building

```rust
pub struct FrameBuilder {
    header: FixedHeader,
    extensions: Extensions,
    payload: Vec<u8>,
}

impl FrameBuilder {
    pub fn new(session: SessionId, node: NodeId, class: PacketClass) -> Self;
    pub fn with_profile(self, profile: RepresentationProfile) -> Self;
    pub fn with_time_hint(self, hint: i32) -> Self;
    pub fn with_seq(self, seq: u16) -> Self;
    pub fn with_payload(self, payload: Vec<u8>) -> Self;
    pub fn with_extension(self, ext: Extension) -> Self;
    pub fn build(self) -> Vec<u8>;
}
```

### Extensions

```rust
pub struct Extensions {
    items: Vec<Extension>,
}

pub enum Extension {
    FragmentInfo { id: u16, seq: u8, total: u8 },
    RelayPath(Vec<NodeId>),
    PriorityHint(u8),
    TimestampFull(u64),
    AckVector(Vec<u16>),
    EpochSync { class: PacketClass, epoch: u32 },
}
```

## elara-crypto

### Identity

```rust
pub struct Identity {
    signing_key: Ed25519SigningKey,
    encryption_secret: X25519StaticSecret,
    node_id: NodeId,
}

impl Identity {
    pub fn generate() -> Self;
    pub fn node_id(&self) -> NodeId;
    pub fn public(&self) -> PublicIdentity;
    pub fn sign(&self, message: &[u8]) -> [u8; 64];
    pub fn to_bytes(&self) -> Vec<u8>;
    pub fn from_bytes(bytes: &[u8]) -> ElaraResult<Self>;
}

pub struct PublicIdentity {
    pub verifying_key: Ed25519VerifyingKey,
    pub encryption_public: X25519PublicKey,
    pub node_id: NodeId,
}

impl PublicIdentity {
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool;
}
```

### AEAD

```rust
pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

pub struct AeadCipher {
    cipher: ChaCha20Poly1305,
}

impl AeadCipher {
    pub fn new(key: &[u8; KEY_SIZE]) -> Self;
    pub fn encrypt(&self, nonce: &[u8; NONCE_SIZE], aad: &[u8], plaintext: &[u8]) -> Vec<u8>;
    pub fn decrypt(&self, nonce: &[u8; NONCE_SIZE], aad: &[u8], ciphertext: &[u8]) -> ElaraResult<Vec<u8>>;
}

pub fn derive_nonce(node_id: NodeId, seq: u16, class: PacketClass) -> [u8; NONCE_SIZE];
```

### Ratchet

```rust
pub struct ClassRatchet {
    chain_key: [u8; 32],
    epoch: u32,
    message_index: u32,
}

impl ClassRatchet {
    pub fn new(root_key: &[u8; 32], class: PacketClass) -> Self;
    pub fn message_key(&self) -> [u8; 32];
    pub fn advance_message(&mut self);
    pub fn advance_epoch(&mut self);
    pub fn epoch(&self) -> u32;
    pub fn sync_to_epoch(&mut self, target_epoch: u32);
}

pub struct MultiRatchet {
    ratchets: HashMap<PacketClass, ClassRatchet>,
}

impl MultiRatchet {
    pub fn new(session_root: &[u8; 32]) -> Self;
    pub fn get(&self, class: PacketClass) -> &ClassRatchet;
    pub fn get_mut(&mut self, class: PacketClass) -> &mut ClassRatchet;
}
```

### Replay Protection

```rust
pub struct ReplayWindow {
    min_seq: u16,
    bitmap: u64,
    window_size: u16,
}

impl ReplayWindow {
    pub fn new(window_size: u16) -> Self;
    pub fn check(&self, seq: u16) -> bool;
    pub fn accept(&mut self, seq: u16) -> bool;
}

pub struct ReplayManager {
    windows: HashMap<(NodeId, PacketClass), ReplayWindow>,
}

impl ReplayManager {
    pub fn new() -> Self;
    pub fn accept(&mut self, node: NodeId, class: PacketClass, seq: u16) -> ElaraResult<()>;
}
```

### Secure Frame Processor

```rust
pub struct SecureFrameProcessor {
    session_id: SessionId,
    node_id: NodeId,
    ratchet: MultiRatchet,
    replay_manager: ReplayManager,
    seq_counters: HashMap<PacketClass, u16>,
}

impl SecureFrameProcessor {
    pub fn new(session: SessionId, node: NodeId, session_key: [u8; 32]) -> Self;
    
    pub fn encrypt_frame(
        &mut self,
        class: PacketClass,
        profile: RepresentationProfile,
        time_hint: i32,
        extensions: Extensions,
        payload: &[u8],
    ) -> ElaraResult<Vec<u8>>;
    
    pub fn decrypt_frame(&mut self, data: &[u8]) -> ElaraResult<DecryptedFrame>;
}

pub struct DecryptedFrame {
    pub header: FixedHeader,
    pub extensions: Extensions,
    pub payload: Vec<u8>,
    pub source: NodeId,
}
```

## elara-time

### Clocks

```rust
pub struct PerceptualClock {
    base: Instant,
}

impl PerceptualClock {
    pub fn new() -> Self;
    pub fn now(&self) -> PerceptualTime;
}

pub struct StateClock {
    base: Instant,
    offset: f64,
    rate: f64,
}

impl StateClock {
    pub fn new() -> Self;
    pub fn now(&self) -> StateTime;
    pub fn apply_correction(&mut self, correction: f64, weight: f64);
    pub fn set_rate(&mut self, rate: f64);
}
```

### Time Engine

```rust
pub struct TimeEngineConfig {
    pub Hp_min: Duration,
    pub Hp_max: Duration,
    pub Hc_min: Duration,
    pub Hc_max: Duration,
}

pub struct TimeEngine {
    perceptual: PerceptualClock,
    state: StateClock,
    config: TimeEngineConfig,
    network: NetworkModel,
    Hp: Duration,
    Hc: Duration,
}

impl TimeEngine {
    pub fn new(config: TimeEngineConfig) -> Self;
    pub fn τp(&self) -> PerceptualTime;
    pub fn τs(&self) -> StateTime;
    pub fn Hp(&self) -> Duration;
    pub fn Hc(&self) -> Duration;
    pub fn reality_window(&self) -> RealityWindow;
    pub fn tick(&mut self, dt: Duration);
    pub fn update_from_packet(&mut self, peer: NodeId, local: f64, remote: f64, seq: u16);
    pub fn correction_weight(&self, event_time: StateTime) -> f64;
}
```

### Network Model

```rust
pub struct PeerNetworkModel {
    pub offset: f64,
    pub skew: f64,
    pub jitter_envelope: f64,
    pub sample_count: u32,
}

pub struct NetworkModel {
    pub peers: HashMap<NodeId, PeerNetworkModel>,
    pub latency_mean: f64,
    pub jitter: f64,
    pub reorder_depth: u32,
    pub loss_rate: f64,
    pub stability_score: f64,
}

impl NetworkModel {
    pub fn new() -> Self;
    pub fn update_from_packet(&mut self, peer: NodeId, local: f64, remote: f64, seq: u16);
    pub fn record_loss(&mut self, lost: u32, total: u32);
    pub fn get_peer(&self, peer: NodeId) -> Option<&PeerNetworkModel>;
}
```

## elara-state

### State Field

```rust
pub struct StateField {
    pub atoms: HashMap<StateId, StateAtom>,
    quarantine: Vec<QuarantinedEvent>,
}

impl StateField {
    pub fn new() -> Self;
    pub fn get(&self, id: &StateId) -> Option<&StateAtom>;
    pub fn get_mut(&mut self, id: &StateId) -> Option<&mut StateAtom>;
    pub fn insert(&mut self, atom: StateAtom);
    pub fn remove(&mut self, id: &StateId) -> Option<StateAtom>;
    pub fn contains(&self, id: &StateId) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = &StateAtom>;
    pub fn iter_by_type(&self, state_type: StateType) -> impl Iterator<Item = &StateAtom>;
}
```

### Reconciliation Engine

```rust
pub struct ReconciliationEngine {
    state_field: StateField,
    local_node: NodeId,
}

impl ReconciliationEngine {
    pub fn new(local_node: NodeId) -> Self;
    pub fn state_field(&self) -> &StateField;
    pub fn state_field_mut(&mut self) -> &mut StateField;
    
    pub fn process_event(
        &mut self,
        event: Event,
        time_engine: &TimeEngine,
    ) -> EventResult;
}

pub enum EventResult {
    Applied,
    Quarantined(QuarantineReason),
    Rejected(RejectReason),
}
```

## elara-test

### Time Simulator

```rust
pub struct ClockDriftModel {
    pub drift_ppm: f64,
    pub jitter_ms: f64,
}

impl ClockDriftModel {
    pub fn perfect() -> Self;
    pub fn fast(ppm: f64) -> Self;
    pub fn slow(ppm: f64) -> Self;
    pub fn unstable(jitter: f64) -> Self;
}

pub struct SimulatedNode {
    pub node_id: NodeId,
    time_engine: TimeEngine,
    drift_model: ClockDriftModel,
}

pub struct TimeSimulator {
    nodes: Vec<SimulatedNode>,
    links: Vec<(usize, usize)>,
}

impl TimeSimulator {
    pub fn new() -> Self;
    pub fn add_node(&mut self, drift: ClockDriftModel) -> usize;
    pub fn add_link(&mut self, a: usize, b: usize);
    pub fn step(&mut self, dt: Duration);
    pub fn run(&mut self, duration: Duration, step: Duration) -> SimulationResult;
}
```

### State Fuzzer

```rust
pub struct FuzzerConfig {
    pub node_count: usize,
    pub event_count: usize,
    pub reorder_probability: f64,
    pub drop_probability: f64,
    pub partition_probability: f64,
    pub seed: u64,
}

impl FuzzerConfig {
    pub fn light() -> Self;
    pub fn default() -> Self;
    pub fn heavy() -> Self;
    pub fn adversarial() -> Self;
}

pub struct StateFuzzer {
    config: FuzzerConfig,
    nodes: Vec<FuzzNode>,
}

impl StateFuzzer {
    pub fn new(config: FuzzerConfig) -> Self;
    pub fn run(&mut self) -> FuzzResult;
}

pub struct FuzzResult {
    pub converged: bool,
    pub events_processed: usize,
    pub authority_violations: usize,
    pub causality_violations: usize,
}
```

### Chaos Network

```rust
pub struct ChaosConfig {
    pub latency_base: Duration,
    pub jitter_range: Range<Duration>,
    pub loss_rate: f64,
    pub reorder_rate: f64,
    pub duplicate_rate: f64,
}

pub struct ChaosNetwork {
    config: ChaosConfig,
    rng: StdRng,
}

impl ChaosNetwork {
    pub fn new(config: ChaosConfig) -> Self;
    pub fn should_drop(&mut self) -> bool;
    pub fn delay(&mut self) -> Duration;
    pub fn should_reorder(&mut self) -> bool;
}
```
