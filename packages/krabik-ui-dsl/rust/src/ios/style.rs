//! Decoration and accessibility shared by every view.

use crate::ios::{as_any, cg_color, color};
use crate::render::RenderNode;
use crate::tree::NodeKind;
use crate::ui::{Border, CornerRadii, Gradient, GradientDirection, ImageScale, dim_color};
use crate::widgets::Widget;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGMutablePath;
use objc2_foundation::{NSArray, NSString};
use objc2_quartz_core::{CACornerMask, CAGradientLayer, CAShapeLayer};
use objc2_ui_kit::NSObjectUIAccessibility;
use objc2_ui_kit::{UIColor, UIControl, UIView};

/// A mask that rounds each corner of a view by its own radius.
///
/// A layer's `cornerRadius` is one number for all four corners. When the
/// corners differ, the view is clipped by a shape instead; the shape has to
/// be refitted whenever the view changes size.
#[derive(Clone)]
pub(crate) struct CornerMask {
    layer: Retained<CAShapeLayer>,
    radii: CornerRadii,
}

impl CornerMask {
    /// Fits the shape to a view of `size`.
    pub(crate) fn fit(&self, size: CGSize) {
        let (width, height) = (size.width, size.height);
        // A radius cannot exceed half of the shorter side.
        let limit = (width.min(height) / 2.0).max(0.0);
        let radius = |value: f32| f64::from(value).clamp(0.0, limit);
        let radii = self.radii;
        let path = CGMutablePath::new();
        let transform = std::ptr::null();
        // SAFETY: `path` is a valid mutable path and a null transform means
        // "no transform". Each arc rounds the corner the previous edge ends at.
        unsafe {
            CGMutablePath::move_to_point(Some(&path), transform, radius(radii.top_left), 0.0);
            CGMutablePath::add_arc_to_point(
                Some(&path),
                transform,
                width,
                0.0,
                width,
                height,
                radius(radii.top_right),
            );
            CGMutablePath::add_arc_to_point(
                Some(&path),
                transform,
                width,
                height,
                0.0,
                height,
                radius(radii.bottom_right),
            );
            CGMutablePath::add_arc_to_point(
                Some(&path),
                transform,
                0.0,
                height,
                0.0,
                0.0,
                radius(radii.bottom_left),
            );
            CGMutablePath::add_arc_to_point(
                Some(&path),
                transform,
                0.0,
                0.0,
                width,
                0.0,
                radius(radii.top_left),
            );
        }
        CGMutablePath::close_subpath(Some(&path));
        self.layer
            .setFrame(CGRect::new(CGPoint::new(0.0, 0.0), size));
        self.layer.setPath(Some(&path));
    }
}

/// `radius`, but no more than half the shorter side of a view of `size`:
/// a pill, not a lens. A view that has no size yet keeps the radius.
fn fitted_radius(radius: f64, size: CGSize) -> f64 {
    let limit = size.width.min(size.height) / 2.0;
    if limit > 0.0 {
        radius.min(limit)
    } else {
        radius
    }
}

/// Applies the corner radius a view asks for at the size it now has.
pub(crate) fn fit_corner_radius(
    view: &UIView,
    gradient: Option<&CAGradientLayer>,
    radius: f64,
    size: CGSize,
) {
    if radius <= 0.0 {
        return;
    }
    let radius = fitted_radius(radius, size);
    view.layer().setCornerRadius(radius);
    if let Some(gradient) = gradient {
        gradient.setCornerRadius(radius);
    }
}

pub(crate) fn apply_style(
    view: &UIView,
    node: &RenderNode,
    gradient: &mut Option<Retained<CAGradientLayer>>,
    corner_mask: &mut Option<CornerMask>,
    corner_radius: &mut f64,
) {
    let style = &node.style;
    // Native text and controls dim themselves; only the decoration the DSL
    // paints is dimmed here.
    let enabled = style.enabled.unwrap_or(true);
    let background = style.background.map(|value| dim_color(value, enabled));
    let gradient_style = style.gradient.map(|gradient| Gradient {
        start: dim_color(gradient.start, enabled),
        end: dim_color(gradient.end, enabled),
        ..gradient
    });
    let border_style = style.border.map(|border| Border {
        color: dim_color(border.color, enabled),
        ..border
    });
    let elevation = if enabled { style.elevation } else { 0.0 };
    let layer = view.layer();

    let radii = style.corner_radii();
    let mut radius = f64::from(
        radii
            .top_left
            .max(radii.top_right)
            .max(radii.bottom_right)
            .max(radii.bottom_left),
    );
    let mut corners = CACornerMask::empty();
    if radii.top_left > 0.0 {
        corners |= CACornerMask::LayerMinXMinYCorner;
    }
    if radii.top_right > 0.0 {
        corners |= CACornerMask::LayerMaxXMinYCorner;
    }
    if radii.bottom_right > 0.0 {
        corners |= CACornerMask::LayerMaxXMaxYCorner;
    }
    if radii.bottom_left > 0.0 {
        corners |= CACornerMask::LayerMinXMaxYCorner;
    }
    // Corners that are rounded by different amounts need a shape. A border
    // and a shadow are drawn by the layer along its own (single-radius)
    // outline and would not follow the shape, so a view that has either
    // keeps the largest radius on every rounded corner.
    let rounded: Vec<f32> = [
        radii.top_left,
        radii.top_right,
        radii.bottom_right,
        radii.bottom_left,
    ]
    .into_iter()
    .filter(|value| *value > 0.0)
    .collect();
    let uneven = rounded
        .windows(2)
        .any(|pair| (pair[0] - pair[1]).abs() > 0.01);
    if uneven && border_style.is_none() && elevation <= 0.0 {
        let mask = corner_mask.get_or_insert_with(|| CornerMask {
            layer: CAShapeLayer::new(),
            radii,
        });
        mask.radii = radii;
        mask.fit(view.bounds().size);
        // SAFETY: the mask layer is not part of any layer tree.
        unsafe { layer.setMask(Some(&mask.layer)) };
        radius = 0.0;
    } else if corner_mask.take().is_some() {
        // SAFETY: removing a mask is always valid.
        unsafe { layer.setMask(None) };
    }
    // The radius the description asks for. A layer given more than half its
    // shorter side draws nothing sensible, and the size is not known until
    // layout, so `fit_corner_radius` applies it again with every frame.
    *corner_radius = radius;
    let radius = fitted_radius(radius, view.bounds().size);
    layer.setCornerRadius(radius);
    layer.setMaskedCorners(corners);

    let fills_frame = matches!(&node.kind, NodeKind::Widget(Widget::Image(props)) if props.scale == ImageScale::Fill);
    let clips = matches!(&node.kind, NodeKind::Scroll(_) | NodeKind::Recycler(_))
        || fills_frame
        || (radius > 0.0 && elevation <= 0.0);
    view.setClipsToBounds(clips);

    if gradient_style.is_some() {
        view.setBackgroundColor(None);
    } else {
        view.setBackgroundColor(background.map(color).as_deref());
    }

    if let Some(shape) = gradient_style {
        let gradient_layer = gradient.get_or_insert_with(CAGradientLayer::new);
        if gradient_layer.superlayer().is_none() {
            view.layer().insertSublayer_atIndex(gradient_layer, 0);
        }

        // `CAGradientLayer.colors` is an `NSArray<id>` of toll-free bridged
        // `CGColor` objects, so the elements must be typed as `AnyObject` for
        // the runtime to see the `^@` element encoding.
        let start: Retained<AnyObject> = unsafe { Retained::cast_unchecked(cg_color(shape.start)) };
        let end: Retained<AnyObject> = unsafe { Retained::cast_unchecked(cg_color(shape.end)) };
        let colors: Retained<NSArray> = NSArray::from_retained_slice(&[start, end]);
        // SAFETY: the array holds two valid `CGColor` objects.
        unsafe { gradient_layer.setColors(Some(&colors)) };
        match shape.direction {
            GradientDirection::Horizontal => {
                gradient_layer.setStartPoint(CGPoint::new(0.0, 0.5));
                gradient_layer.setEndPoint(CGPoint::new(1.0, 0.5));
            }
            GradientDirection::Vertical => {
                gradient_layer.setStartPoint(CGPoint::new(0.5, 0.0));
                gradient_layer.setEndPoint(CGPoint::new(0.5, 1.0));
            }
        }
        gradient_layer.setCornerRadius(radius);
        gradient_layer.setMaskedCorners(corners);
        gradient_layer.setMasksToBounds(true);
        gradient_layer.setFrame(view.bounds());
    } else if let Some(gradient_layer) = gradient.take() {
        gradient_layer.removeFromSuperlayer();
    }

    if let Some(border) = border_style {
        let border_color = cg_color(border.color);
        layer.setBorderWidth(f64::from(border.width));
        layer.setBorderColor(Some(&border_color));
    } else {
        layer.setBorderWidth(0.0);
        layer.setBorderColor(None);
    }

    view.setAlpha(f64::from(style.opacity.unwrap_or(1.0)));

    if elevation > 0.0 {
        // SAFETY: `CGColor` is retained for as long as the layer keeps it.
        let shadow = unsafe { UIColor::blackColor().CGColor() };
        layer.setShadowColor(Some(&shadow));
        layer.setShadowOpacity(0.18);
        layer.setShadowRadius(f64::from(elevation));
        layer.setShadowOffset(CGSize::new(0.0, f64::from(elevation / 2.0)));
    } else {
        layer.setShadowOpacity(0.0);
    }
}

/// Applies interaction state and accessibility metadata to a native view.
pub(crate) fn apply_accessibility(view: &UIView, node: &RenderNode, mtm: MainThreadMarker) {
    if let Some(label) = &node.style.accessibility_label {
        let label = NSString::from_str(label);
        view.setAccessibilityLabel(Some(&label), mtm);
        // A radio group must stay a plain container so its options remain
        // individually reachable by accessibility services. A text field is
        // already exposed by UIKit as an editable element; forcing the flag
        // would turn it into a plain labelled view.
        if !matches!(node.kind, NodeKind::Widget(Widget::TextField(_))) {
            view.setIsAccessibilityElement(
                !matches!(
                    node.kind,
                    NodeKind::Widget(Widget::RadioGroup(_)) | NodeKind::Widget(Widget::TabBar(_))
                ),
                mtm,
            );
        }
    } else {
        view.setAccessibilityLabel(None, mtm);
    }

    let enabled = node.style.enabled.unwrap_or(true);
    view.setUserInteractionEnabled(enabled);
    if let Some(control) = as_any(view).downcast_ref::<UIControl>() {
        control.setEnabled(enabled);
    }
}
