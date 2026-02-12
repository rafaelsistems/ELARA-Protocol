/**
 * ELARA Video Chat Sample Activity
 *
 * Demonstrates how to use ElaraVideoSession, ElaraVideoView,
 * ElaraViewRegistry, and ElaraBeautyFilter for a 1-on-1 video chat.
 *
 * This is a minimal example. In production you would add:
 * - A signaling server to exchange IP/port between peers
 * - UI for entering peer address or scanning QR code
 * - Proper permission handling with rationale dialogs
 */
package com.elara.app

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import com.elara.sdk.ElaraBeautyFilter
import com.elara.sdk.ElaraVideoSession
import com.elara.sdk.ElaraVideoView
import com.elara.sdk.ElaraViewRegistry

class VideoChatActivity : Activity() {

    companion object {
        private const val TAG = "VideoChatActivity"
        private const val PERMISSION_REQUEST = 200
    }

    private var session: ElaraVideoSession? = null
    private lateinit var localView: ElaraVideoView
    private lateinit var remoteView: ElaraVideoView
    private lateinit var statusText: TextView
    private lateinit var qualityText: TextView
    private val beautyFilter = ElaraBeautyFilter()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Build UI programmatically
        val root = FrameLayout(this)
        root.setBackgroundColor(Color.BLACK)

        // Remote video (full screen)
        remoteView = ElaraVideoView(this)
        root.addView(remoteView, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        ))

        // Local video (small overlay, top-right)
        localView = ElaraVideoView(this)
        localView.setMirrorMode(true)
        val localParams = FrameLayout.LayoutParams(240, 320)
        localParams.gravity = Gravity.TOP or Gravity.END
        localParams.topMargin = 48
        localParams.marginEnd = 16
        root.addView(localView, localParams)

        // Controls overlay (bottom)
        val controls = LinearLayout(this)
        controls.orientation = LinearLayout.HORIZONTAL
        controls.gravity = Gravity.CENTER
        controls.setBackgroundColor(Color.argb(128, 0, 0, 0))
        val controlParams = FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        )
        controlParams.gravity = Gravity.BOTTOM

        val toggleVideo = Button(this).apply { text = "Video" }
        val toggleAudio = Button(this).apply { text = "Audio" }
        val switchCam = Button(this).apply { text = "Flip" }
        val beautyBtn = Button(this).apply { text = "Beauty: 2" }
        val endCall = Button(this).apply { text = "End"; setTextColor(Color.RED) }

        toggleVideo.setOnClickListener {
            val enabled = session?.toggleVideo() ?: true
            toggleVideo.text = if (enabled) "Video" else "Video OFF"
        }
        toggleAudio.setOnClickListener {
            val enabled = session?.toggleAudio() ?: true
            toggleAudio.text = if (enabled) "Audio" else "Audio OFF"
        }
        switchCam.setOnClickListener { session?.switchCamera() }
        beautyBtn.setOnClickListener {
            beautyFilter.beautyLevel = (beautyFilter.beautyLevel + 1) % 4
            beautyBtn.text = "Beauty: ${beautyFilter.beautyLevel}"
        }
        endCall.setOnClickListener {
            session?.close()
            session = null
            finish()
        }

        controls.addView(toggleVideo)
        controls.addView(toggleAudio)
        controls.addView(switchCam)
        controls.addView(beautyBtn)
        controls.addView(endCall)
        root.addView(controls, controlParams)

        // Status text (top-left)
        val infoLayout = LinearLayout(this)
        infoLayout.orientation = LinearLayout.VERTICAL
        infoLayout.setPadding(16, 48, 16, 0)
        statusText = TextView(this).apply {
            setTextColor(Color.WHITE); textSize = 14f; text = "Status: Waiting"
        }
        qualityText = TextView(this).apply {
            setTextColor(Color.CYAN); textSize = 12f; text = "Quality: --"
        }
        infoLayout.addView(statusText)
        infoLayout.addView(qualityText)
        root.addView(infoLayout)

        setContentView(root)

        // Check permissions
        val needed = mutableListOf<String>()
        if (checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED)
            needed.add(Manifest.permission.CAMERA)
        if (checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED)
            needed.add(Manifest.permission.RECORD_AUDIO)

        if (needed.isEmpty()) {
            startSession()
        } else {
            requestPermissions(needed.toTypedArray(), PERMISSION_REQUEST)
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int, permissions: Array<out String>, grantResults: IntArray
    ) {
        if (requestCode == PERMISSION_REQUEST &&
            grantResults.all { it == PackageManager.PERMISSION_GRANTED }) {
            startSession()
        } else {
            statusText.text = "Status: Permissions denied"
        }
    }

    private fun startSession() {
        val sessionId = "sample-${System.currentTimeMillis()}"
        val localPeerId = "local-${android.os.Build.MODEL}"
        val remotePeerId = "remote-peer"

        session = ElaraVideoSession(sessionId, localPeerId, remotePeerId, this)
        session?.beautyFilter = beautyFilter

        // Register views for frame delivery
        ElaraViewRegistry.registerLocalView(sessionId, localView)
        ElaraViewRegistry.registerRemoteView(sessionId, remoteView)
        session?.frameListener = ElaraViewRegistry

        session?.setEventListener(object : ElaraVideoSession.EventListener {
            override fun onConnected() {
                runOnUiThread { statusText.text = "Status: Connected" }
            }
            override fun onDisconnected(reason: String) {
                runOnUiThread { statusText.text = "Status: Disconnected ($reason)" }
            }
            override fun onQualityChanged(quality: Int) {
                runOnUiThread { qualityText.text = "Quality: $quality%" }
            }
            override fun onMessage(message: String) {
                Log.d(TAG, "Message: $message")
            }
            override fun onRemoteFrame(bitmap: Bitmap) { }
            override fun onLocalFrame(bitmap: Bitmap) { }
        })

        // In a real app, you would:
        // 1. Get local port via session.getLocalPort()
        // 2. Exchange IP/port with peer via signaling server
        // 3. Call session.setRemoteAddress(peerIp, peerPort)
        // 4. Then call session.connect(callback)
        //
        // For this demo, we just start the session in loopback mode:
        statusText.text = "Status: Connecting..."
        session?.connect(object : ElaraVideoSession.ConnectCallback {
            override fun onSuccess() {
                Log.d(TAG, "Session connected, local port: ${session?.getLocalPort()}")
                // Loopback: set remote to localhost for testing
                // session?.setRemoteAddress("127.0.0.1", session?.getLocalPort() ?: 0)
            }
            override fun onError(error: String) {
                runOnUiThread { statusText.text = "Status: Error - $error" }
            }
        })
    }

    override fun onDestroy() {
        val sid = session?.sessionId
        session?.close()
        session = null
        // Clear registry to prevent leaking view/activity references
        if (sid != null) ElaraViewRegistry.clearSession(sid)
        super.onDestroy()
    }
}
