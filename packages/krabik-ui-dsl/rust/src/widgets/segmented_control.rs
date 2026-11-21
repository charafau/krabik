//! The `segmented_control` widget: a few options side by side, one picked.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, FontWeight, Justify};
use crate::widgets::{Container, Row, Text};

/// Two to five options side by side, of which one is picked.
///
/// ```ignore
/// SegmentedControl::new(["Day", "Week", "Month"])
///     .selected(self.range)
///     .on_select(Msg::Range)
/// ```
///
/// The segments share the width equally. For views that are swiped between,
/// use [`TabsView`](crate::ui::TabsView); this is for a setting.
pub struct SegmentedControl<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts<M>,
);

struct Parts<M> {
    options: Vec<String>,
    selected: usize,
    on_select: Option<Arc<dyn Fn(usize) -> M + Send + Sync>>,
}

impl<M> SegmentedControl<M> {
    /// Creates a control over the given options, with the first picked.
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut shell = RawElement::container(ElementKind::Row);
        shell.style.padding = EdgeInsets::all(2.0);
        shell.style.gap = 2.0;
        shell.style.corner_radius = 9.0;
        shell.style.background = Some(Color::SURFACE);
        Self(
            shell,
            PhantomData,
            Parts {
                options: options.into_iter().map(Into::into).collect(),
                selected: 0,
                on_select: None,
            },
        )
    }

    /// Sets which option is picked, counting from zero.
    pub fn selected(mut self, index: usize) -> Self {
        self.2.selected = index;
        self
    }

    /// Reports the index of the option that was tapped.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self {
        self.2.on_select = Some(Arc::new(message));
        self
    }
}

impl<M: 'static> From<SegmentedControl<M>> for Element<M> {
    fn from(component: SegmentedControl<M>) -> Self {
        let SegmentedControl(shell, _, parts) = component;
        let mut row = Row::<M>::new();
        for (index, option) in parts.options.into_iter().enumerate() {
            let picked = index == parts.selected;
            let handler = match &parts.on_select {
                // Tapping the picked segment again changes nothing.
                Some(on_select) if !picked => {
                    let on_select = Arc::clone(on_select);
                    Handler::new(move |()| on_select(index))
                }
                _ => Handler::none(),
            };
            let mut segment = Container::<M>::new()
                .handler(handler)
                .grow(1)
                .width_percent(0.0)
                .min_width(0)
                .height(32)
                .corner_radius(7)
                .align_items(Align::Center)
                .justify_content(Justify::Center)
                .accessibility_label(option.clone())
                .key(format!("segment-{index}"));
            if picked {
                segment = segment.background(Color::BACKGROUND).elevation(1);
            }
            let label = Text::new(option).size(14);
            row = row.child(segment.child(if picked {
                label.weight(FontWeight::SemiBold)
            } else {
                label
            }));
        }
        finish(shell, row)
    }
}

style_builder!(SegmentedControl);
