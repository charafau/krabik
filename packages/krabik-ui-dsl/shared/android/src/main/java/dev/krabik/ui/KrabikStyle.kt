package dev.krabik.ui

import android.animation.ArgbEvaluator
import android.animation.ValueAnimator
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import android.view.animation.AccelerateInterpolator
import android.view.animation.DecelerateInterpolator
import android.view.animation.Interpolator
import android.view.animation.LinearInterpolator
import android.view.animation.PathInterpolator
import java.util.WeakHashMap
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * Applies the platform-independent style properties to native views.
 *
 * Keeping drawable construction here keeps the Rust renderer to a single JNI
 * call per styled node and preserves the native ripple of buttons. When the
 * node opts into an animation, the same values are interpolated by a
 * [ValueAnimator] instead of being applied immediately.
 */
object KrabikStyle {
    private val states = WeakHashMap<View, ViewStyle>()

    @JvmStatic
    fun apply(
        view: View,
        backgroundColor: Int,
        hasBackground: Boolean,
        gradientStart: Int,
        gradientEnd: Int,
        hasGradient: Boolean,
        gradientHorizontal: Boolean,
        topLeftRadius: Float,
        topRightRadius: Float,
        bottomRightRadius: Float,
        bottomLeftRadius: Float,
        borderWidth: Float,
        borderColor: Int,
        hasBorder: Boolean,
        opacity: Float,
        ripple: Boolean,
        elevation: Float,
        animationDuration: Int,
        curveKind: Int,
        p1: Int,
        p2: Int,
        p3: Int,
        p4: Int,
    ) {
        val target = StyleValues(
            backgroundColor = backgroundColor,
            hasBackground = hasBackground,
            gradientStart = gradientStart,
            gradientEnd = gradientEnd,
            hasGradient = hasGradient,
            gradientHorizontal = gradientHorizontal,
            topLeft = topLeftRadius,
            topRight = topRightRadius,
            bottomRight = bottomRightRadius,
            bottomLeft = bottomLeftRadius,
            borderWidth = borderWidth,
            borderColor = borderColor,
            hasBorder = hasBorder,
            alpha = opacity.coerceIn(0f, 1f),
            elevation = elevation,
        )

        val style = states.getOrPut(view) { ViewStyle() }
        // A cancelled animator must not apply its old target; the new
        // animation starts from the last rendered frame instead.
        style.animator?.let {
            it.removeAllListeners()
            it.cancel()
        }
        style.animator = null

        val from = style.current
        if (animationDuration <= 0 || from == null) {
            applyValues(view, style, target, ripple)
            style.current = target
            return
        }

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = animationDuration.toLong()
            interpolator = interpolator(curveKind, p1, p2, p3, p4)
            addUpdateListener { animation ->
                val fraction = animation.animatedValue as Float
                val value = interpolate(from, target, fraction)
                applyValues(view, style, value, ripple)
                style.current = value
            }
        }
        animator.addListener(
            object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: android.animation.Animator) {
                    applyValues(view, style, target, ripple)
                    style.current = target
                    if (style.animator === animation) {
                        style.animator = null
                    }
                }
            },
        )
        style.animator = animator
        animator.start()
    }

    /** Cancels a view's style animation and forgets its state. */
    @JvmStatic
    fun cancel(view: View) {
        states.remove(view)?.animator?.let {
            it.removeAllListeners()
            it.cancel()
        }
    }

    /** The native interpolator for a packed animation curve. */
    @JvmStatic
    fun interpolator(kind: Int, p1: Int, p2: Int, p3: Int, p4: Int): Interpolator =
        when (kind) {
            1 -> PathInterpolator(
                (p1 / SCALE).coerceIn(0f, 1f),
                p2 / SCALE,
                (p3 / SCALE).coerceIn(0f, 1f),
                p4 / SCALE,
            )
            2 -> SpringInterpolator(p1 / SCALE, p2 / SCALE)
            else -> when (p1) {
                1 -> LinearInterpolator()
                2 -> AccelerateInterpolator()
                3 -> DecelerateInterpolator()
                else -> AccelerateDecelerateInterpolator()
            }
        }

    private fun applyValues(view: View, style: ViewStyle, values: StyleValues, ripple: Boolean) {
        val density = view.resources.displayMetrics.density
        fun radius(value: Float): Float = (value * density).coerceAtLeast(0f)
        val radii = floatArrayOf(
            radius(values.topLeft),
            radius(values.topLeft),
            radius(values.topRight),
            radius(values.topRight),
            radius(values.bottomRight),
            radius(values.bottomRight),
            radius(values.bottomLeft),
            radius(values.bottomLeft),
        )
        val hasShape = values.hasBackground || values.hasGradient || values.hasBorder ||
            radii.any { it > 0f } || values.elevation > 0f

        if (!hasShape) {
            style.shape = null
            style.mask = null
            style.ripple = null
            restoreBackground(view, style)
        } else {
            rememberBackground(view, style)
            val shape = style.shape ?: GradientDrawable().apply {
                this.shape = GradientDrawable.RECTANGLE
                style.shape = this
            }
            if (values.hasGradient) {
                shape.colors = intArrayOf(values.gradientStart, values.gradientEnd)
                shape.orientation = if (values.gradientHorizontal) {
                    GradientDrawable.Orientation.LEFT_RIGHT
                } else {
                    GradientDrawable.Orientation.TOP_BOTTOM
                }
            } else {
                shape.setColor(if (values.hasBackground) values.backgroundColor else Color.TRANSPARENT)
            }
            shape.cornerRadii = radii
            if (values.hasBorder && values.borderWidth > 0f) {
                shape.setStroke(
                    (values.borderWidth.coerceAtLeast(0f) * density).toInt().coerceAtLeast(1),
                    values.borderColor,
                )
            } else {
                shape.setStroke(0, Color.TRANSPARENT)
            }

            if (ripple) {
                val mask = style.mask ?: GradientDrawable().apply {
                    this.shape = GradientDrawable.RECTANGLE
                    setColor(Color.WHITE)
                    style.mask = this
                }
                mask.cornerRadii = radii
                val wrapper = style.ripple ?: RippleDrawable(
                    ColorStateList.valueOf(rippleColor(view.context)),
                    shape,
                    mask,
                ).also { style.ripple = it }
                if (view.background !== wrapper) {
                    view.background = wrapper
                }
            } else if (view.background !== shape) {
                view.background = shape
            }
            view.clipToOutline = radii.any { it > 0f }
        }

        view.alpha = values.alpha.coerceIn(0f, 1f)
        view.elevation = (values.elevation * density).coerceAtLeast(0f)
    }

    private fun rememberBackground(view: View, style: ViewStyle) {
        if (!style.replaced) {
            style.original = view.background
            style.replaced = true
        }
    }

    private fun restoreBackground(view: View, style: ViewStyle) {
        if (style.replaced) {
            view.background = style.original
            style.original = null
            style.replaced = false
        }
    }

    private fun interpolate(from: StyleValues, to: StyleValues, fraction: Float): StyleValues {
        val evaluator = ArgbEvaluator()
        fun color(a: Int, b: Int): Int = evaluator.evaluate(fraction, a, b) as Int
        fun lerp(a: Float, b: Float): Float = a + (b - a) * fraction
        return StyleValues(
            backgroundColor = color(
                if (from.hasBackground) from.backgroundColor else Color.TRANSPARENT,
                if (to.hasBackground) to.backgroundColor else Color.TRANSPARENT,
            ),
            hasBackground = from.hasBackground || to.hasBackground,
            gradientStart = color(
                if (from.hasGradient) from.gradientStart else Color.TRANSPARENT,
                if (to.hasGradient) to.gradientStart else Color.TRANSPARENT,
            ),
            gradientEnd = color(
                if (from.hasGradient) from.gradientEnd else Color.TRANSPARENT,
                if (to.hasGradient) to.gradientEnd else Color.TRANSPARENT,
            ),
            hasGradient = from.hasGradient || to.hasGradient,
            gradientHorizontal = to.gradientHorizontal,
            topLeft = lerp(from.topLeft, to.topLeft),
            topRight = lerp(from.topRight, to.topRight),
            bottomRight = lerp(from.bottomRight, to.bottomRight),
            bottomLeft = lerp(from.bottomLeft, to.bottomLeft),
            borderWidth = lerp(from.borderWidth, to.borderWidth),
            borderColor = color(
                if (from.hasBorder) from.borderColor else Color.TRANSPARENT,
                if (to.hasBorder) to.borderColor else Color.TRANSPARENT,
            ),
            hasBorder = from.hasBorder || to.hasBorder,
            alpha = lerp(from.alpha, to.alpha),
            elevation = lerp(from.elevation, to.elevation),
        )
    }

    private class ViewStyle {
        var current: StyleValues? = null
        var animator: ValueAnimator? = null
        var shape: GradientDrawable? = null
        var mask: GradientDrawable? = null
        var ripple: RippleDrawable? = null
        var original: Drawable? = null
        var replaced = false
    }

    private data class StyleValues(
        val backgroundColor: Int,
        val hasBackground: Boolean,
        val gradientStart: Int,
        val gradientEnd: Int,
        val hasGradient: Boolean,
        val gradientHorizontal: Boolean,
        val topLeft: Float,
        val topRight: Float,
        val bottomRight: Float,
        val bottomLeft: Float,
        val borderWidth: Float,
        val borderColor: Int,
        val hasBorder: Boolean,
        val alpha: Float,
        val elevation: Float,
    )

    /** A damped-spring timing curve evaluated in normalized time. */
    private class SpringInterpolator(
        damping: Float,
        response: Float,
    ) : Interpolator {
        private val zeta = damping.coerceIn(0.01f, 1f)
        private val response = response.coerceAtLeast(0.01f)

        override fun getInterpolation(input: Float): Float {
            if (input <= 0f) return 0f
            if (input >= 1f) return 1f

            val omega = (2.0 * PI / response).toFloat()
            val time = input * response
            val decay = exp(-zeta * omega * time)
            if (zeta >= 0.999f) {
                return 1f - decay * (1f + omega * time)
            }
            val damped = omega * sqrt(1f - zeta * zeta)
            return 1f - decay *
                (cos(damped * time) + (zeta * omega / damped) * sin(damped * time))
        }
    }

    /** The theme's press highlight, so the ripple suits light and dark. */
    private fun rippleColor(context: android.content.Context): Int {
        val value = android.util.TypedValue()
        val found = context.theme.resolveAttribute(
            android.R.attr.colorControlHighlight,
            value,
            true,
        )
        return if (found && value.resourceId != 0) {
            context.getColor(value.resourceId)
        } else if (found) {
            value.data
        } else {
            0x1F000000
        }
    }

    private const val SCALE = 10_000f
}
