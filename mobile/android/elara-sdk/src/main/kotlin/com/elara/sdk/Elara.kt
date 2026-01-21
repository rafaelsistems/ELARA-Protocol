/**
 * ELARA SDK for Android
 * 
 * Kotlin wrapper for the ELARA FFI library.
 * This provides a native Kotlin API for ELARA protocol operations.
 */
package com.elara.sdk

import java.nio.ByteBuffer

/**
 * Main entry point for ELARA SDK
 */
object Elara {
    
    init {
        System.loadLibrary("elara_ffi")
    }
    
    // Native function declarations
    private external fun nativeVersion(): String
    private external fun nativeInit(): Int
    private external fun nativeShutdown()
    
    /**
     * Get the ELARA library version
     */
    fun version(): String = nativeVersion()
    
    /**
     * Initialize the ELARA library
     * Must be called before any other operations
     */
    fun init(): Result<Unit> {
        val result = nativeInit()
        return if (result == 0) {
            Result.success(Unit)
        } else {
            Result.failure(ElaraException(ErrorCode.fromInt(result)))
        }
    }
    
    /**
     * Shutdown the ELARA library
     */
    fun shutdown() = nativeShutdown()
}

/**
 * ELARA Identity - cryptographic identity for a node
 */
class Identity private constructor(private val handle: Long) : AutoCloseable {
    
    companion object {
        private external fun nativeGenerate(): Long
        private external fun nativeFree(handle: Long)
        private external fun nativeNodeId(handle: Long): Long
        private external fun nativePublicKey(handle: Long): ByteArray
        private external fun nativeExport(handle: Long): ByteArray
        private external fun nativeImport(data: ByteArray): Long
        
        /**
         * Generate a new random identity
         */
        fun generate(): Identity {
            val handle = nativeGenerate()
            if (handle == 0L) {
                throw ElaraException(ErrorCode.INTERNAL_ERROR)
            }
            return Identity(handle)
        }
        
        /**
         * Import an identity from bytes
         */
        fun import(data: ByteArray): Identity {
            val handle = nativeImport(data)
            if (handle == 0L) {
                throw ElaraException(ErrorCode.INVALID_ARGUMENT)
            }
            return Identity(handle)
        }
    }
    
    /**
     * Get the node ID for this identity
     */
    val nodeId: NodeId
        get() = NodeId(nativeNodeId(handle))
    
    /**
     * Get the public key bytes
     */
    val publicKey: ByteArray
        get() = nativePublicKey(handle)
    
    /**
     * Export the identity to bytes for storage
     */
    fun export(): ByteArray = nativeExport(handle)
    
    override fun close() {
        nativeFree(handle)
    }
}

/**
 * ELARA Session - a communication session
 */
class Session private constructor(private val handle: Long) : AutoCloseable {
    
    companion object {
        private external fun nativeCreate(identityHandle: Long, sessionId: Long): Long
        private external fun nativeFree(handle: Long)
        private external fun nativeSessionId(handle: Long): Long
        private external fun nativeNodeId(handle: Long): Long
        private external fun nativePresence(handle: Long): FloatArray
        private external fun nativeDegradation(handle: Long): Int
        private external fun nativeSend(handle: Long, dest: Long, data: ByteArray): Int
        private external fun nativeReceive(handle: Long, data: ByteArray): Int
        private external fun nativeTick(handle: Long): Int
        
        /**
         * Create a new session
         */
        fun create(identity: Identity, sessionId: Long): Session {
            // Note: This requires access to identity's internal handle
            // In a real implementation, we'd need a way to get this
            throw NotImplementedError("Requires internal handle access")
        }
    }
    
    /**
     * Get the session ID
     */
    val sessionId: SessionId
        get() = SessionId(nativeSessionId(handle))
    
    /**
     * Get the local node ID
     */
    val nodeId: NodeId
        get() = NodeId(nativeNodeId(handle))
    
    /**
     * Get current presence vector
     */
    val presence: Presence
        get() {
            val values = nativePresence(handle)
            return Presence(
                liveness = values[0],
                immediacy = values[1],
                coherence = values[2],
                relationalContinuity = values[3],
                emotionalBandwidth = values[4]
            )
        }
    
    /**
     * Get current degradation level
     */
    val degradationLevel: DegradationLevel
        get() = DegradationLevel.fromInt(nativeDegradation(handle))
    
    /**
     * Send data to a peer
     */
    fun send(dest: NodeId, data: ByteArray): Result<Unit> {
        val result = nativeSend(handle, dest.value, data)
        return if (result == 0) {
            Result.success(Unit)
        } else {
            Result.failure(ElaraException(ErrorCode.fromInt(result)))
        }
    }
    
    /**
     * Process received data
     */
    fun receive(data: ByteArray): Result<Unit> {
        val result = nativeReceive(handle, data)
        return if (result == 0) {
            Result.success(Unit)
        } else {
            Result.failure(ElaraException(ErrorCode.fromInt(result)))
        }
    }
    
    /**
     * Tick the session (advance time)
     */
    fun tick(): Result<Unit> {
        val result = nativeTick(handle)
        return if (result == 0) {
            Result.success(Unit)
        } else {
            Result.failure(ElaraException(ErrorCode.fromInt(result)))
        }
    }
    
    override fun close() {
        nativeFree(handle)
    }
}

/**
 * Node ID wrapper
 */
@JvmInline
value class NodeId(val value: Long)

/**
 * Session ID wrapper
 */
@JvmInline
value class SessionId(val value: Long)

/**
 * Presence vector
 */
data class Presence(
    val liveness: Float,
    val immediacy: Float,
    val coherence: Float,
    val relationalContinuity: Float,
    val emotionalBandwidth: Float
) {
    /**
     * Calculate overall presence score
     */
    fun score(): Float = (liveness + immediacy + coherence + 
                          relationalContinuity + emotionalBandwidth) / 5f
    
    /**
     * Check if presence is alive
     */
    fun isAlive(): Boolean = score() > 0.1f
}

/**
 * Degradation levels
 */
enum class DegradationLevel(val level: Int) {
    L0_FULL_PERCEPTION(0),
    L1_DISTORTED_PERCEPTION(1),
    L2_FRAGMENTED_PERCEPTION(2),
    L3_SYMBOLIC_PRESENCE(3),
    L4_MINIMAL_PRESENCE(4),
    L5_LATENT_PRESENCE(5);
    
    companion object {
        fun fromInt(value: Int): DegradationLevel {
            return entries.find { it.level == value } ?: L5_LATENT_PRESENCE
        }
    }
}

/**
 * Error codes
 */
enum class ErrorCode(val code: Int) {
    OK(0),
    INVALID_ARGUMENT(-1),
    NOT_INITIALIZED(-2),
    ALREADY_INITIALIZED(-3),
    OUT_OF_MEMORY(-4),
    NETWORK_ERROR(-5),
    CRYPTO_ERROR(-6),
    TIMEOUT(-7),
    SESSION_NOT_FOUND(-8),
    NODE_NOT_FOUND(-9),
    BUFFER_TOO_SMALL(-10),
    INTERNAL_ERROR(-99);
    
    companion object {
        fun fromInt(value: Int): ErrorCode {
            return entries.find { it.code == value } ?: INTERNAL_ERROR
        }
    }
}

/**
 * ELARA Exception
 */
class ElaraException(val errorCode: ErrorCode) : Exception("ELARA error: $errorCode")
