# Ringkasan Protokol ELARA

**ELARA** = **E**mylton **L**eunufna **A**daptive **R**eality **A**rchitecture

Substrat komunikasi real-time universal untuk sinkronisasi realitas kriptografis.

## Apa itu ELARA?

ELARA **BUKAN**:
- Protokol chat
- Protokol voice/video
- Protokol streaming
- WebRTC wrapper

ELARA **ADALAH**:
- Fondasi komunikasi universal
- Semua modalitas (chat, voice, video, presence, AI) adalah konfigurasi di atas satu hukum protokol

## Filosofi Inti

> "Komunikasi bukan pengiriman pesan. Komunikasi adalah sinkronisasi realitas."

## Lima Primitif Fundamental

| Primitif | Simbol | Deskripsi |
|----------|--------|-----------|
| **State** | ω | Realitas yang hidup |
| **Event** | ε | Mutasi yang sah |
| **Time** | τ | Objek protokol |
| **Authority** | - | Siapa boleh mengubah apa |
| **Interest** | - | Siapa perlu melihat apa |

## Empat Pilar

### 1. Cryptographic Reality Physics
- Identitas terikat kriptografis
- Session key hierarkis
- Multi-ratchet per kelas paket
- AEAD (ChaCha20-Poly1305)

### 2. Time Convergence Engine
- Dual clock: τp (persepsi) dan τs (state)
- Reality Window: [τs - Hc, τs + Hp]
- Koreksi non-destruktif
- Adaptasi horizon

### 3. State Field & Swarm Diffusion
- Pipeline rekonsiliasi 6 tahap
- Version vector untuk kausalitas
- Penanganan partisi
- Kontrol divergensi

### 4. Packet Ecology & Wire Semantics
- 5 kelas paket (Core, Perceptual, Enhancement, Cosmetic, Repair)
- Frame header 28 byte
- Degradasi graceful
- Replay protection

## Representation Profiles

| Profile | Kegunaan |
|---------|----------|
| Textual | Chat, pesan |
| VoiceMinimal | Panggilan suara |
| VoiceRich | Suara HD |
| VideoPerceptual | Video call |
| GroupSwarm | Komunikasi grup |
| Agent | Integrasi AI/Bot |

## MSP v0 (Minimum Survivable Product)

### Scope
- ✅ Text real-time + async
- ✅ Voice real-time (minimal)
- ✅ 1-1 dan small group
- ✅ NAT hostile networks
- ✅ Full cryptographic physics
- ❌ Video
- ❌ Neural codec
- ❌ AI rendering

### Target Hardware
- ≤2 core CPU
- ≤2 GB RAM
- Tanpa GPU
- Jaringan 2G-class

### Degradasi Graceful

```
Full Quality
    ↓
Reduced Quality
    ↓
Symbolic
    ↓
Presence Only
    ↓
Identity Heartbeat

⚠️ SESSION TIDAK PERNAH PUTUS ⚠️
Realitas menyederhanakan, koneksi tetap.
```

## Struktur Crate

```
elara-core      → Tipe dan primitif inti
elara-wire      → Protokol wire
elara-crypto    → Binding kriptografis
elara-time      → Time engine
elara-state     → Rekonsiliasi state
elara-transport → Transport jaringan
elara-runtime   → Runtime node
elara-msp       → Profile MSP
elara-test      → Harness testing
```

## Status Implementasi

| Komponen | Status | Tests |
|----------|--------|-------|
| elara-core | ✅ Selesai | 16 |
| elara-wire | ✅ Selesai | 13 |
| elara-crypto | ✅ Selesai | 23 |
| elara-time | ✅ Selesai | 10 |
| elara-state | ✅ Selesai | 3 |
| elara-transport | ✅ Selesai | 2 |
| elara-runtime | ✅ Selesai | 2 |
| elara-msp | ✅ Selesai | 2 |
| elara-test | ✅ Selesai | 15 |
| **Total** | **✅** | **86** |

## Invariant Inti

1. **Kontinuitas Kriptografis** - Identitas terikat matematis
2. **Koherensi Temporal** - Waktu mengalir, tidak melompat
3. **Konvergensi Eventual** - Semua node mencapai realitas ekuivalen
4. **Survival Terbatas Sumber Daya** - Degradasi graceful, tidak crash
5. **Independensi Representasi** - Hukum sama, tampilan berbeda
6. **Server Blindness** - Infrastruktur tidak bisa membaca konten

## Dokumen Lengkap

### Arsitektur
- `docs/architecture/core-concepts.md` - Konsep fundamental
- `docs/architecture/four-pillars.md` - Empat pilar
- `docs/architecture/representation-profiles.md` - Profile representasi

### Spesifikasi
- `docs/specs/wire-protocol.md` - Format frame biner
- `docs/specs/crypto-binding.md` - Spesifikasi keamanan
- `docs/specs/time-engine.md` - Mekanika temporal
- `docs/specs/state-reconciliation.md` - Spesifikasi konvergensi

### Implementasi
- `docs/implementation/crate-structure.md` - Layout workspace
- `docs/implementation/api-reference.md` - Referensi API
- `docs/implementation/testing-strategy.md` - Strategi testing

### MSP
- `docs/msp/msp-v0.md` - Spesifikasi MSP v0

## Menjalankan

```bash
# Build semua crate
cargo build --workspace

# Jalankan semua test
cargo test --workspace

# Generate dokumentasi
cargo doc --workspace --no-deps
```

## Insight Kunci

> Protokol tradisional: "Jaringan buruk → freeze/reset"
> 
> ELARA: "Jaringan buruk → bengkokkan waktu, sederhanakan realitas, tetap mengalir"

Inilah mengapa ELARA dapat mempertahankan komunikasi berkelanjutan bahkan di bawah kekacauan jaringan yang parah.
