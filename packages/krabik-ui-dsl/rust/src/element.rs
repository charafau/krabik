//! The declarative UI description: [`Element`] and the builder macros every
//! widget uses for the shared layout and decoration methods.

use crate::style::{FontWeight, Style};
use crate::widgets::Widget;
use crate::widgets::{Axis, ListProps, PagerProps, ScrollProps};
use std::borrow::Cow;

/// The component-specific part of an [`Element`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ElementKind {
    Column,
    Row,
    Spacer,
    Pager(PagerProps),
    /// A leaf widget; see [`Widget`].
    Widget(Widget),
    Scroll(ScrollProps),
    List(ListProps),
}

impl ElementKind {
    /// A stable tag used when hashing node identifiers.
    ///
    /// The scroll axis is part of the tag: switching it rebuilds the subtree,
    /// because the native scroll container changes class.
    pub(crate) fn tag(&self) -> u64 {
        match self {
            Self::Column => 1,
            Self::Row => 2,
            Self::Spacer => 3,
            Self::Pager(_) => 20,
            Self::Widget(widget) => widget.tag(),
            Self::Scroll(props) => match props.axis {
                Axis::Vertical => 8,
                Axis::Horizontal => 9,
            },
            Self::List(_) => 11,
        }
    }
}

/// A node of the declarative UI description.
///
/// Components such as [`Column`] and [`Text`] convert into an `Element` and can
/// therefore be nested with [`Column::child`] and [`Row::child`].
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub(crate) kind: ElementKind,
    pub(crate) style: Style,
    pub(crate) key: Option<Cow<'static, str>>,
    pub(crate) children: Vec<Element>,
}

impl Element {
    /// Creates a container element.
    pub(crate) fn container(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            key: None,
            children: Vec::new(),
        }
    }

    /// Creates a leaf element.
    pub(crate) fn leaf(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            key: None,
            children: Vec::new(),
        }
    }

    /// The children of the element.
    pub fn children(&self) -> &[Element] {
        &self.children
    }

    /// The key that identifies the element among its siblings.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

/// Adds the shared style builder methods to a component.
macro_rules! style_builder {
    ($component:ty) => {
        impl $component {
            /// Sets uniform padding.
            pub fn padding(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.padding = $crate::style::EdgeInsets::all(value.into_points());
                self
            }

            /// Sets padding on each side.
            pub fn padding_each(mut self, value: $crate::style::EdgeInsets) -> Self {
                self.0.style.padding = value;
                self
            }

            /// Sets uniform margin.
            pub fn margin(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.margin = $crate::style::EdgeInsets::all(value.into_points());
                self
            }

            /// Sets margin on each side.
            pub fn margin_each(mut self, value: $crate::style::EdgeInsets) -> Self {
                self.0.style.margin = value;
                self
            }

            /// Sets a fixed width in logical points.
            pub fn width(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.width = $crate::style::Length::Points(value.into_points());
                self
            }

            /// Sets the width as a fraction of the parent's width.
            pub fn width_percent(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.width = $crate::style::Length::Percent(value.into_points());
                self
            }

            /// Sets a fixed height in logical points.
            pub fn height(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.height = $crate::style::Length::Points(value.into_points());
                self
            }

            /// Sets the height as a fraction of the parent's height.
            pub fn height_percent(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.height = $crate::style::Length::Percent(value.into_points());
                self
            }

            /// Sets the width-to-height ratio.
            pub fn aspect_ratio(mut self, ratio: f32) -> Self {
                self.0.style.aspect_ratio = Some(ratio);
                self
            }

            /// Lets the node grow into the remaining main-axis space.
            pub fn grow(mut self, value: impl $crate::style::IntoPoints) -> Self {
                self.0.style.grow = value.into_points();
                self
            }

            /// Sets the cross-axis alignment of the node's children.
            pub fn align_items(mut self, value: $crate::style::Align) -> Self {
                self.0.style.align_items = value;
                self
            }

            /// Sets the main-axis distribution of the node's children.
            pub fn justify_content(mut self, value: $crate::style::Justify) -> Self {
                self.0.style.justify_content = value;
                self
            }

            /// Sets the background color as ARGB.
            pub fn background(mut self, color: u32) -> Self {
                self.0.style.background = Some(color);
                self
            }

            /// Sets a two-color background gradient.
            pub fn gradient(mut self, gradient: $crate::style::Gradient) -> Self {
                self.0.style.gradient = Some(gradient);
                self
            }

            /// Rounds the corners of the node's background and border.
            pub fn corner_radius(mut self, radius: impl $crate::style::IntoPoints) -> Self {
                self.0.style.corner_radius = radius.into_points();
                self
            }

            /// Rounds each corner of the node independently.
            pub fn corner_radii(mut self, radii: $crate::style::CornerRadii) -> Self {
                self.0.style.corner_radii = Some(radii);
                self
            }

            /// Draws a border around the node.
            pub fn border(mut self, width: impl $crate::style::IntoPoints, color: u32) -> Self {
                self.0.style.border = Some($crate::style::Border::new(width.into_points(), color));
                self
            }

            /// Sets the opacity of the node and its children.
            pub fn opacity(mut self, opacity: f32) -> Self {
                self.0.style.opacity = Some(opacity.clamp(0.0, 1.0));
                self
            }

            /// Raises the node above its siblings with a native shadow.
            pub fn elevation(mut self, elevation: impl $crate::style::IntoPoints) -> Self {
                self.0.style.elevation = elevation.into_points().max(0.0);
                self
            }

            /// Animates this node's frame and opacity changes.
            ///
            /// A duration of zero makes subsequent changes snap.
            pub fn animate(mut self, duration_ms: u32) -> Self {
                self.0.style.animation = Some($crate::style::Animation::new(duration_ms));
                self
            }

            /// Sets the full implicit animation.
            pub fn animation(mut self, animation: $crate::style::Animation) -> Self {
                self.0.style.animation = Some(animation);
                self
            }

            /// Replaces the timing curve, keeping the current duration.
            pub fn animation_curve(mut self, curve: $crate::style::AnimationCurve) -> Self {
                let animation = self.0.style.animation.unwrap_or_default().curve(curve);
                self.0.style.animation = Some(animation);
                self
            }

            /// Fades and scales the node in when it is created.
            pub fn animate_enter(mut self, duration_ms: u32) -> Self {
                self.0.style.enter = Some($crate::style::Transition::new(duration_ms));
                self
            }

            /// Fades and scales the node out before it is removed.
            pub fn animate_exit(mut self, duration_ms: u32) -> Self {
                self.0.style.exit = Some($crate::style::Transition::new(duration_ms));
                self
            }

            /// Replaces the enter transition's timing curve.
            pub fn enter_curve(mut self, curve: $crate::style::AnimationCurve) -> Self {
                let transition = self.0.style.enter.take().unwrap_or_default().curve(curve);
                self.0.style.enter = Some(transition);
                self
            }

            /// Replaces the exit transition's timing curve.
            pub fn exit_curve(mut self, curve: $crate::style::AnimationCurve) -> Self {
                let transition = self.0.style.exit.take().unwrap_or_default().curve(curve);
                self.0.style.exit = Some(transition);
                self
            }

            /// Enables or disables interaction with the node.
            pub fn enabled(mut self, enabled: bool) -> Self {
                self.0.style.enabled = Some(enabled);
                self
            }

            /// Sets the label announced by accessibility services.
            pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
                self.0.style.accessibility_label = Some(label.into());
                self
            }

            /// Identifies the node among its siblings across rebuilds.
            ///
            /// Keys must be unique among siblings and stable across rebuilds.
            /// Both static strings and owned `String`s are accepted, which lets
            /// list rows derive their key from the item they represent.
            pub fn key(mut self, key: impl Into<std::borrow::Cow<'static, str>>) -> Self {
                self.0.key = Some(key.into());
                self
            }
        }
    };
}

/// Adds the shared text styling methods to a text-bearing component.
macro_rules! text_builder {
    ($component:ty) => {
        impl $component {
            /// Sets the font size in logical points.
            pub fn size(mut self, size: impl $crate::style::IntoPoints) -> Self {
                self.0.set_text_size(size.into_points());
                self
            }

            /// Sets the ARGB text color.
            pub fn color(mut self, color: u32) -> Self {
                self.0.set_text_color(color);
                self
            }

            /// Renders the text in bold.
            pub fn bold(mut self) -> Self {
                self.0.set_text_weight($crate::style::FontWeight::Bold);
                self
            }
        }
    };
}

impl Element {
    pub(crate) fn set_text_size(&mut self, size: f32) {
        match &mut self.kind {
            ElementKind::Widget(Widget::Text(props)) => props.size = size,
            ElementKind::Widget(Widget::Button(props)) => props.size = size,
            ElementKind::Widget(Widget::TextField(props)) => props.size = size,
            ElementKind::Widget(Widget::Checkbox(props))
            | ElementKind::Widget(Widget::RadioButton(props)) => props.size = size,
            ElementKind::Widget(Widget::RadioGroup(props)) => props.size = size,
            ElementKind::Widget(Widget::TabBar(props)) => props.size = size,
            ElementKind::Widget(Widget::DatePicker(props)) => props.size = size,
            _ => {}
        }
    }

    pub(crate) fn set_text_color(&mut self, color: u32) {
        match &mut self.kind {
            ElementKind::Widget(Widget::Text(props)) => props.color = color,
            ElementKind::Widget(Widget::Button(props)) => props.color = color,
            ElementKind::Widget(Widget::TextField(props)) => props.color = color,
            ElementKind::Widget(Widget::Checkbox(props))
            | ElementKind::Widget(Widget::RadioButton(props)) => props.color = color,
            ElementKind::Widget(Widget::RadioGroup(props)) => props.color = color,
            ElementKind::Widget(Widget::TabBar(props)) => props.color = color,
            ElementKind::Widget(Widget::DatePicker(props)) => props.color = color,
            _ => {}
        }
    }

    pub(crate) fn set_text_weight(&mut self, weight: FontWeight) {
        match &mut self.kind {
            ElementKind::Widget(Widget::Text(props)) => props.weight = weight,
            ElementKind::Widget(Widget::Button(props)) => props.weight = weight,
            ElementKind::Widget(Widget::TextField(props)) => props.weight = weight,
            ElementKind::Widget(Widget::Checkbox(props))
            | ElementKind::Widget(Widget::RadioButton(props)) => props.weight = weight,
            ElementKind::Widget(Widget::RadioGroup(props)) => props.weight = weight,
            ElementKind::Widget(Widget::TabBar(props)) => props.weight = weight,
            ElementKind::Widget(Widget::DatePicker(props)) => props.weight = weight,
            _ => {}
        }
    }
}

pub(crate) use style_builder;
pub(crate) use text_builder;
