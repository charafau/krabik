//! The `autocomplete` widget: a text field with suggestions under it.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Color, EdgeInsets};
use crate::widgets::{Column, Container, Divider, Text, TextField};

/// A text field that offers the options matching what was typed.
///
/// ```ignore
/// Autocomplete::new(&self.city, CITIES.iter().copied())
///     .placeholder("City")
///     .on_change(Msg::City)
/// ```
///
/// An option is offered while it contains the text, ignoring case, and is
/// not the text itself; at most five are shown. Tapping one reports it
/// through `on_select`, or through `on_change` when `on_select` is not set.
///
/// The suggestions are part of the layout: they push what follows down
/// instead of floating over it.
pub struct Autocomplete<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts<M>,
);

/// A function from a text to the application's message.
type TextMessage<M> = Arc<dyn Fn(String) -> M + Send + Sync>;

struct Parts<M> {
    value: String,
    options: Vec<String>,
    placeholder: String,
    limit: usize,
    on_change: Option<TextMessage<M>>,
    on_select: Option<TextMessage<M>>,
}

impl<M> Autocomplete<M> {
    /// Creates a field that shows `value` and offers from `options`.
    pub fn new(
        value: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.gap = 4.0;
        Self(
            shell,
            PhantomData,
            Parts {
                value: value.into(),
                options: options.into_iter().map(Into::into).collect(),
                placeholder: String::new(),
                limit: 5,
                on_change: None,
                on_select: None,
            },
        )
    }

    /// Sets the hint shown while the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.2.placeholder = placeholder.into();
        self
    }

    /// Sets how many suggestions are shown at most; five unless set.
    pub fn max_suggestions(mut self, limit: usize) -> Self {
        self.2.limit = limit;
        self
    }

    /// Reports the text whenever it changes.
    pub fn on_change(mut self, message: impl Fn(String) -> M + Send + Sync + 'static) -> Self {
        self.2.on_change = Some(Arc::new(message));
        self
    }

    /// Reports the option that was tapped.
    pub fn on_select(mut self, message: impl Fn(String) -> M + Send + Sync + 'static) -> Self {
        self.2.on_select = Some(Arc::new(message));
        self
    }
}

/// The options to offer for `value`.
fn suggestions<'a>(value: &str, options: &'a [String], limit: usize) -> Vec<&'a String> {
    let typed = value.trim().to_lowercase();
    if typed.is_empty() {
        return Vec::new();
    }
    options
        .iter()
        .filter(|option| {
            let option = option.to_lowercase();
            option.contains(&typed) && option != typed
        })
        .take(limit)
        .collect()
}

impl<M: 'static> From<Autocomplete<M>> for Element<M> {
    fn from(component: Autocomplete<M>) -> Self {
        let Autocomplete(shell, _, parts) = component;
        let offered = suggestions(&parts.value, &parts.options, parts.limit);
        let mut field = TextField::<M>::new(parts.value.clone())
            .placeholder(parts.placeholder)
            .key("field");
        if let Some(on_change) = &parts.on_change {
            let on_change = Arc::clone(on_change);
            field = field.on_change(move |text| on_change(text));
        }
        let mut column = Column::<M>::new().child(field);
        let pick = parts.on_select.or(parts.on_change);
        if !offered.is_empty() {
            let mut list = Column::<M>::new()
                .background(Color::SURFACE)
                .corner_radius(8)
                .border(1, Color::OUTLINE)
                .animate_enter(120)
                .key("suggestions");
            for (index, option) in offered.into_iter().enumerate() {
                if index > 0 {
                    list = list.child(Divider::new().key(format!("line-{option}")));
                }
                let handler = match &pick {
                    Some(pick) => {
                        let (pick, option) = (Arc::clone(pick), option.clone());
                        Handler::new(move |()| pick(option.clone()))
                    }
                    None => Handler::none(),
                };
                list = list.child(
                    Container::new()
                        .handler(handler)
                        .padding_each(EdgeInsets::symmetric(12.0, 10.0))
                        .key(format!("option-{option}"))
                        .child(Text::new(option.clone())),
                );
            }
            column = column.child(list);
        }
        finish(shell, column)
    }
}

style_builder!(Autocomplete);

#[cfg(test)]
mod tests {
    use super::suggestions;

    #[test]
    fn options_are_offered_while_they_contain_the_text() {
        let options: Vec<String> = ["Kraków", "Kyoto", "Tokyo", "Oslo"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let offered = |value: &str, limit| -> Vec<String> {
            suggestions(value, &options, limit)
                .into_iter()
                .cloned()
                .collect()
        };
        assert!(offered("", 5).is_empty());
        assert!(offered("  ", 5).is_empty());
        assert_eq!(offered("KYO", 5), ["Kyoto", "Tokyo"]);
        assert_eq!(offered("o", 2), ["Kyoto", "Tokyo"]);
        // The text itself is not a suggestion.
        assert!(offered("oslo", 5).is_empty());
        assert!(offered("x", 5).is_empty());
    }
}
