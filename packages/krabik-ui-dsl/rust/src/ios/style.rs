//! Decoration and accessibility shared by every view.

use crate::ios::{as_any, cg_color, color};
use crate::render::RenderNode;
use crate::tree::NodeKind;
use crate::ui::{Border, Gradient, GradientDirection, ImageScale, dim_color};
use crate::widgets::Widget;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_foundation::{NSArray, NSString};
use objc2_quartz_core::{CACornerMask, CAGradientLayer};
use objc2_ui_kit::NSObjectUIAccessibility;
use objc2_ui_kit::{UIColor, UIControl, UIView};

pub(crate) fn apply_style(
    view: &UIView,
    node: &RenderNode,
    gradient: &mut Option<Retained<CAGradientLayer>>,
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
    let radius = f64::from(
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
