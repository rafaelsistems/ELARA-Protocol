/**
 * ELARA Beauty Filter
 *
 * ML-powered face beauty filter using Google ML Kit Face Detection.
 * Designed for real-time video chat with minimal CPU overhead.
 *
 * How it works:
 * 1. ML Kit detects face bounding box + landmarks (eyes, nose, mouth)
 * 2. Skin smoothing applied ONLY to face region (not entire frame)
 * 3. Eye brightening around eye landmarks
 * 4. Skin tone warmth + brightness enhancement on face area
 * 5. Face detection runs async every few frames to stay smooth
 *
 * Performance strategy:
 * - Face detection runs every 3rd frame (async, non-blocking)
 * - Beauty effect uses last known face position for in-between frames
 * - GPU-accelerated Canvas operations (no CPU pixel loops)
 * - Works on all devices: low-end to high-end
 *
 * Usage:
 *   val filter = ElaraBeautyFilter()
 *   filter.beautyLevel = 2  // 0=off, 1=light, 2=medium, 3=heavy
 *   val beautified = filter.apply(sourceBitmap)
 */
package com.elara.sdk

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.ColorMatrix
import android.graphics.ColorMatrixColorFilter
import android.graphics.Paint
import android.graphics.PointF
import android.graphics.RectF
import android.util.Log
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.face.FaceDetection
import com.google.mlkit.vision.face.FaceDetectorOptions
import com.google.mlkit.vision.face.FaceLandmark

/**
 * Real-time face beauty filter powered by Google ML Kit.
 *
 * All processing happens on-device. No data is sent to any server.
 */
class ElaraBeautyFilter {

    companion object {
        private const val TAG = "ElaraBeautyFilter"
    }

    /** Beauty level: 0 = off, 1 = light, 2 = medium, 3 = heavy */
    @Volatile
    var beautyLevel: Int = 2

    // ML Kit face detector (lazy init)
    private val detector by lazy {
        val options = FaceDetectorOptions.Builder()
            .setPerformanceMode(FaceDetectorOptions.PERFORMANCE_MODE_FAST)
            .setLandmarkMode(FaceDetectorOptions.LANDMARK_MODE_ALL)
            .setMinFaceSize(0.15f)
            .build()
        FaceDetection.getClient(options)
    }

    // Last detected face data (used between detection frames)
    @Volatile private var lastFaceRect: RectF? = null
    @Volatile private var lastLeftEye: PointF? = null
    @Volatile private var lastRightEye: PointF? = null
    @Volatile private var lastNose: PointF? = null
    @Volatile private var hasFace: Boolean = false

    // Frame counter for async detection scheduling
    private var frameCount = 0
    private var detectingNow = false

    // Reusable Paint objects
    private val smoothPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    private val colorPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    private val eyePaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)

    /**
     * Run ML face detection asynchronously every N frames.
     * Updates lastFaceRect/landmarks for use by apply().
     */
    private fun scheduleDetection(bitmap: Bitmap) {
        frameCount++
        if (detectingNow || (frameCount % 3 != 0 && hasFace)) return
        detectingNow = true

        try {
            val image = InputImage.fromBitmap(bitmap, 0)
            detector.process(image)
                .addOnSuccessListener { faces ->
                    if (faces.isNotEmpty()) {
                        val face = faces[0]
                        val bounds = face.boundingBox
                        lastFaceRect = RectF(
                            bounds.left.toFloat(),
                            bounds.top.toFloat(),
                            bounds.right.toFloat(),
                            bounds.bottom.toFloat()
                        )
                        lastLeftEye = face.getLandmark(FaceLandmark.LEFT_EYE)?.position
                        lastRightEye = face.getLandmark(FaceLandmark.RIGHT_EYE)?.position
                        lastNose = face.getLandmark(FaceLandmark.NOSE_BASE)?.position
                        hasFace = true
                    } else {
                        hasFace = false
                        lastFaceRect = null
                    }
                    detectingNow = false
                }
                .addOnFailureListener {
                    detectingNow = false
                }
        } catch (e: Exception) {
            detectingNow = false
        }
    }

    /**
     * Apply beauty filter to a bitmap.
     * Returns a new bitmap. Caller should recycle input if not needed.
     *
     * @param src Source bitmap (camera frame)
     * @return Beautified bitmap (new instance)
     */
    fun apply(src: Bitmap): Bitmap {
        if (beautyLevel == 0) return src

        val w = src.width
        val h = src.height

        // Schedule async face detection
        scheduleDetection(src)

        // Start with original
        val output = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(output)
        canvas.drawBitmap(src, 0f, 0f, null)

        // === STEP 1: Global subtle color enhancement ===
        val globalBrightness = when (beautyLevel) {
            1 -> 5f; 2 -> 8f; else -> 12f
        }
        val globalContrast = when (beautyLevel) {
            1 -> 1.01f; 2 -> 1.02f; else -> 1.03f
        }
        val cm = ColorMatrix(floatArrayOf(
            globalContrast, 0f, 0f, 0f, globalBrightness + 2f,  // R: slight warm
            0f, globalContrast, 0f, 0f, globalBrightness,        // G
            0f, 0f, globalContrast, 0f, globalBrightness - 1f,   // B: slight warm
            0f, 0f, 0f, 1f, 0f
        ))
        val colorOutput = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        val colorCanvas = Canvas(colorOutput)
        colorPaint.colorFilter = ColorMatrixColorFilter(cm)
        colorCanvas.drawBitmap(output, 0f, 0f, colorPaint)
        output.recycle()

        // === STEP 2: Face-targeted beauty (only if face detected) ===
        val faceRect = lastFaceRect
        if (faceRect != null && hasFace) {
            val expandX = faceRect.width() * 0.15f
            val expandY = faceRect.height() * 0.15f
            val faceLeft = (faceRect.left - expandX).coerceAtLeast(0f).toInt()
            val faceTop = (faceRect.top - expandY).coerceAtLeast(0f).toInt()
            val faceRight = (faceRect.right + expandX).coerceAtMost(w.toFloat()).toInt()
            val faceBottom = (faceRect.bottom + expandY).coerceAtMost(h.toFloat()).toInt()
            val faceW = faceRight - faceLeft
            val faceH = faceBottom - faceTop

            if (faceW > 10 && faceH > 10) {
                // Extract face region
                val faceBitmap = Bitmap.createBitmap(colorOutput, faceLeft, faceTop, faceW, faceH)

                // Smooth face via downscale-upscale (GPU bilinear = natural blur)
                val downFactor = when (beautyLevel) {
                    1 -> 3; 2 -> 4; else -> 5
                }
                val smallW = (faceW / downFactor).coerceAtLeast(2)
                val smallH = (faceH / downFactor).coerceAtLeast(2)
                val small = Bitmap.createScaledBitmap(faceBitmap, smallW, smallH, true)
                val smoothFace = Bitmap.createScaledBitmap(small, faceW, faceH, true)
                small.recycle()

                // Blend smoothed face onto output with alpha
                val blendAlpha = when (beautyLevel) {
                    1 -> 55; 2 -> 80; else -> 105
                }
                smoothPaint.alpha = blendAlpha
                val outCanvas = Canvas(colorOutput)
                outCanvas.drawBitmap(smoothFace, faceLeft.toFloat(), faceTop.toFloat(), smoothPaint)
                smoothFace.recycle()
                faceBitmap.recycle()

                // === STEP 3: Eye brightening ===
                val leftEye = lastLeftEye
                val rightEye = lastRightEye
                if (leftEye != null && rightEye != null) {
                    val eyeRadius = faceRect.width() * 0.08f
                    val eyeBrightness = when (beautyLevel) {
                        1 -> 8f; 2 -> 12f; else -> 16f
                    }
                    val eyeCm = ColorMatrix(floatArrayOf(
                        1.05f, 0f, 0f, 0f, eyeBrightness,
                        0f, 1.05f, 0f, 0f, eyeBrightness,
                        0f, 0f, 1.05f, 0f, eyeBrightness,
                        0f, 0f, 0f, 1f, 0f
                    ))
                    eyePaint.colorFilter = ColorMatrixColorFilter(eyeCm)
                    eyePaint.alpha = when (beautyLevel) {
                        1 -> 80; 2 -> 120; else -> 160
                    }

                    for (eye in listOf(leftEye, rightEye)) {
                        val ex = eye.x.toInt()
                        val ey = eye.y.toInt()
                        val er = eyeRadius.toInt()
                        val eLeft = (ex - er).coerceAtLeast(0)
                        val eTop = (ey - er).coerceAtLeast(0)
                        val eRight = (ex + er).coerceAtMost(w)
                        val eBottom = (ey + er).coerceAtMost(h)
                        val eW = eRight - eLeft
                        val eH = eBottom - eTop
                        if (eW > 2 && eH > 2) {
                            val eyeRegion = Bitmap.createBitmap(colorOutput, eLeft, eTop, eW, eH)
                            outCanvas.drawBitmap(eyeRegion, eLeft.toFloat(), eTop.toFloat(), eyePaint)
                            eyeRegion.recycle()
                        }
                    }
                }
            }
        }

        return colorOutput
    }
}
