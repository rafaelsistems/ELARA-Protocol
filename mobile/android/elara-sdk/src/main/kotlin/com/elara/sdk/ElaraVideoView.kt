/**
 * ELARA Video View
 *
 * A native Android View that renders video frames directly via Canvas.
 * Designed for high-performance frame rendering without intermediate layers.
 *
 * Features:
 * - Cover mode scaling (fills view, crops excess)
 * - Mirror mode for front camera preview
 * - Thread-safe frame updates from any thread
 * - Anti-aliased bitmap rendering with bilinear filtering
 *
 * Usage:
 *   val videoView = ElaraVideoView(context)
 *   // From any thread:
 *   videoView.updateFrame(bitmap)
 */
package com.elara.sdk

import android.content.Context
import android.graphics.*
import android.util.AttributeSet
import android.view.View

/**
 * Native View that renders ELARA video frames directly via Canvas.
 * Frames are pushed from the session thread and rendered on the UI thread.
 */
class ElaraVideoView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    private var currentBitmap: Bitmap? = null
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    private val srcRect = Rect()
    private val dstRect = RectF()
    private var mirror = false

    /**
     * Update the displayed frame. Can be called from any thread.
     * The view will request a redraw on the UI thread.
     */
    fun updateFrame(bitmap: Bitmap) {
        currentBitmap = bitmap
        postInvalidate()
    }

    /**
     * Enable or disable horizontal mirror mode.
     * Typically enabled for local front camera preview.
     */
    fun setMirrorMode(enabled: Boolean) {
        mirror = enabled
        postInvalidate()
    }

    /**
     * Clear the current frame, showing a blank view.
     */
    fun clearFrame() {
        currentBitmap = null
        postInvalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val bmp = currentBitmap ?: return

        srcRect.set(0, 0, bmp.width, bmp.height)

        // Cover mode: scale to fill, crop excess
        val viewRatio = width.toFloat() / height.toFloat()
        val bmpRatio = bmp.width.toFloat() / bmp.height.toFloat()

        if (bmpRatio > viewRatio) {
            // Bitmap wider than view — crop sides
            val scaledW = height * bmpRatio
            val offsetX = (width - scaledW) / 2f
            dstRect.set(offsetX, 0f, offsetX + scaledW, height.toFloat())
        } else {
            // Bitmap taller than view — crop top/bottom
            val scaledH = width / bmpRatio
            val offsetY = (height - scaledH) / 2f
            dstRect.set(0f, offsetY, width.toFloat(), offsetY + scaledH)
        }

        if (mirror) {
            canvas.save()
            canvas.scale(-1f, 1f, width / 2f, height / 2f)
        }

        canvas.drawBitmap(bmp, srcRect, dstRect, paint)

        if (mirror) {
            canvas.restore()
        }
    }
}

/**
 * Registry that connects ElaraVideoSession instances to ElaraVideoView instances.
 * Allows sessions to push frames directly to views without intermediate layers.
 *
 * Usage:
 *   // Register views
 *   ElaraViewRegistry.registerLocalView(sessionId, localVideoView)
 *   ElaraViewRegistry.registerRemoteView(sessionId, remoteVideoView)
 *
 *   // Session pushes frames automatically via FrameListener
 *   session.frameListener = ElaraViewRegistry
 *
 *   // Cleanup
 *   ElaraViewRegistry.clearSession(sessionId)
 */
object ElaraViewRegistry : ElaraVideoSession.FrameListener {

    private val localViews = mutableMapOf<String, MutableList<ElaraVideoView>>()
    private val remoteViews = mutableMapOf<String, MutableList<ElaraVideoView>>()

    fun registerLocalView(sessionId: String, view: ElaraVideoView) {
        synchronized(this) {
            localViews.getOrPut(sessionId) { mutableListOf() }.let { list ->
                if (!list.contains(view)) list.add(view)
            }
        }
    }

    fun registerRemoteView(sessionId: String, view: ElaraVideoView) {
        synchronized(this) {
            remoteViews.getOrPut(sessionId) { mutableListOf() }.let { list ->
                if (!list.contains(view)) list.add(view)
            }
        }
    }

    fun unregisterView(view: ElaraVideoView) {
        synchronized(this) {
            localViews.values.forEach { it.remove(view) }
            remoteViews.values.forEach { it.remove(view) }
        }
    }

    fun clearSession(sessionId: String) {
        synchronized(this) {
            localViews.remove(sessionId)
            remoteViews.remove(sessionId)
        }
    }

    override fun onLocalFrame(sessionId: String, bitmap: Bitmap) {
        synchronized(this) {
            localViews[sessionId]?.forEach { it.updateFrame(bitmap) }
        }
    }

    override fun onRemoteFrame(sessionId: String, bitmap: Bitmap) {
        synchronized(this) {
            remoteViews[sessionId]?.forEach { it.updateFrame(bitmap) }
        }
    }
}
