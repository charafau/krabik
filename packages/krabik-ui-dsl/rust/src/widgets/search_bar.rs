//! The `search_bar` widget: a rounded field with a search icon and a clear button.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Icon, Justify, Length};
use crate::widgets::{Container, IconButton, Image, ImeAction, Row, TextField};

/// A rounded text field for searching: a magnifying glass, the text, and a
/// button that clears it while there is any.
///
/// ```ignore
/// SearchBar::new(&self.query)
///     .placeholder("Search notes")
///     .on_change(Msg::Query)
///     .on_submit(Msg::Search)
/// ```
///
/// The clear button reports an empty text through `on_change`. The
/// keyboard's action key is "Search".
pub struct SearchBar<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts<M>,
);

struct Parts<M> {
    value: String,
    placeholder: String,
    on_change: Option<Arc<dyn Fn(String) -> M + Send + Sync>>,
    on_submit: Handler<()>,
}

impl<M> SearchBar<M> {
    /// Creates a search bar that shows `value`.
    pub fn new(value: impl Into<String>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.height = Length::Points(48.0);
        shell.style.corner_radius = 24.0;
        shell.style.background = Some(Color::SURFACE);
        shell.style.padding = EdgeInsets {
            top: 0.0,
            right: 4.0,
            bottom: 0.0,
            left: 14.0,
        };
        shell.style.justify_content = Justify::Center;
        Self(
            shell,
            PhantomData,
            Parts {
                value: value.into(),
                placeholder: String::new(),
                on_change: None,
                on_submit: Handler::none(),
            },
        )
    }

    /// Sets the hint shown while the bar is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.2.placeholder = placeholder.into();
        self
    }

    /// Reports the text whenever it changes, and an empty text when it is
    /// cleared.
    pub fn on_change(mut self, message: impl Fn(String) -> M + Send + Sync + 'static) -> Self {
        self.2.on_change = Some(Arc::new(message));
        self
    }

    /// Sends `message` when the keyboard's search key is pressed.
    pub fn on_submit(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.2.on_submit = Handler::new(move |()| message.clone());
        self
    }
}

impl<M: Clone + Send + Sync + 'static> From<SearchBar<M>> for Element<M> {
    fn from(component: SearchBar<M>) -> Self {
        let SearchBar(shell, _, parts) = component;
        let has_text = !parts.value.is_empty();
        let mut field = TextField::<M>::new(parts.value)
            .placeholder(parts.placeholder)
            .ime_action(ImeAction::Search)
            .grow(1)
            .key("field");
        if let Some(on_change) = &parts.on_change {
            let on_change = Arc::clone(on_change);
            field = field.on_change(move |text| on_change(text));
        }
        let mut field = Element::from(field).into_raw();
        if let ElementKind::Widget(crate::widgets::Widget::TextField(props)) = &mut field.kind {
            props.on_submit = parts.on_submit;
        }
        let mut row = Row::<M>::new()
            .spacing(10)
            .align_items(Align::Center)
            .child(
                Image::icon(Icon::search())
                    .size(20)
                    .tint(Color::SECONDARY_TEXT)
                    .key("icon"),
            )
            .child(Element::from_raw(field));
        match parts.on_change {
            Some(on_change) if has_text => {
                row = row.child(
                    IconButton::new(Icon::close(), "Clear")
                        .icon_size(20)
                        .tint(Color::SECONDARY_TEXT)
                        .handler(Handler::new(move |()| on_change(String::new())))
                        .key("clear"),
                );
            }
            // Keeps the field clear of the rounded end.
            _ => row = row.child(Container::new().width(10).key("end")),
        }
        finish(shell, Container::new().child(row))
    }
}

style_builder!(SearchBar);
