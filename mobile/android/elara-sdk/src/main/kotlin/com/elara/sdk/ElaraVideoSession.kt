/**
 * ELARA Video Session
 *
 * Production-ready 1-on-1 video chat session built on ELARA Protocol.
 * Handles Camera2 capture, UDP transport, audio pipeline, adaptive quality,
 * and frame chunking — all following ELARA's core principles:
 *
 *   - "Reality Never Waits" — non-blocking architecture
 *   - "Experience Degrades, Never Collapses" — graceful degradation (L0–L5)
 *
 * Transport: UDP direct peer-to-peer
 * Video: Camera2 API → JPEG compress → ELARA frames → UDP
 * Audio: AudioRecord (PCM 16-bit mono 16kHz) → UDP
 *
 * Usage:
 *   val session = ElaraVideoSession(sessionId, localPeerId, remotePeerId, context)
 *   session.setEventListener(listener)
 *   session.setRemoteAddress(ip, port)
 *   session.connect(callback)
 *   // ... later
 *   session.close()
 */
package com.elara.sdk

import android.app.ActivityManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageFormat
import android.graphics.Matrix
import android.hardware.camera2.*
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.ImageReader
import android.media.MediaRecorder
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.os.PowerManager
import android.util.Log
import java.io.ByteArrayOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetSocketAddress
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * A single 1-on-1 video/audio session over ELARA Protocol.
 *
 * Manages camera capture, audio recording/playback, UDP transport,
 * adaptive quality, and ELARA runtime ticking.
 */
class ElaraVideoSession(
    val sessionId: String,
    val localPeerId: String,
    val remotePeerId: String,
    private val context: Context
) {
    companion object {
        private const val TAG = "ElaraVideoSession"
        private const val MAX_UDP_PAYLOAD = 60000
        private const val FRAME_HEADER_SIZE = 8 // 4 bytes seq + 2 bytes chunk_idx + 2 bytes total_chunks

        // Audio constants
        private const val AUDIO_SAMPLE_RATE = 16000
        private const val AUDIO_CHANNEL = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT
        private const val AUDIO_CHANNEL_OUT = AudioFormat.CHANNEL_OUT_MONO

        // Packet type prefixes
        private const val PACKET_TYPE_VIDEO: Byte = 0x01
        private const val PACKET_TYPE_AUDIO: Byte = 0x02

        private var elaraInitialized = false

        fun ensureInitialized() {
            if (!elaraInitialized) {
                Elara.init()
                elaraInitialized = true
                Log.d(TAG, "ELARA library initialized: ${Elara.version()}")
            }
        }

        /**
         * Detect device performance tier based on available RAM.
         * Used for adaptive video quality settings.
         *
         * @return 0 = low-end (<=3GB), 1 = mid-range (<=6GB), 2 = high-end (>6GB)
         */
        fun getDeviceTier(context: Context): Int {
            val am = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
            val memInfo = ActivityManager.MemoryInfo()
            am.getMemoryInfo(memInfo)
            val totalRamGB = memInfo.totalMem / (1024.0 * 1024.0 * 1024.0)
            return when {
                totalRamGB <= 3.0 -> 0
                totalRamGB <= 6.0 -> 1
                else -> 2
            }
        }
    }

    // Adaptive video settings based on device tier
    private val deviceTier = getDeviceTier(context)
    private val videoWidth = when (deviceTier) { 0 -> 480; 1 -> 640; else -> 640 }
    private val videoHeight = when (deviceTier) { 0 -> 360; 1 -> 480; else -> 480 }
    private val cameraQuality = when (deviceTier) { 0 -> 60; 1 -> 70; else -> 80 }
    private val tickIntervalMs = when (deviceTier) { 0 -> 50L; 1 -> 40L; else -> 33L }
    private val targetSendSize = 55000 // 55KB fits in 1 UDP packet
    private var adaptiveQuality = when (deviceTier) { 0 -> 50; 1 -> 55; else -> 60 }
    private var sendErrorCount = 0

    init {
        Log.d(TAG, "Device tier=$deviceTier W=$videoWidth H=$videoHeight camQ=$cameraQuality adaptQ=$adaptiveQuality")
    }

    // --- Public types ---

    enum class Status {
        WAITING, CONNECTING, CONNECTED, DEGRADED, DISCONNECTED
    }

    data class Stats(
        val quality: Int,           // 0-100
        val latencyMs: Int,
        val packetLoss: Float,
        val bandwidthKbps: Int,
        val duration: Long,         // seconds
        val transportType: String
    )

    interface EventListener {
        fun onConnected()
        fun onDisconnected(reason: String)
        fun onQualityChanged(quality: Int)
        fun onMessage(message: String)
        fun onRemoteFrame(bitmap: Bitmap)
        fun onLocalFrame(bitmap: Bitmap)
    }

    interface ConnectCallback {
        fun onSuccess()
        fun onError(error: String)
    }

    // --- Internal state ---

    private var status = Status.WAITING
    private var eventListener: EventListener? = null
    private val isVideoEnabled = AtomicBoolean(true)
    private val isAudioEnabled = AtomicBoolean(true)
    private var startTime: Long = 0
    private var currentQuality: Int = 100

    // ELARA native handles
    private var identity: Identity? = null
    private var session: Session? = null

    // UDP transport
    private var udpSocket: DatagramSocket? = null
    private var remoteAddress: InetSocketAddress? = null
    private val udpRunning = AtomicBoolean(false)
    private var udpReceiveThread: Thread? = null
    private var udpTickThread: Thread? = null

    // Camera
    private var cameraDevice: CameraDevice? = null
    private var cameraCaptureSession: CameraCaptureSession? = null
    private var imageReader: ImageReader? = null
    private var cameraThread: HandlerThread? = null
    private var cameraHandler: Handler? = null
    private val isCameraRunning = AtomicBoolean(false)
    private var useFrontCamera = true
    private var sensorOrientation: Int = 270

    // WakeLock
    private var wakeLock: PowerManager.WakeLock? = null

    // Video frame tracking
    private var frameSeq: Int = 0
    private val lastRemoteFrame = AtomicReference<Bitmap?>(null)
    private val mainHandler = Handler(Looper.getMainLooper())

    // Frame send thread
    @Volatile private var pendingFrame: ByteArray? = null
    private var sendThread: Thread? = null
    private val sendRunning = AtomicBoolean(false)
    private var frameSendCount = 0

    // Audio
    private var audioRecord: AudioRecord? = null
    private var audioTrack: AudioTrack? = null
    private var audioRecordThread: Thread? = null
    private val isAudioRecording = AtomicBoolean(false)
    private var audioSendCount = 0
    private var audioPlayCount = 0

    // Beauty filter (optional)
    var beautyFilter: ElaraBeautyFilter? = null

    // Frame listener for external view rendering
    var frameListener: FrameListener? = null

    interface FrameListener {
        fun onLocalFrame(sessionId: String, bitmap: Bitmap)
        fun onRemoteFrame(sessionId: String, bitmap: Bitmap)
    }

    // --- Public API ---

    fun setEventListener(listener: EventListener) {
        this.eventListener = listener
    }

    /**
     * Set remote peer address for UDP transport.
     * Call after signaling exchange provides the peer's IP and port.
     */
    fun setRemoteAddress(ip: String, port: Int) {
        remoteAddress = InetSocketAddress(ip, port)
        Log.d(TAG, "Remote address set: $ip:$port")
    }

    /**
     * Get local UDP port for signaling exchange.
     */
    fun getLocalPort(): Int = udpSocket?.localPort ?: 0

    /**
     * Connect to the remote peer and start video/audio streaming.
     */
    fun connect(callback: ConnectCallback) {
        Log.d(TAG, "Connecting to peer: $remotePeerId")
        status = Status.CONNECTING
        startTime = System.currentTimeMillis()

        Thread {
            try {
                // 1. Initialize ELARA
                ensureInitialized()

                // 2. Generate cryptographic identity
                identity = Identity.generate()
                Log.d(TAG, "ELARA identity generated, nodeId: ${identity?.nodeId}")

                // 3. Create ELARA session
                val sessionIdLong = sessionId.hashCode().toLong() and 0xFFFFFFFFL
                session = Session.create(identity!!, sessionIdLong)
                Log.d(TAG, "ELARA session created: $sessionIdLong")

                // 4. Acquire WakeLock to prevent system kill during streaming
                try {
                    val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
                    wakeLock = pm.newWakeLock(
                        PowerManager.PARTIAL_WAKE_LOCK,
                        "ELARA::VideoSession"
                    )
                    wakeLock?.acquire(60 * 60 * 1000L) // Max 1 hour
                    Log.d(TAG, "WakeLock acquired")
                } catch (e: Exception) {
                    Log.w(TAG, "WakeLock acquire failed: ${e.message}")
                }

                // 5. Setup UDP socket
                udpSocket = DatagramSocket(0)
                udpSocket?.soTimeout = 100
                udpSocket?.sendBufferSize = 512 * 1024
                udpSocket?.receiveBufferSize = 512 * 1024
                Log.d(TAG, "UDP socket bound on port: ${udpSocket?.localPort}")

                // 6. Start UDP receive loop
                udpRunning.set(true)
                startUdpReceiveLoop()

                // 7. Start ELARA tick loop
                startTickLoop()

                // 8. Start audio
                try { startAudioCapture() } catch (e: Exception) {
                    Log.e(TAG, "Audio capture failed: ${e.message}", e)
                }
                try { startAudioPlayback() } catch (e: Exception) {
                    Log.e(TAG, "Audio playback failed: ${e.message}", e)
                }

                // 9. Start camera
                startCamera()

                // 10. Start frame send thread
                startSendThread()

                status = Status.CONNECTED
                mainHandler.post {
                    eventListener?.onConnected()
                    callback.onSuccess()
                }
                Log.d(TAG, "ELARA session connected successfully")

            } catch (e: Exception) {
                Log.e(TAG, "Failed to connect ELARA session", e)
                // Tear down any partially started resources
                close()
                mainHandler.post { callback.onError("ELARA connect failed: ${e.message}") }
            }
        }.start()
    }

    fun toggleVideo(): Boolean {
        val newValue = !isVideoEnabled.get()
        isVideoEnabled.set(newValue)
        Log.d(TAG, "Video ${if (newValue) "enabled" else "disabled"}")
        return newValue
    }

    fun toggleAudio(): Boolean {
        val newValue = !isAudioEnabled.get()
        isAudioEnabled.set(newValue)
        Log.d(TAG, "Audio ${if (newValue) "enabled" else "disabled"}")
        return newValue
    }

    fun sendMessage(message: String) {
        Log.d(TAG, "Sending message: $message")
        session?.send(NodeId(0L), message.toByteArray())
        session?.tick()
    }

    fun switchCamera() {
        Log.d(TAG, "Switching camera: front=$useFrontCamera -> front=${!useFrontCamera}")
        useFrontCamera = !useFrontCamera
        try {
            cameraCaptureSession?.close()
            cameraDevice?.close()
            imageReader?.close()
        } catch (e: Exception) {
            Log.w(TAG, "Camera close error during switch: ${e.message}")
        }
        cameraCaptureSession = null
        cameraDevice = null
        imageReader = null
        isCameraRunning.set(false)
        startCamera()
    }

    fun getLastRemoteFrame(): Bitmap? = lastRemoteFrame.get()

    fun getStats(): Stats {
        val duration = if (startTime > 0) (System.currentTimeMillis() - startTime) / 1000 else 0
        val degradation = session?.degradationLevel?.level ?: 5
        val quality = when (degradation) {
            0 -> 100; 1 -> 80; 2 -> 60; 3 -> 40; 4 -> 20; else -> 5
        }
        return Stats(
            quality = quality,
            latencyMs = 50,
            packetLoss = 0.5f,
            bandwidthKbps = 2500,
            duration = duration,
            transportType = "elara-udp"
        )
    }

    /**
     * Adapt quality based on network conditions.
     * ELARA automatically degrades quality but NEVER drops the connection.
     */
    fun adaptQuality(bandwidthKbps: Int, packetLoss: Float) {
        val oldQuality = currentQuality
        currentQuality = when {
            bandwidthKbps >= 2500 && packetLoss < 1.0 -> 100
            bandwidthKbps >= 1500 && packetLoss < 3.0 -> 75
            bandwidthKbps >= 800 && packetLoss < 5.0 -> 50
            bandwidthKbps >= 300 -> 25
            else -> 10 // Audio only mode
        }
        if (currentQuality != oldQuality) {
            Log.d(TAG, "Quality changed: $oldQuality -> $currentQuality")
            if (currentQuality < 50 && status != Status.DEGRADED) {
                status = Status.DEGRADED
            } else if (currentQuality >= 50 && status == Status.DEGRADED) {
                status = Status.CONNECTED
            }
            eventListener?.onQualityChanged(currentQuality)
        }
    }

    /**
     * Close the session and release all resources.
     */
    fun close() {
        Log.d(TAG, "Closing session: $sessionId")
        status = Status.DISCONNECTED
        udpRunning.set(false)

        // Stop send thread
        sendRunning.set(false)
        sendThread?.interrupt()
        sendThread = null

        // Stop audio
        isAudioRecording.set(false)
        try {
            audioRecord?.stop()
            audioRecord?.release()
            audioTrack?.stop()
            audioTrack?.release()
            val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
            audioManager.mode = AudioManager.MODE_NORMAL
            audioManager.isSpeakerphoneOn = false
        } catch (e: Exception) {
            Log.w(TAG, "Audio cleanup error: ${e.message}")
        }
        audioRecord = null
        audioTrack = null
        audioRecordThread?.interrupt()

        // Release WakeLock
        try {
            if (wakeLock?.isHeld == true) {
                wakeLock?.release()
                Log.d(TAG, "WakeLock released")
            }
        } catch (e: Exception) {
            Log.w(TAG, "WakeLock release error: ${e.message}")
        }
        wakeLock = null

        // Stop camera
        try {
            cameraCaptureSession?.close()
            cameraDevice?.close()
            imageReader?.close()
            cameraThread?.quitSafely()
        } catch (e: Exception) {
            Log.w(TAG, "Camera cleanup error: ${e.message}")
        }
        cameraCaptureSession = null
        cameraDevice = null
        imageReader = null
        isCameraRunning.set(false)

        // Stop UDP
        try { udpSocket?.close() } catch (e: Exception) {
            Log.w(TAG, "UDP cleanup error: ${e.message}")
        }
        udpSocket = null

        // Stop threads
        udpReceiveThread?.interrupt()
        udpTickThread?.interrupt()

        // Free ELARA native resources
        session?.close()
        identity?.close()
        session = null
        identity = null

        eventListener?.onDisconnected("Session closed")
    }

    // --- Internal: UDP receive ---

    private fun startUdpReceiveLoop() {
        udpReceiveThread = Thread {
            val buf = ByteArray(MAX_UDP_PAYLOAD + 1 + FRAME_HEADER_SIZE)
            Log.d(TAG, "UDP receive loop started")

            var currentFrameSeq = -1
            var expectedChunks = 0
            val chunks = mutableMapOf<Int, ByteArray>()

            while (udpRunning.get()) {
                try {
                    val packet = DatagramPacket(buf, buf.size)
                    udpSocket?.receive(packet)
                    if (packet.length < 2) continue

                    val data = packet.data
                    val offset = packet.offset

                    when (data[offset]) {
                        PACKET_TYPE_AUDIO -> {
                            val audioLen = packet.length - 1
                            if (audioLen > 0) {
                                val pcm = data.copyOfRange(offset + 1, offset + 1 + audioLen)
                                playAudioData(pcm, audioLen)
                            }
                        }
                        PACKET_TYPE_VIDEO -> {
                            val headerStart = offset + 1
                            if (packet.length < 1 + FRAME_HEADER_SIZE) continue

                            val seq = ((data[headerStart].toInt() and 0xFF) shl 24) or
                                    ((data[headerStart + 1].toInt() and 0xFF) shl 16) or
                                    ((data[headerStart + 2].toInt() and 0xFF) shl 8) or
                                    (data[headerStart + 3].toInt() and 0xFF)
                            val chunkIdx = ((data[headerStart + 4].toInt() and 0xFF) shl 8) or
                                    (data[headerStart + 5].toInt() and 0xFF)
                            val totalChunks = ((data[headerStart + 6].toInt() and 0xFF) shl 8) or
                                    (data[headerStart + 7].toInt() and 0xFF)

                            val payload = data.copyOfRange(headerStart + FRAME_HEADER_SIZE, offset + packet.length)

                            if (seq != currentFrameSeq) {
                                currentFrameSeq = seq
                                expectedChunks = totalChunks
                                chunks.clear()
                            }
                            chunks[chunkIdx] = payload

                            if (chunks.size == expectedChunks) {
                                val fullData = ByteArrayOutputStream()
                                for (i in 0 until expectedChunks) {
                                    chunks[i]?.let { fullData.write(it) }
                                }
                                chunks.clear()

                                val jpegBytes = fullData.toByteArray()
                                val rawBitmap = BitmapFactory.decodeByteArray(jpegBytes, 0, jpegBytes.size)
                                if (rawBitmap != null) {
                                    lastRemoteFrame.set(rawBitmap)
                                    frameListener?.onRemoteFrame(sessionId, rawBitmap)
                                    eventListener?.onRemoteFrame(rawBitmap)
                                }
                            }
                        }
                    }
                } catch (e: java.net.SocketTimeoutException) {
                    // Normal timeout
                } catch (e: Exception) {
                    if (udpRunning.get()) Log.w(TAG, "UDP receive error: ${e.message}")
                }
            }
            Log.d(TAG, "UDP receive loop stopped")
        }
        udpReceiveThread?.name = "ELARA-UDP-Recv"
        udpReceiveThread?.isDaemon = true
        udpReceiveThread?.start()
    }

    // --- Internal: ELARA tick ---

    private fun startTickLoop() {
        udpTickThread = Thread {
            Log.d(TAG, "ELARA tick loop started")
            while (udpRunning.get()) {
                try {
                    session?.tick()
                    val degradation = session?.degradationLevel?.level ?: 5
                    val newQuality = when (degradation) {
                        0 -> 100; 1 -> 80; 2 -> 60; 3 -> 40; 4 -> 20; else -> 5
                    }
                    if (newQuality != currentQuality) {
                        currentQuality = newQuality
                        val newStatus = if (newQuality < 50) Status.DEGRADED else Status.CONNECTED
                        if (newStatus != status && status != Status.DISCONNECTED) {
                            status = newStatus
                            mainHandler.post { eventListener?.onQualityChanged(newQuality) }
                        }
                    }
                    Thread.sleep(tickIntervalMs)
                } catch (e: InterruptedException) { break }
                catch (e: Exception) { Log.w(TAG, "Tick error: ${e.message}") }
            }
            Log.d(TAG, "ELARA tick loop stopped")
        }
        udpTickThread?.name = "ELARA-Tick"
        udpTickThread?.isDaemon = true
        udpTickThread?.start()
    }

    // --- Internal: Camera ---

    private fun startCamera() {
        cameraThread = HandlerThread("ELARA-Camera").also { it.start() }
        cameraHandler = Handler(cameraThread!!.looper)

        imageReader = ImageReader.newInstance(videoWidth, videoHeight, ImageFormat.JPEG, 2)
        imageReader?.setOnImageAvailableListener({ reader ->
            val image = reader.acquireLatestImage() ?: return@setOnImageAvailableListener
            try {
                if (!isVideoEnabled.get()) return@setOnImageAvailableListener

                val buffer = image.planes[0].buffer
                val bytes = ByteArray(buffer.remaining())
                buffer.get(bytes)

                val rawBitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
                    ?: return@setOnImageAvailableListener

                // Apply beauty filter if set
                var processed: Bitmap? = null
                beautyFilter?.let { filter ->
                    if (filter.beautyLevel > 0) {
                        try { processed = filter.apply(rawBitmap) }
                        catch (e: Exception) { Log.w(TAG, "Beauty filter error: ${e.message}") }
                    }
                }

                // Rotate for local preview (mirror for front camera)
                val sourceForLocal = processed ?: rawBitmap
                val localRotated = rotateBitmap(sourceForLocal, sensorOrientation, mirror = useFrontCamera)
                frameListener?.onLocalFrame(sessionId, localRotated)
                eventListener?.onLocalFrame(localRotated)

                // Rotate for sending (no mirror — partner sees correct orientation)
                if (remoteAddress != null) {
                    val sourceForSend = processed ?: rawBitmap
                    val sendRotated = rotateBitmap(sourceForSend, sensorOrientation, mirror = false)
                    val out = ByteArrayOutputStream()
                    sendRotated.compress(Bitmap.CompressFormat.JPEG, cameraQuality, out)
                    sendVideoFrame(out.toByteArray())
                    if (sendRotated !== sourceForSend) sendRotated.recycle()
                }

                if (processed != null && processed !== rawBitmap) processed?.recycle()
                rawBitmap.recycle()
            } catch (e: Exception) {
                Log.w(TAG, "Image capture error: ${e.message}")
            } finally {
                image.close()
            }
        }, cameraHandler)

        val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val targetFacing = if (useFrontCamera) CameraCharacteristics.LENS_FACING_FRONT
                           else CameraCharacteristics.LENS_FACING_BACK
        val selectedCameraId = cameraManager.cameraIdList.firstOrNull { id ->
            cameraManager.getCameraCharacteristics(id)
                .get(CameraCharacteristics.LENS_FACING) == targetFacing
        } ?: cameraManager.cameraIdList.firstOrNull()

        if (selectedCameraId == null) {
            Log.e(TAG, "No camera found")
            return
        }

        val characteristics = cameraManager.getCameraCharacteristics(selectedCameraId)
        sensorOrientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 270
        Log.d(TAG, "Camera sensor orientation: $sensorOrientation (front=$useFrontCamera)")

        try {
            cameraManager.openCamera(selectedCameraId, object : CameraDevice.StateCallback() {
                override fun onOpened(camera: CameraDevice) {
                    cameraDevice = camera
                    isCameraRunning.set(true)
                    createCaptureSession(camera)
                    Log.d(TAG, "Camera opened: $selectedCameraId (front=$useFrontCamera)")
                }
                override fun onDisconnected(camera: CameraDevice) {
                    camera.close(); cameraDevice = null; isCameraRunning.set(false)
                }
                override fun onError(camera: CameraDevice, error: Int) {
                    Log.e(TAG, "Camera error: $error")
                    camera.close(); cameraDevice = null; isCameraRunning.set(false)
                }
            }, cameraHandler)
        } catch (e: SecurityException) {
            Log.e(TAG, "Camera permission denied", e)
        }
    }

    private fun rotateBitmap(bitmap: Bitmap, degrees: Int, mirror: Boolean): Bitmap {
        if (degrees == 0 && !mirror) return bitmap
        val matrix = Matrix()
        matrix.postRotate(degrees.toFloat())
        if (mirror) matrix.postScale(-1f, 1f)
        return Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true)
    }

    private fun createCaptureSession(camera: CameraDevice) {
        try {
            val surface = imageReader?.surface ?: return
            camera.createCaptureSession(
                listOf(surface),
                object : CameraCaptureSession.StateCallback() {
                    override fun onConfigured(session: CameraCaptureSession) {
                        cameraCaptureSession = session
                        val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                            addTarget(surface)
                            set(CaptureRequest.JPEG_QUALITY, cameraQuality.toByte())
                        }
                        session.setRepeatingRequest(request.build(), null, cameraHandler)
                        Log.d(TAG, "Camera capture session started")
                    }
                    override fun onConfigureFailed(session: CameraCaptureSession) {
                        Log.e(TAG, "Camera capture session configure failed")
                    }
                },
                cameraHandler
            )
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create capture session", e)
        }
    }

    // --- Internal: Video frame send ---

    private fun startSendThread() {
        sendRunning.set(true)
        sendThread = Thread {
            while (sendRunning.get() && udpRunning.get()) {
                try {
                    val frame = pendingFrame
                    pendingFrame = null
                    if (frame != null) doSendFrame(frame)
                    Thread.sleep(tickIntervalMs)
                } catch (e: InterruptedException) { break }
                catch (e: Exception) { if (sendRunning.get()) Log.w(TAG, "Send thread error: ${e.message}") }
            }
        }
        sendThread?.name = "ELARA-FrameSend"
        sendThread?.isDaemon = true
        sendThread?.start()
    }

    /**
     * Smart compress: compress bitmap to JPEG that fits within targetSendSize.
     * Starts at adaptiveQuality, steps down by 5 until it fits.
     */
    private fun compressToTargetSize(bitmap: Bitmap): ByteArray? {
        var quality = adaptiveQuality
        while (quality >= 20) {
            val out = ByteArrayOutputStream()
            bitmap.compress(Bitmap.CompressFormat.JPEG, quality, out)
            val data = out.toByteArray()
            if (data.size <= targetSendSize) return data
            quality -= 5
        }
        val out = ByteArrayOutputStream()
        bitmap.compress(Bitmap.CompressFormat.JPEG, 15, out)
        return out.toByteArray()
    }

    private fun sendVideoFrame(jpegBytes: ByteArray) {
        if (jpegBytes.size <= targetSendSize) {
            pendingFrame = jpegBytes
            return
        }
        val bmp = BitmapFactory.decodeByteArray(jpegBytes, 0, jpegBytes.size)
        if (bmp != null) {
            val compressed = compressToTargetSize(bmp)
            bmp.recycle()
            if (compressed != null && compressed.size <= MAX_UDP_PAYLOAD - 20) {
                pendingFrame = compressed
            }
        }
    }

    /**
     * Send a video frame via UDP with chunking.
     * Packet format: [type:1][seq:4][chunk_idx:2][total_chunks:2][payload]
     */
    private fun doSendFrame(jpegBytes: ByteArray) {
        val remote = remoteAddress ?: return
        val socket = udpSocket ?: return

        try {
            frameSeq++
            val headerSize = 1 + FRAME_HEADER_SIZE
            val chunkSize = MAX_UDP_PAYLOAD - headerSize
            val totalChunks = (jpegBytes.size + chunkSize - 1) / chunkSize

            for (i in 0 until totalChunks) {
                val start = i * chunkSize
                val end = minOf(start + chunkSize, jpegBytes.size)
                val chunkData = jpegBytes.copyOfRange(start, end)

                val packet = ByteArray(headerSize + chunkData.size)
                packet[0] = PACKET_TYPE_VIDEO
                packet[1] = ((frameSeq shr 24) and 0xFF).toByte()
                packet[2] = ((frameSeq shr 16) and 0xFF).toByte()
                packet[3] = ((frameSeq shr 8) and 0xFF).toByte()
                packet[4] = (frameSeq and 0xFF).toByte()
                packet[5] = ((i shr 8) and 0xFF).toByte()
                packet[6] = (i and 0xFF).toByte()
                packet[7] = ((totalChunks shr 8) and 0xFF).toByte()
                packet[8] = (totalChunks and 0xFF).toByte()
                System.arraycopy(chunkData, 0, packet, headerSize, chunkData.size)

                socket.send(DatagramPacket(packet, packet.size, remote))
            }

            frameSendCount++
            sendErrorCount = 0
            checkAdaptiveQualityUp()
            if (frameSendCount % 30 == 1) {
                Log.d(TAG, "Sent frame #$frameSendCount size=${jpegBytes.size} chunks=$totalChunks Q=$adaptiveQuality")
            }
        } catch (e: Exception) {
            if (sendRunning.get()) {
                Log.w(TAG, "Send frame error: ${e.message}")
                sendErrorCount++
                if (sendErrorCount >= 3 && adaptiveQuality > 25) {
                    adaptiveQuality -= 5
                    sendErrorCount = 0
                    Log.d(TAG, "Adaptive quality lowered to $adaptiveQuality")
                }
            }
        }
    }

    private var stableFrameCount = 0
    private fun checkAdaptiveQualityUp() {
        stableFrameCount++
        val maxQ = when (deviceTier) { 0 -> 50; 1 -> 55; else -> 60 }
        if (stableFrameCount >= 60 && adaptiveQuality < maxQ) {
            adaptiveQuality += 5
            stableFrameCount = 0
            Log.d(TAG, "Adaptive quality raised to $adaptiveQuality")
        }
    }

    // --- Internal: Audio ---

    private fun startAudioCapture() {
        val bufSize = AudioRecord.getMinBufferSize(AUDIO_SAMPLE_RATE, AUDIO_CHANNEL, AUDIO_FORMAT)
        try {
            audioRecord = AudioRecord(
                MediaRecorder.AudioSource.VOICE_COMMUNICATION,
                AUDIO_SAMPLE_RATE, AUDIO_CHANNEL, AUDIO_FORMAT, bufSize * 2
            )
            if (audioRecord?.state != AudioRecord.STATE_INITIALIZED) {
                Log.e(TAG, "AudioRecord failed to initialize")
                return
            }
            audioRecord?.startRecording()
            isAudioRecording.set(true)
            Log.d(TAG, "Audio capture started")

            audioRecordThread = Thread {
                val buf = ByteArray(bufSize)
                while (isAudioRecording.get() && udpRunning.get()) {
                    try {
                        val read = audioRecord?.read(buf, 0, buf.size) ?: -1
                        if (read > 0 && isAudioEnabled.get() && remoteAddress != null) {
                            sendAudioPacket(buf, read)
                            audioSendCount++
                            if (audioSendCount % 100 == 1) {
                                Log.d(TAG, "Audio sent #$audioSendCount bytes=$read")
                            }
                        }
                    } catch (e: Exception) {
                        if (isAudioRecording.get()) Log.w(TAG, "Audio capture error: ${e.message}")
                    }
                }
            }
            audioRecordThread?.name = "ELARA-AudioCapture"
            audioRecordThread?.isDaemon = true
            audioRecordThread?.start()
        } catch (e: SecurityException) {
            Log.e(TAG, "Audio permission denied", e)
        }
    }

    private fun startAudioPlayback() {
        val audioManager = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
        audioManager.mode = AudioManager.MODE_IN_COMMUNICATION
        audioManager.isSpeakerphoneOn = true

        val bufSize = AudioTrack.getMinBufferSize(AUDIO_SAMPLE_RATE, AUDIO_CHANNEL_OUT, AUDIO_FORMAT)
        audioTrack = AudioTrack(
            AudioManager.STREAM_VOICE_CALL,
            AUDIO_SAMPLE_RATE, AUDIO_CHANNEL_OUT, AUDIO_FORMAT,
            bufSize * 2, AudioTrack.MODE_STREAM
        )
        audioTrack?.play()
        Log.d(TAG, "Audio playback started")
    }

    /**
     * Send audio packet via UDP.
     * Packet format: [type:1][pcm_payload]
     */
    private fun sendAudioPacket(pcmData: ByteArray, length: Int) {
        val remote = remoteAddress ?: return
        val socket = udpSocket ?: return
        try {
            val packet = ByteArray(1 + length)
            packet[0] = PACKET_TYPE_AUDIO
            System.arraycopy(pcmData, 0, packet, 1, length)
            socket.send(DatagramPacket(packet, packet.size, remote))
        } catch (_: Exception) { }
    }

    private fun playAudioData(pcmData: ByteArray, length: Int) {
        try {
            audioTrack?.write(pcmData, 0, length)
            audioPlayCount++
            if (audioPlayCount % 100 == 1) {
                Log.d(TAG, "Audio play #$audioPlayCount bytes=$length")
            }
        } catch (e: Exception) {
            Log.w(TAG, "Audio playback error: ${e.message}")
        }
    }
}
