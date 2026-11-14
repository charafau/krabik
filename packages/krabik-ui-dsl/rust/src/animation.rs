//! Pure curve evaluation for the iOS frame animator.
//!
//! UIKit exposes its timing curves as native objects that cannot
//! be sampled from Rust. The display-link frame animator needs a value for
//! every tick, so the curves are evaluated here instead.

use crate::ui::AnimationCurve;

/// Evaluates one of the shared curves at a normalized time.
///
/// Springs use a damped-oscillator approximation; the result is clamped to a
/// small overshoot so a spring can visibly settle.
pub(crate) fn ease_curve(curve: &AnimationCurve, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match curve {
        AnimationCurve::Linear => t,
        AnimationCurve::EaseIn => t * t,
        AnimationCurve::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        AnimationCurve::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        AnimationCurve::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(t, *x1, *y1, *x2, *y2),
        AnimationCurve::Spring { damping, .. } => {
            let damping = damping.clamp(0.05, 1.0);
            let omega = 2.0 * std::f32::consts::PI * (1.0 + (1.0 - damping) * 1.2);
            let decay = (-damping * 12.0 * t).exp();
            let value = 1.0
                - decay
                    * ((omega * t).cos() + damping * (omega * t).sin() / (1.0 - damping).max(0.05));
            value.clamp(-0.25, 1.1)
        }
    }
}

/// Samples a cubic Bezier timing curve by solving its x coordinate.
///
/// The bisection is fixed-iteration, so the cost does not depend on how close
/// the control points are to the curve.
fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let curve = |a: f32, b: f32, value: f32| {
        let inverse = 1.0 - value;
        3.0 * inverse * inverse * value * a
            + 3.0 * inverse * value * value * b
            + value * value * value
    };
    let mut low = 0.0;
    let mut high = 1.0;
    let mut value = t;
    for _ in 0..18 {
        let x = curve(x1, x2, value);
        if x < t {
            low = value;
        } else {
            high = value;
        }
        value = (low + high) / 2.0;
    }
    curve(y1, y2, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn linear_curves_return_their_progress() {
        assert_eq!(ease_curve(&AnimationCurve::Linear, 0.25), 0.25);
        assert_eq!(ease_curve(&AnimationCurve::Linear, 1.5), 1.0);
        assert_eq!(ease_curve(&AnimationCurve::Linear, -1.0), 0.0);
    }

    #[test]
    fn standard_curves_start_and_end_at_the_edges() {
        for curve in [
            AnimationCurve::EaseIn,
            AnimationCurve::EaseOut,
            AnimationCurve::EaseInOut,
            AnimationCurve::CubicBezier {
                x1: 0.2,
                y1: 0.8,
                x2: 0.2,
                y2: 1.0,
            },
            AnimationCurve::spring(0.7, 0.35),
        ] {
            assert_close(ease_curve(&curve, 0.0), 0.0);
            assert_close(ease_curve(&curve, 1.0), 1.0);
        }
    }

    #[test]
    fn a_linear_bezier_matches_the_progress() {
        let curve = AnimationCurve::CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        };
        for t in [0.1, 0.25, 0.5, 0.75, 0.9] {
            assert_close(ease_curve(&curve, t), t);
        }
    }

    #[test]
    fn ease_in_starts_slower_than_ease_out() {
        assert!(ease_curve(&AnimationCurve::EaseIn, 0.25) < 0.25);
        assert!(ease_curve(&AnimationCurve::EaseOut, 0.25) > 0.25);
    }
}
