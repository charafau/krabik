//! The `chip` widget: a compact, rounded label that can be picked or removed.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Icon, Justify, Length};
use crate::widgets::{Container, Image, Row, Text};

/// The background of a selected chip: the accent, mostly transparent.
const SELECTED: Color = Color::adaptive(0x3321_96F3, 0x4064_B5F6);

/// A compact, rounded label: a filter that can be switched on, a tag that
/// can be removed, a suggestion that can be tapped.
///
/// ```ignore
/// Wrap::new().spacing(8).run_spacing(8)
///     .child(Chip::new("Rust").selected(self.rust).on_tap(Msg::ToggleRust))
///     .child(Chip::new("draft").on_delete(Msg::RemoveTag(3)))
/// ```
///
/// A selected chip is filled and shows a check mark. The state is the
/// application's: `on_tap` only reports the tap. Several chips in a
/// [`Wrap`](crate::ui::Wrap) are how a choice of many is offered.
pub struct Chip<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

struct Parts {
    label: String,
    icon: Option<Icon>,
    selected: bool,
    on_tap: Handler<()>,
    on_delete: Handler<()>,
}

impl<M> Chip<M> {
    /// Creates a chip with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.height = Length::Points(32.0);
        shell.style.corner_radius = 16.0;
        shell.style.padding = EdgeInsets::symmetric(12.0, 0.0);
        shell.style.justify_content = Justify::Center;
        Self(
            shell,
            PhantomData,
            Parts {
                label: label.into(),
                icon: None,
                selected: false,
                on_tap: Handler::none(),
                on_delete: Handler::none(),
            },
        )
    }

    /// Shows an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.2.icon = Some(icon);
        self
    }

    /// Shows the chip as picked.
    pub fn selected(mut self, selected: bool) -> Self {
        self.2.selected = selected;
        self
    }
}

impl<M: Clone + Send + Sync + 'static> Chip<M> {
    /// Sends `message` when the chip is tapped.
    pub fn on_tap(mut self, message: M) -> Self {
        self.2.on_tap = Handler::new(move |()| message.clone());
        self
    }

    /// Shows a remove button after the label that sends `message`.
    pub fn on_delete(mut self, message: M) -> Self {
        self.2.on_delete = Handler::new(move |()| message.clone());
        self
    }
}

impl<M: 'static> From<Chip<M>> for Element<M> {
    fn from(component: Chip<M>) -> Self {
        let Chip(mut shell, _, parts) = component;
        // The look follows the state unless the application styled the chip.
        if shell.style.background.is_none() && shell.style.border.is_none() {
            if parts.selected {
                shell.style.background = Some(SELECTED);
            } else {
                shell.style.border = Some(crate::style::Border::new(1.0, Color::OUTLINE));
            }
        }
        let mut row = Row::<M>::new().spacing(6).align_items(Align::Center);
        if parts.selected {
            row = row.child(Image::icon(Icon::check()).size(16).key("check"));
        } else if let Some(icon) = parts.icon {
            row = row.child(Image::icon(icon).size(16).key("icon"));
        }
        row = row.child(Text::new(parts.label.clone()).size(14).key("label"));
        if parts.on_delete.is_set() {
            row = row.child(
                Container::new()
                    .handler(parts.on_delete)
                    .accessibility_label(format!("Remove {}", parts.label))
                    .key("delete")
                    .child(Image::icon(Icon::close()).size(16)),
            );
        }
        finish(shell, Container::new().handler(parts.on_tap).child(row))
    }
}

style_builder!(Chip);
