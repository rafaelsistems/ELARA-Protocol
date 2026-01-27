/**
 * ELARA SDK for iOS
 *
 * Swift wrapper for the ELARA FFI library.
 * This provides a native Swift API for ELARA protocol operations.
 */

import Foundation
import AVFoundation

// MARK: - Main Entry Point

/// Main entry point for ELARA SDK
public final class Elara {
    
    /// Shared instance
    public static let shared = Elara()
    
    private init() {}
    
    /// Get the ELARA library version
    public var version: String {
        guard let cStr = elara_version() else { return "unknown" }
        return String(cString: cStr)
    }
    
    /// Initialize the ELARA library
    public func initialize() throws {
        let result = elara_init()
        if result != 0 {
            throw ElaraError(code: result)
        }
    }
    
    /// Shutdown the ELARA library
    public func shutdown() {
        elara_shutdown()
    }
}

// MARK: - Identity

/// ELARA Identity - cryptographic identity for a node
public final class Identity {
    
    private let handle: OpaquePointer
    
    /// Generate a new random identity
    public init() throws {
        guard let h = elara_identity_generate() else {
            throw ElaraError.internalError
        }
        self.handle = h
    }
    
    /// Import an identity from bytes
    public init(data: Data) throws {
        let bytes = [UInt8](data)
        guard let h = bytes.withUnsafeBufferPointer({ ptr in
            elara_identity_import(ptr.baseAddress, bytes.count)
        }) else {
            throw ElaraError.invalidArgument
        }
        self.handle = h
    }
    
    deinit {
        elara_identity_free(handle)
    }
    
    /// Get the node ID for this identity
    public var nodeId: NodeId {
        let id = elara_identity_node_id(handle)
        return NodeId(value: id.value)
    }
    
    /// Get the public key bytes
    public var publicKey: Data {
        var buffer = [UInt8](repeating: 0, count: 32)
        let len = elara_identity_public_key(handle, &buffer, 32)
        if len > 0 {
            return Data(buffer[0..<Int(len)])
        }
        return Data()
    }
    
    /// Export the identity to bytes for storage
    public func export() -> Data {
        let bytes = elara_identity_export(handle)
        if bytes.data != nil && bytes.len > 0 {
            let data = Data(bytes: bytes.data!, count: bytes.len)
            elara_free_bytes(bytes.data, bytes.len)
            return data
        }
        return Data()
    }
    
    /// Internal handle access for session creation
    internal var internalHandle: OpaquePointer { handle }
}

// MARK: - Session

/// ELARA Session - a communication session
public final class Session {
    
    private let handle: OpaquePointer
    
    /// Create a new session
    public init(identity: Identity, sessionId: UInt64) throws {
        guard let h = elara_session_create(identity.internalHandle, sessionId) else {
            throw ElaraError.internalError
        }
        self.handle = h
    }
    
    deinit {
        clearCallback()
        elara_session_free(handle)
    }
    
    /// Get the session ID
    public var sessionId: SessionId {
        let id = elara_session_id(handle)
        return SessionId(value: id.value)
    }
    
    /// Get the local node ID
    public var nodeId: NodeId {
        let id = elara_session_node_id(handle)
        return NodeId(value: id.value)
    }
    
    /// Get current presence vector
    public var presence: Presence {
        let p = elara_session_presence(handle)
        return Presence(
            liveness: p.liveness,
            immediacy: p.immediacy,
            coherence: p.coherence,
            relationalContinuity: p.relational_continuity,
            emotionalBandwidth: p.emotional_bandwidth
        )
    }
    
    /// Get current degradation level
    public var degradationLevel: DegradationLevel {
        let level = elara_session_degradation(handle)
        return DegradationLevel(rawValue: Int(level.rawValue)) ?? .l5LatentPresence
    }
    
    /// Send data to a peer
    public func send(to dest: NodeId, data: Data) throws {
        let bytes = [UInt8](data)
        let destId = ElaraNodeId(value: dest.value)
        let result = bytes.withUnsafeBufferPointer { ptr in
            elara_session_send(handle, destId, ptr.baseAddress, bytes.count)
        }
        if result != 0 {
            throw ElaraError(code: result)
        }
    }
    
    /// Process received data
    public func receive(data: Data) throws {
        let bytes = [UInt8](data)
        let result = bytes.withUnsafeBufferPointer { ptr in
            elara_session_receive(handle, ptr.baseAddress, bytes.count)
        }
        if result != 0 {
            throw ElaraError(code: result)
        }
    }

    public func setSessionKey(sessionId: UInt64, key: Data) throws {
        let bytes = [UInt8](key)
        let result = bytes.withUnsafeBufferPointer { ptr in
            elara_session_set_session_key(handle, sessionId, ptr.baseAddress, bytes.count)
        }
        if result != 0 {
            throw ElaraError(code: result)
        }
    }
    
    /// Tick the session (advance time)
    public func tick() throws {
        let result = elara_session_tick(handle)
        if result != 0 {
            throw ElaraError(code: result)
        }
    }

    public func setCallback(_ callback: SessionCallback?) throws {
        clearCallback()
        guard let callback = callback else {
            elara_session_clear_callbacks(handle)
            return
        }
        let state = CallbackState(callback)
        let unmanaged = Unmanaged.passRetained(state)
        let userData = unmanaged.toOpaque()
        let result = elara_session_set_message_callback(handle, messageCallback, userData)
        if result != 0 {
            unmanaged.release()
            throw ElaraError(code: result)
        }
        let resultPresence = elara_session_set_presence_callback(handle, presenceCallback, userData)
        if resultPresence != 0 {
            unmanaged.release()
            throw ElaraError(code: resultPresence)
        }
        let resultDegradation = elara_session_set_degradation_callback(
            handle,
            degradationCallback,
            userData
        )
        if resultDegradation != 0 {
            unmanaged.release()
            throw ElaraError(code: resultDegradation)
        }
        sessionCallbacksLock.lock()
        sessionCallbacks[handle] = unmanaged
        sessionCallbacksLock.unlock()
    }

    private func clearCallback() {
        sessionCallbacksLock.lock()
        if let unmanaged = sessionCallbacks.removeValue(forKey: handle) {
            unmanaged.release()
        }
        sessionCallbacksLock.unlock()
        elara_session_clear_callbacks(handle)
    }
}

public protocol SessionCallback: AnyObject {
    func onMessage(source: NodeId, data: Data)
    func onPresence(node: NodeId, presence: Presence)
    func onDegradation(level: DegradationLevel)
}

public final class AudioLoopback: SessionCallback {
    private let session: Session
    private let frameDurationMs: Int
    private var sampleRate: Double
    private var frameSamples: Int
    private var engine: AVAudioEngine?
    private var player: AVAudioPlayerNode?
    private var isRunning = false
    private var frameCounter: UInt64 = 0
    private let processingQueue = DispatchQueue(label: "elara.audio.capture")
    private let playbackQueue = DispatchQueue(label: "elara.audio.playback")

    public init(session: Session, frameDurationMs: Int = 20) {
        self.session = session
        self.frameDurationMs = frameDurationMs
        self.sampleRate = 16000
        self.frameSamples = Int(Double(frameDurationMs) * 16)
    }

    public func start() throws {
        if isRunning {
            return
        }

        let audioSession = AVAudioSession.sharedInstance()
        try audioSession.setCategory(
            .playAndRecord,
            mode: .voiceChat,
            options: [.defaultToSpeaker, .allowBluetooth]
        )
        try audioSession.setPreferredSampleRate(sampleRate)
        try audioSession.setPreferredIOBufferDuration(Double(frameDurationMs) / 1000.0)
        try audioSession.setActive(true, options: [])

        let engine = AVAudioEngine()
        let input = engine.inputNode
        let inputFormat = input.outputFormat(forBus: 0)
        sampleRate = inputFormat.sampleRate
        frameSamples = Int(sampleRate * Double(frameDurationMs) / 1000.0)
        let playbackFormat = AVAudioFormat(
            standardFormatWithSampleRate: sampleRate,
            channels: 1
        )

        let player = AVAudioPlayerNode()
        engine.attach(player)
        if let playbackFormat = playbackFormat {
            engine.connect(player, to: engine.mainMixerNode, format: playbackFormat)
        }

        input.removeTap(onBus: 0)
        input.installTap(onBus: 0, bufferSize: AVAudioFrameCount(frameSamples), format: inputFormat) {
            [weak self] buffer, _ in
            self?.processInput(buffer: buffer, format: inputFormat)
        }

        try engine.start()
        player.play()
        self.engine = engine
        self.player = player
        try session.setCallback(self)
        isRunning = true
    }

    public func stop() {
        if !isRunning {
            return
        }
        engine?.inputNode.removeTap(onBus: 0)
        player?.stop()
        engine?.stop()
        try? session.setCallback(nil)
        isRunning = false
    }

    public func onMessage(source: NodeId, data: Data) {
        if data.count != 9 {
            try? session.receive(data: data)
            return
        }
        let frame = decodeFrame(data)
        playbackQueue.async { [weak self] in
            self?.play(frame: frame)
        }
    }

    public func onPresence(node: NodeId, presence: Presence) {
    }

    public func onDegradation(level: DegradationLevel) {
    }

    private func processInput(buffer: AVAudioPCMBuffer, format: AVAudioFormat) {
        processingQueue.async { [weak self] in
            guard let self = self else { return }
            guard let channelData = buffer.floatChannelData else { return }
            let channelCount = Int(format.channelCount)
            let frameLength = Int(buffer.frameLength)
            if frameLength == 0 {
                return
            }

            var mono = [Float](repeating: 0, count: frameLength)
            for channel in 0..<channelCount {
                let data = channelData[channel]
                for i in 0..<frameLength {
                    mono[i] += data[i]
                }
            }
            let scale = 1.0 / Float(channelCount)
            for i in 0..<frameLength {
                mono[i] *= scale
            }

            var offset = 0
            while offset + self.frameSamples <= mono.count {
                let slice = Array(mono[offset..<(offset + self.frameSamples)])
                let energy = self.estimateEnergy(slice)
                let pitch = self.estimatePitch(slice)
                let voiced = energy > 0.02 && pitch >= 50.0 && pitch <= 500.0
                let frameData = self.encodeFrame(
                    voiced: voiced,
                    pitchHz: pitch,
                    energy: energy,
                    durationMs: self.frameDurationMs
                )
                do {
                    try self.session.send(to: self.session.nodeId, data: frameData)
                } catch {
                    break
                }
                offset += self.frameSamples
                self.frameCounter &+= 1
            }
        }
    }

    private func estimateEnergy(_ samples: [Float]) -> Float {
        var sum: Float = 0
        for sample in samples {
            sum += sample * sample
        }
        return sqrt(sum / Float(samples.count))
    }

    private func estimatePitch(_ samples: [Float]) -> Float {
        if samples.count < 2 {
            return 0
        }
        var crossings = 0
        var previous = samples[0]
        for i in 1..<samples.count {
            let current = samples[i]
            if (previous >= 0 && current < 0) || (previous < 0 && current >= 0) {
                crossings += 1
            }
            previous = current
        }
        let duration = Float(samples.count) / Float(sampleRate)
        if duration == 0 {
            return 0
        }
        let freq = Float(crossings) / (2.0 * duration)
        return freq
    }

    private func encodeFrame(voiced: Bool, pitchHz: Float, energy: Float, durationMs: Int) -> Data {
        var bytes = [UInt8](repeating: 0, count: 9)
        let duration = UInt8(min(max(durationMs, 0), 127))
        bytes[0] = (voiced ? 0x80 : 0x00) | duration

        let clampedPitch = min(max(pitchHz, 50.0), 500.0)
        let pitchNorm = (clampedPitch - 50.0) / 450.0
        bytes[1] = UInt8(min(max(Int(pitchNorm * 255.0), 0), 255))

        let energyByte = UInt8(min(max(Int(energy * 255.0), 0), 255))
        bytes[2] = energyByte

        bytes[3] = 0
        bytes[4] = 0
        bytes[5] = 0
        bytes[6] = 0

        let offsetUnits = UInt16((frameCounter * UInt64(durationMs) * 10) & 0xFFFF)
        let ts = offsetUnits.littleEndian
        bytes[7] = UInt8(truncatingIfNeeded: ts & 0xFF)
        bytes[8] = UInt8(truncatingIfNeeded: (ts >> 8) & 0xFF)

        return Data(bytes)
    }

    private func decodeFrame(_ data: Data) -> (voiced: Bool, pitch: Float, energy: Float, durationMs: Int) {
        let bytes = [UInt8](data)
        let voiced = (bytes[0] & 0x80) != 0
        let duration = Int(bytes[0] & 0x7F)
        let pitchByte = Float(bytes[1])
        let energyByte = Float(bytes[2])
        let pitch = 50.0 + (pitchByte / 255.0) * 450.0
        let energy = energyByte / 255.0
        return (voiced, pitch, energy, duration)
    }

    private func play(frame: (voiced: Bool, pitch: Float, energy: Float, durationMs: Int)) {
        guard let player = player else { return }
        guard let engine = engine else { return }
        let durationMs = frame.durationMs > 0 ? frame.durationMs : frameDurationMs
        let samples = Int(sampleRate * Double(durationMs) / 1000.0)
        guard samples > 0 else { return }
        let format = AVAudioFormat(standardFormatWithSampleRate: sampleRate, channels: 1)
        guard let pcmFormat = format else { return }
        guard let buffer = AVAudioPCMBuffer(pcmFormat: pcmFormat, frameCapacity: AVAudioFrameCount(samples)) else {
            return
        }
        buffer.frameLength = AVAudioFrameCount(samples)
        guard let channel = buffer.floatChannelData?[0] else { return }
        let gain = frame.energy
        let freq = max(frame.pitch, 50.0)
        var phase: Float = 0
        let phaseInc = Float(2.0 * Double.pi) * freq / Float(sampleRate)
        for i in 0..<samples {
            let sample: Float
            if frame.voiced {
                sample = sin(phase) * gain
            } else {
                sample = (Float.random(in: -1.0...1.0)) * gain * 0.3
            }
            channel[i] = sample
            phase += phaseInc
            if phase > Float.pi * 2 {
                phase -= Float.pi * 2
            }
        }
        if !engine.isRunning {
            try? engine.start()
        }
        player.scheduleBuffer(buffer, completionHandler: nil)
    }
}

private final class CallbackState {
    var callback: SessionCallback?

    init(_ callback: SessionCallback?) {
        self.callback = callback
    }
}

private var sessionCallbacks: [OpaquePointer: Unmanaged<CallbackState>] = [:]
private let sessionCallbacksLock = NSLock()

private let messageCallback: @convention(c) (
    UnsafeMutableRawPointer?,
    ElaraNodeId,
    UnsafePointer<UInt8>?,
    Int
) -> Void = { userData, source, data, len in
    guard let userData = userData else { return }
    let state = Unmanaged<CallbackState>.fromOpaque(userData).takeUnretainedValue()
    guard let callback = state.callback else { return }
    guard let data = data, len > 0 else { return }
    let payload = Data(bytes: data, count: len)
    callback.onMessage(source: NodeId(value: source.value), data: payload)
}

private let presenceCallback: @convention(c) (
    UnsafeMutableRawPointer?,
    ElaraNodeId,
    ElaraPresence
) -> Void = { userData, node, presence in
    guard let userData = userData else { return }
    let state = Unmanaged<CallbackState>.fromOpaque(userData).takeUnretainedValue()
    guard let callback = state.callback else { return }
    let value = Presence(
        liveness: presence.liveness,
        immediacy: presence.immediacy,
        coherence: presence.coherence,
        relationalContinuity: presence.relational_continuity,
        emotionalBandwidth: presence.emotional_bandwidth
    )
    callback.onPresence(node: NodeId(value: node.value), presence: value)
}

private let degradationCallback: @convention(c) (
    UnsafeMutableRawPointer?,
    ElaraDegradationLevel
) -> Void = { userData, level in
    guard let userData = userData else { return }
    let state = Unmanaged<CallbackState>.fromOpaque(userData).takeUnretainedValue()
    guard let callback = state.callback else { return }
    let mapped = DegradationLevel(rawValue: Int(level.rawValue)) ?? .l5LatentPresence
    callback.onDegradation(level: mapped)
}

// MARK: - Types

/// Node ID
public struct NodeId: Hashable {
    public let value: UInt64
    
    public init(value: UInt64) {
        self.value = value
    }
}

/// Session ID
public struct SessionId: Hashable {
    public let value: UInt64
    
    public init(value: UInt64) {
        self.value = value
    }
}

/// Presence vector
public struct Presence {
    public let liveness: Float
    public let immediacy: Float
    public let coherence: Float
    public let relationalContinuity: Float
    public let emotionalBandwidth: Float
    
    /// Calculate overall presence score
    public var score: Float {
        (liveness + immediacy + coherence + relationalContinuity + emotionalBandwidth) / 5.0
    }
    
    /// Check if presence is alive
    public var isAlive: Bool {
        score > 0.1
    }
}

/// Degradation levels
public enum DegradationLevel: Int {
    case l0FullPerception = 0
    case l1DistortedPerception = 1
    case l2FragmentedPerception = 2
    case l3SymbolicPresence = 3
    case l4MinimalPresence = 4
    case l5LatentPresence = 5
}

// MARK: - Errors

/// ELARA Error
public enum ElaraError: Error {
    case invalidArgument
    case notInitialized
    case alreadyInitialized
    case outOfMemory
    case networkError
    case cryptoError
    case timeout
    case sessionNotFound
    case nodeNotFound
    case bufferTooSmall
    case internalError
    
    init(code: Int32) {
        switch code {
        case -1: self = .invalidArgument
        case -2: self = .notInitialized
        case -3: self = .alreadyInitialized
        case -4: self = .outOfMemory
        case -5: self = .networkError
        case -6: self = .cryptoError
        case -7: self = .timeout
        case -8: self = .sessionNotFound
        case -9: self = .nodeNotFound
        case -10: self = .bufferTooSmall
        default: self = .internalError
        }
    }
}

// MARK: - FFI Declarations (would be in bridging header)

// These would normally be in a bridging header generated by cbindgen
// For now, they serve as documentation of the expected C interface

/*
@_silgen_name("elara_version")
func elara_version() -> UnsafePointer<CChar>?

@_silgen_name("elara_init")
func elara_init() -> Int32

@_silgen_name("elara_shutdown")
func elara_shutdown()

@_silgen_name("elara_identity_generate")
func elara_identity_generate() -> OpaquePointer?

@_silgen_name("elara_identity_free")
func elara_identity_free(_ handle: OpaquePointer)

// ... etc
*/
