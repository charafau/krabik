//! Frame and decoration animations.

use crate::animation::ease_curve;
use crate::geometry::Size;
use crate::ios::inset;
use crate::render::RenderNode;
use crate::ui::{Animation, AnimationCurve, EdgeInsets};
use block2::RcBlock;
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::{CFTimeInterval, CGPoint, CGRect, CGSize, CGVector};
use objc2_core_graphics::CGColor;
use objc2_foundation::{NSArray, NSNumber, NSRunLoop, NSRunLoopCommonModes, NSString, NSValue};
use objc2_quartz_core::CAMediaTiming;
use objc2_quartz_core::{
    CABasicAnimation, CACurrentMediaTime, CADisplayLink, CAGradientLayer, CALayer,
    CAMediaTimingFunction, kCAMediaTimingFunctionEaseIn, kCAMediaTimingFunctionEaseInEaseOut,
    kCAMediaTimingFunctionEaseOut, kCAMediaTimingFunctionLinear,
};
use objc2_ui_kit::{
    UIAccessibilityIsReduceMotionEnabled, UIColor, UICubicTimingParameters, UIScrollView,
    UISpringTimingParameters, UITimingCurveProvider, UIView, UIViewAnimating,
    UIViewAnimatingPosition, UIViewAnimationCurve, UIViewPropertyAnimator,
};
use std::ptr::NonNull;

/// The views and frames updated by one layout operation.
///
/// The values are owned so the same work can run directly or inside a UIKit
/// animation block.
pub(crate) struct LayoutFrames {
    pub(crate) view: Retained<UIView>,
    pub(crate) inset: Option<Retained<UIView>>,
    pub(crate) host: Option<Retained<UIView>>,
    pub(crate) scroll: Option<(Retained<UIScrollView>, Retained<UIView>, Size)>,
    pub(crate) gradient: Option<Retained<CAGradientLayer>>,
    pub(crate) padding: EdgeInsets,
}

impl LayoutFrames {
    /// Applies the frames with the view at `rect`.
    ///
    /// Frame animations call this on every display-link tick, so the model
    /// frames always match what the user sees and hit-testing stays exact.
    pub(crate) fn apply_rect(&self, rect: CGRect) {
        self.view.setFrame(rect);

        if let Some(content) = &self.inset {
            content.setFrame(inset(rect.size, self.padding));
        }
        if let Some(host) = &self.host {
            host.setFrame(inset(rect.size, self.padding));
        }
        if let Some((scroll, host, size)) = &self.scroll {
            host.setFrame(CGRect::new(
                CGPoint::new(f64::from(self.padding.left), f64::from(self.padding.top)),
                CGSize::new(
                    f64::from(size.width.max(0.0)),
                    f64::from(size.height.max(0.0)),
                ),
            ));
            scroll.setContentSize(CGSize::new(
                f64::from(self.padding.left + size.width + self.padding.right),
                f64::from(self.padding.top + size.height + self.padding.bottom),
            ));
        }
        if let Some(gradient) = &self.gradient {
            gradient.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), rect.size));
        }
    }
}

/// Interpolates between two frames.
pub(crate) fn lerp_rect(from: CGRect, to: CGRect, progress: f64) -> CGRect {
    let lerp = |a: f64, b: f64| a + (b - a) * progress;
    CGRect::new(
        CGPoint::new(
            lerp(from.origin.x, to.origin.x),
            lerp(from.origin.y, to.origin.y),
        ),
        CGSize::new(
            lerp(from.size.width, to.size.width),
            lerp(from.size.height, to.size.height),
        ),
    )
}

/// Drives a frame animation from the display link.
pub(crate) struct FrameTickerIvars {
    pub(crate) frames: LayoutFrames,
    pub(crate) from: CGRect,
    pub(crate) to: CGRect,
    pub(crate) start: CFTimeInterval,
    pub(crate) duration: f64,
    pub(crate) curve: AnimationCurve,
}

objc2::define_class!(
    // The ticker and its retained views live on the main thread.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikFrameTicker"]
    #[ivars = FrameTickerIvars]
    pub(crate) struct FrameTicker;

    unsafe impl NSObjectProtocol for FrameTicker {}

    impl FrameTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &CADisplayLink) {
            let ivars = self.ivars();
            let progress = if ivars.duration <= 0.0 {
                1.0
            } else {
                ((link.timestamp() - ivars.start) / ivars.duration).clamp(0.0, 1.0)
            };
            let eased = f64::from(ease_curve(&ivars.curve, progress as f32));
            ivars
                .frames
                .apply_rect(lerp_rect(ivars.from, ivars.to, eased));
            if progress >= 1.0 {
                link.invalidate();
            }
        }
    }
);

impl FrameTicker {
    /// Starts a frame tween and returns the display link that drives it.
    ///
    /// The link retains the ticker for as long as it runs, so the caller only
    /// keeps the link in order to cancel it.
    pub(crate) fn start(
        frames: LayoutFrames,
        from: CGRect,
        to: CGRect,
        duration_ms: u32,
        curve: AnimationCurve,
        mtm: MainThreadMarker,
    ) -> Retained<CADisplayLink> {
        let ticker = Self::alloc(mtm).set_ivars(FrameTickerIvars {
            frames,
            from,
            to,
            start: CACurrentMediaTime(),
            duration: f64::from(duration_ms) / 1000.0,
            curve,
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        let ticker: Retained<Self> = unsafe { objc2::msg_send![super(ticker), init] };
        // SAFETY: the ticker implements `tick:` and is retained by the link.
        let link =
            unsafe { CADisplayLink::displayLinkWithTarget_selector(&ticker, objc2::sel!(tick:)) };
        // SAFETY: the run loop and the static common mode both outlive the link.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        link
    }
}

/// Whether a transition with this duration should run on this device.
pub(crate) fn animation_is_enabled(duration_ms: u32) -> bool {
    duration_ms > 0 && !UIAccessibilityIsReduceMotionEnabled()
}

/// Builds UIKit timing parameters for one of the shared curves.
pub(crate) fn timing_parameters(
    curve: &AnimationCurve,
    mtm: MainThreadMarker,
) -> Retained<ProtocolObject<dyn UITimingCurveProvider>> {
    match curve {
        AnimationCurve::Linear => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::Linear,
            ))
        }
        AnimationCurve::EaseIn => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseIn,
            ))
        }
        AnimationCurve::EaseOut => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseOut,
            ))
        }
        AnimationCurve::EaseInOut => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseInOut,
            ))
        }
        AnimationCurve::CubicBezier { x1, y1, x2, y2 } => ProtocolObject::from_retained(
            UICubicTimingParameters::initWithControlPoint1_controlPoint2(
                UICubicTimingParameters::alloc(mtm),
                CGPoint::new(f64::from(*x1), f64::from(*y1)),
                CGPoint::new(f64::from(*x2), f64::from(*y2)),
            ),
        ),
        AnimationCurve::Spring { damping, response } => {
            let response = f64::from(*response).max(0.01);
            let stiffness = (2.0 * std::f64::consts::PI / response).powi(2);
            let damping = 2.0 * f64::from(*damping) * stiffness.sqrt();
            ProtocolObject::from_retained(
                UISpringTimingParameters::initWithMass_stiffness_damping_initialVelocity(
                    UISpringTimingParameters::alloc(mtm),
                    1.0,
                    stiffness,
                    damping,
                    CGVector::new(0.0, 0.0),
                ),
            )
        }
    }
}

/// Runs a UIKit property animation and optionally calls `completion`.
pub(crate) fn run_animation(
    duration_ms: u32,
    curve: &AnimationCurve,
    animations: RcBlock<dyn Fn()>,
    completion: Option<RcBlock<dyn Fn(UIViewAnimatingPosition)>>,
    mtm: MainThreadMarker,
) {
    let parameters = timing_parameters(curve, mtm);
    let animator = UIViewPropertyAnimator::initWithDuration_timingParameters(
        UIViewPropertyAnimator::alloc(mtm),
        f64::from(duration_ms) / 1000.0,
        &parameters,
    );
    animator.addAnimations(&animations);
    if let Some(completion) = completion {
        let block = RcBlock::new(move |position: UIViewAnimatingPosition| {
            completion.call((position,));
        });
        animator.addCompletion(&block);
    }
    UIViewAnimating::startAnimation(&*animator);
}

/// The Core Animation timing function for a curve.
pub(crate) fn timing_function(curve: &AnimationCurve) -> Retained<CAMediaTimingFunction> {
    match curve {
        AnimationCurve::Linear => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionLinear })
        }
        AnimationCurve::EaseIn => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseIn })
        }
        AnimationCurve::EaseOut => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseOut })
        }
        AnimationCurve::EaseInOut => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseInEaseOut })
        }
        AnimationCurve::CubicBezier { x1, y1, x2, y2 } => {
            CAMediaTimingFunction::functionWithControlPoints(*x1, *y1, *x2, *y2)
        }
        // Core Animation has no spring timing function in these bindings;
        // layer-property changes fall back to the default ease.
        AnimationCurve::Spring { .. } => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseInEaseOut })
        }
    }
}

/// Adds a Core Animation interpolation from `from` to `to` on a layer key.
pub(crate) fn add_layer_animation(
    layer: &CALayer,
    key_path: &str,
    from: Option<&AnyObject>,
    to: Option<&AnyObject>,
    duration_ms: u32,
    curve: &AnimationCurve,
) {
    let animation = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str(key_path)));
    // SAFETY: the values are toll-free bridged or boxed objects matching the
    // key path's value type.
    unsafe {
        animation.setFromValue(from);
        animation.setToValue(to);
    }
    animation.setDuration(f64::from(duration_ms) / 1000.0);
    animation.setTimingFunction(Some(&timing_function(curve)));
    layer.addAnimation_forKey(&animation, Some(&NSString::from_str(key_path)));
}

pub(crate) struct StyleSnapshot {
    pub(crate) background: Option<Retained<UIColor>>,
    pub(crate) border_color: Option<Retained<CGColor>>,
    pub(crate) border_width: f64,
    pub(crate) corner_radius: f64,
    pub(crate) gradient_colors: Option<Retained<NSArray>>,
    pub(crate) gradient_start: Option<CGPoint>,
    pub(crate) gradient_end: Option<CGPoint>,
    pub(crate) shadow_opacity: f32,
    pub(crate) shadow_radius: f64,
    pub(crate) shadow_offset: CGSize,
    pub(crate) alpha: f64,
}

pub(crate) fn style_snapshot(
    view: &Retained<UIView>,
    gradient: Option<&Retained<CAGradientLayer>>,
) -> StyleSnapshot {
    let layer = view.layer();
    StyleSnapshot {
        background: view.backgroundColor(),
        border_color: layer.borderColor(),
        border_width: layer.borderWidth(),
        corner_radius: layer.cornerRadius(),
        gradient_colors: gradient.and_then(|gradient| gradient.colors()),
        gradient_start: gradient.map(|gradient| gradient.startPoint()),
        gradient_end: gradient.map(|gradient| gradient.endPoint()),
        shadow_opacity: layer.shadowOpacity(),
        shadow_radius: layer.shadowRadius(),
        shadow_offset: layer.shadowOffset(),
        alpha: view.alpha(),
    }
}

/// Boxes a point for a Core Animation value.
pub(crate) fn box_point(point: CGPoint) -> Retained<NSValue> {
    // SAFETY: both pointers are valid for the duration of the call; the type
    // encoding matches the pointed-to `CGPoint`.
    unsafe {
        NSValue::valueWithBytes_objCType(
            NonNull::new_unchecked((&point as *const CGPoint).cast_mut().cast()),
            NonNull::new_unchecked(c"{CGPoint=dd}".as_ptr().cast_mut()),
        )
    }
}

/// Boxes a size for a Core Animation value.
pub(crate) fn box_size(size: CGSize) -> Retained<NSValue> {
    // SAFETY: both pointers are valid for the duration of the call; the type
    // encoding matches the pointed-to `CGSize`.
    unsafe {
        NSValue::valueWithBytes_objCType(
            NonNull::new_unchecked((&size as *const CGSize).cast_mut().cast()),
            NonNull::new_unchecked(c"{CGSize=dd}".as_ptr().cast_mut()),
        )
    }
}

/// Animates a style update: background and alpha through UIKit, layer
/// properties through Core Animation.
pub(crate) fn animate_style_change(
    view: &Retained<UIView>,
    gradient: Option<&Retained<CAGradientLayer>>,
    node: &RenderNode,
    snapshot: StyleSnapshot,
    animation: &Animation,
    mtm: MainThreadMarker,
) {
    let target_background = view.backgroundColor();
    let target_alpha = f64::from(node.style.opacity.unwrap_or(1.0));

    view.setBackgroundColor(snapshot.background.as_deref());
    view.setAlpha(snapshot.alpha);

    let view_for_block = view.clone();
    let block = RcBlock::new(move || {
        view_for_block.setBackgroundColor(target_background.as_deref());
        view_for_block.setAlpha(target_alpha);
    });
    run_animation(animation.duration_ms, &animation.curve, block, None, mtm);

    let layer = view.layer();

    let from_color = snapshot
        .border_color
        .map(|color| unsafe { Retained::cast_unchecked(color) });
    let to_color = layer
        .borderColor()
        .map(|color| unsafe { Retained::cast_unchecked(color) });
    add_layer_animation(
        &layer,
        "borderColor",
        from_color.as_deref(),
        to_color.as_deref(),
        animation.duration_ms,
        &animation.curve,
    );

    let from_width: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.border_width)) };
    let to_width: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.borderWidth())) };
    add_layer_animation(
        &layer,
        "borderWidth",
        Some(&from_width),
        Some(&to_width),
        animation.duration_ms,
        &animation.curve,
    );

    let from_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.corner_radius)) };
    let to_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.cornerRadius())) };
    add_layer_animation(
        &layer,
        "cornerRadius",
        Some(&from_radius),
        Some(&to_radius),
        animation.duration_ms,
        &animation.curve,
    );

    if let (Some(from_colors), Some(gradient)) = (snapshot.gradient_colors, gradient)
        && let Some(to_colors) = gradient.colors()
    {
        let from_colors: Retained<AnyObject> = unsafe { Retained::cast_unchecked(from_colors) };
        let to_colors: Retained<AnyObject> = unsafe { Retained::cast_unchecked(to_colors) };
        add_layer_animation(
            gradient,
            "colors",
            Some(&from_colors),
            Some(&to_colors),
            animation.duration_ms,
            &animation.curve,
        );

        // Gradient direction is expressed by its endpoints, so animating them
        // turns a direction change into a smooth sweep.
        if let Some(from) = snapshot.gradient_start {
            let from = box_point(from);
            let to = box_point(gradient.startPoint());
            add_layer_animation(
                gradient,
                "startPoint",
                Some(&from),
                Some(&to),
                animation.duration_ms,
                &animation.curve,
            );
        }
        if let Some(from) = snapshot.gradient_end {
            let from = box_point(from);
            let to = box_point(gradient.endPoint());
            add_layer_animation(
                gradient,
                "endPoint",
                Some(&from),
                Some(&to),
                animation.duration_ms,
                &animation.curve,
            );
        }
    }

    let from_shadow_opacity: Retained<AnyObject> = unsafe {
        Retained::cast_unchecked(NSNumber::numberWithDouble(f64::from(
            snapshot.shadow_opacity,
        )))
    };
    let to_shadow_opacity: Retained<AnyObject> = unsafe {
        Retained::cast_unchecked(NSNumber::numberWithDouble(f64::from(layer.shadowOpacity())))
    };
    add_layer_animation(
        &layer,
        "shadowOpacity",
        Some(&from_shadow_opacity),
        Some(&to_shadow_opacity),
        animation.duration_ms,
        &animation.curve,
    );

    let from_shadow_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.shadow_radius)) };
    let to_shadow_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.shadowRadius())) };
    add_layer_animation(
        &layer,
        "shadowRadius",
        Some(&from_shadow_radius),
        Some(&to_shadow_radius),
        animation.duration_ms,
        &animation.curve,
    );

    let from_shadow_offset: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(box_size(snapshot.shadow_offset)) };
    let to_shadow_offset: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(box_size(layer.shadowOffset())) };
    add_layer_animation(
        &layer,
        "shadowOffset",
        Some(&from_shadow_offset),
        Some(&to_shadow_offset),
        animation.duration_ms,
        &animation.curve,
    );
}
