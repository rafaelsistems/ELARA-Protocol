package com.elara.app

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaRecorder
import android.os.Bundle
import android.widget.LinearLayout
import android.widget.TextView
import com.elara.sdk.Elara
import com.elara.sdk.Identity
import com.elara.sdk.NodeId
import com.elara.sdk.Session
import com.elara.sdk.SessionCallback
import kotlin.math.PI
import kotlin.math.max
import kotlin.math.sin
import kotlin.math.sqrt

class MainActivity : Activity() {
    private var session: Session? = null
    private var audioRecord: AudioRecord? = null
    private var audioTrack: AudioTrack? = null
    private var worker: Thread? = null
    private var running = false
    private var phase = 0.0
    private lateinit var presenceText: TextView
    private lateinit var degradationText: TextView

    private val sampleRate = 16000
    private val frameMs = 20
    private val frameSamples = sampleRate * frameMs / 1000

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val layout = LinearLayout(this)
        layout.orientation = LinearLayout.VERTICAL
        val label = TextView(this)
        label.text = "ELARA Voice Loopback"
        presenceText = TextView(this)
        presenceText.text = "Presence: -"
        degradationText = TextView(this)
        degradationText.text = "Degradation: -"
        layout.addView(label)
        layout.addView(presenceText)
        layout.addView(degradationText)
        setContentView(layout)
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
            startPipeline()
        } else {
            requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 100)
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        if (requestCode == 100 && grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
            startPipeline()
        }
    }

    private fun startPipeline() {
        if (running) return
        Elara.init()
        val identity = Identity.generate()
        val created = Session.create(identity, 1L)
        session = created
        val key = ByteArray(32) { 0x42.toByte() }
        created.setSessionKey(created.sessionId, key)

        val bufferSize = max(
            AudioRecord.getMinBufferSize(
                sampleRate,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT
            ),
            frameSamples * 2
        )
        audioRecord = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            sampleRate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
            bufferSize
        )
        audioTrack = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_VOICE_COMMUNICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(sampleRate)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build()
            )
            .setBufferSizeInBytes(frameSamples * 4)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()

        audioTrack?.play()

        created.setCallback(object : SessionCallback {
            override fun onMessage(source: Long, data: ByteArray) {
                val current = session ?: return
                if (data.size != 9) {
                    current.receive(data)
                    return
                }
                val frame = decodeFrame(data)
                val samples = synthesizeFrame(frame)
                audioTrack?.write(samples, 0, samples.size)
            }

            override fun onPresence(node: Long, presence: FloatArray) {
                val text =
                    "Presence: %.2f %.2f %.2f %.2f %.2f".format(
                        presence.getOrNull(0) ?: 0f,
                        presence.getOrNull(1) ?: 0f,
                        presence.getOrNull(2) ?: 0f,
                        presence.getOrNull(3) ?: 0f,
                        presence.getOrNull(4) ?: 0f
                    )
                runOnUiThread { presenceText.text = text }
            }

            override fun onDegradation(level: Int) {
                runOnUiThread { degradationText.text = "Degradation: $level" }
            }
        })

        running = true
        worker = Thread {
            val buffer = ShortArray(frameSamples)
            val localNode = NodeId(created.nodeId.value)
            audioRecord?.startRecording()
            while (running) {
                val read = audioRecord?.read(buffer, 0, buffer.size) ?: 0
                if (read > 0) {
                    val pitch = estimatePitch(buffer, read)
                    val energy = estimateEnergy(buffer, read)
                    val voiced = energy > 0.02f && pitch in 50.0..500.0
                    val frame = encodeFrame(voiced, pitch.toFloat(), energy, frameMs)
                    created.send(localNode, frame)
                }
            }
            audioRecord?.stop()
        }
        worker?.start()
    }

    override fun onDestroy() {
        running = false
        worker?.join(200)
        audioRecord?.release()
        audioTrack?.release()
        session?.close()
        Elara.shutdown()
        super.onDestroy()
    }

    private fun estimateEnergy(buffer: ShortArray, count: Int): Float {
        var sum = 0.0
        for (i in 0 until count) {
            val v = buffer[i].toDouble()
            sum += v * v
        }
        val rms = sqrt(sum / count)
        return (rms / 32768.0).toFloat().coerceIn(0f, 1f)
    }

    private fun estimatePitch(buffer: ShortArray, count: Int): Double {
        var crossings = 0
        var prev = buffer[0]
        for (i in 1 until count) {
            val current = buffer[i]
            if ((prev >= 0 && current < 0) || (prev < 0 && current >= 0)) {
                crossings += 1
            }
            prev = current
        }
        val zeroCrossRate = crossings.toDouble() / count
        val hz = zeroCrossRate * sampleRate / 2.0
        return hz
    }

    private fun encodeFrame(voiced: Boolean, pitchHz: Float, energy: Float, durationMs: Int): ByteArray {
        val data = ByteArray(9)
        val duration = durationMs.coerceIn(0, 127)
        data[0] = ((if (voiced) 0x80 else 0x00) or duration).toByte()
        val pitch = ((pitchHz - 50f) / 450f * 255f).toInt().coerceIn(0, 255)
        val energyQ = (energy * 255f).toInt().coerceIn(0, 255)
        data[1] = pitch.toByte()
        data[2] = energyQ.toByte()
        data[3] = 0
        data[4] = 0
        data[5] = 0
        data[6] = 0
        data[7] = 0
        data[8] = 0
        return data
    }

    private data class VoiceFrame(val voiced: Boolean, val pitchHz: Float, val energy: Float, val durationMs: Int)

    private fun decodeFrame(data: ByteArray): VoiceFrame {
        val voiced = data[0].toInt() and 0x80 != 0
        val duration = data[0].toInt() and 0x7F
        val pitchRaw = data[1].toInt() and 0xFF
        val energyRaw = data[2].toInt() and 0xFF
        val pitchHz = 50f + (pitchRaw / 255f) * 450f
        val energy = energyRaw / 255f
        return VoiceFrame(voiced, pitchHz, energy, duration)
    }

    private fun synthesizeFrame(frame: VoiceFrame): ShortArray {
        val samples = ShortArray(frameSamples)
        val amplitude = (frame.energy * 0.6f).coerceIn(0f, 1f)
        val freq = if (frame.voiced) frame.pitchHz else 0f
        val step = if (freq > 0f) 2.0 * PI * freq / sampleRate else 0.0
        for (i in samples.indices) {
            val value = if (freq > 0f) sin(phase) else 0.0
            phase += step
            if (phase > 2.0 * PI) {
                phase -= 2.0 * PI
            }
            val sample = (value * amplitude * 32767.0).toInt().coerceIn(-32768, 32767)
            samples[i] = sample.toShort()
        }
        return samples
    }
}
