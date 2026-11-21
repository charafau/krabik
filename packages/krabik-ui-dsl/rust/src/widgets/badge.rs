//! The `badge` widget: a count or a dot on the corner of something.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::style::{Align, Color, EdgeInsets, Inset, Justify};
use crate::widgets::{Container, Stack, Text};

/// A small mark on the top right corner of another element: a count, a
/// short label, or a dot.
///
/// ```ignore
/// Badge::new(Image::icon(Icon::new().drawable("ic_inbox").sf("tray")))
///     .count(self.unread)
/// ```
///
/// A count of zero shows no mark, and one above 99 shows "99+". The mark
/// sits a little outside the element, so leave it a few points of room.
///
/// The badge is as large as its child. A column stretches its children to
/// its width, which would put the mark at the far edge; in a column, wrap
/// the badge in a [`Row`](crate::ui::Row).
pub struct Badge<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

struct Parts {
    child: RawElement,
    mark: Mark,
    color: Color,
}

enum Mark {
    None,
    Dot,
    Label(String),
}

impl<M> Badge<M> {
    /// Creates a badge on `child`, with no mark yet.
    pub fn new(child: impl Into<Element<M>>) -> Self {
        let mut shell = RawElement::container(ElementKind::Stack);
        shell.style.align_items = Align::Start;
        Self(
            shell,
            PhantomData,
            Parts {
                child: child.into().into_raw(),
                mark: Mark::None,
                color: Color::fixed(0xFFD3_2F2F),
            },
        )
    }

    /// Shows a number. Zero shows nothing.
    pub fn count(mut self, count: usize) -> Self {
        self.2.mark = match count {
            0 => Mark::None,
            1..=99 => Mark::Label(count.to_string()),
            _ => Mark::Label("99+".to_owned()),
        };
        self
    }

    /// Shows a short text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.2.mark = Mark::Label(label.into());
        self
    }

    /// Shows a dot while `visible`.
    pub fn dot(mut self, visible: bool) -> Self {
        self.2.mark = if visible { Mark::Dot } else { Mark::None };
        self
    }

    /// Sets the color of the mark; red unless set.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.2.color = color.into();
        self
    }
}

impl<M: 'static> From<Badge<M>> for Element<M> {
    fn from(component: Badge<M>) -> Self {
        let Badge(shell, _, parts) = component;
        let mut stack = Stack::<M>::new().child(Element::from_raw(parts.child));
        let mark = match parts.mark {
            Mark::None => None,
            Mark::Dot => Some(
                Container::<M>::new()
                    .width(8)
                    .height(8)
                    .corner_radius(4)
                    .positioned(Inset::new().top(-2).right(-2)),
            ),
            Mark::Label(label) => Some(
                Container::new()
                    .min_width(16)
                    .height(16)
                    .corner_radius(8)
                    .padding_each(EdgeInsets::symmetric(4.0, 0.0))
                    .align_items(Align::Center)
                    .justify_content(Justify::Center)
                    .positioned(Inset::new().top(-6).right(-8))
                    .child(
                        Text::new(label)
                            .size(11)
                            .bold()
                            .color(Color::fixed(0xFFFF_FFFF)),
                    ),
            ),
        };
        if let Some(mark) = mark {
            stack = stack.child(mark.background(parts.color).key("badge"));
        }
        finish(shell, stack)
    }
}

style_builder!(Badge);
