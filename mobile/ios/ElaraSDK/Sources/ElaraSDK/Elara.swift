/**
 * ELARA SDK for iOS
 *
 * Swift wrapper for the ELARA FFI library.
 * This provides a native Swift API for ELARA protocol operations.
 */

import Foundation

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
    
    /// Tick the session (advance time)
    public func tick() throws {
        let result = elara_session_tick(handle)
        if result != 0 {
            throw ElaraError(code: result)
        }
    }
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
