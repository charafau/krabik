//! The `pager` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::style::Length;
use crate::widgets::Widget;
use crate::widgets::{Column, TabBar};
use std::borrow::Cow;

/// Platform-independent properties of a swipeable page container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PagerProps {
    /// The number of pages.
    pub pages: usize,
    /// The index of the selected page.
    pub selected: usize,
    /// The action dispatched when the user swipes to another page.
    pub action: Cow<'static, str>,
}

/// A native horizontally paged container.
#[derive(Clone, Debug, PartialEq)]
pub struct Pager(pub(crate) Element);

impl Pager {
    /// Creates an empty pager that fills the available width.
    pub fn new() -> Self {
        let mut element = Element::container(ElementKind::Pager(PagerProps::default()));
        element.style.width = Length::Percent(1.0);
        Self(element)
    }

    /// Sets the selected page index. Changing it scrolls the native pager
    /// to that page in place; the pages keep their state.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched when the page changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Pager> for Element {
    fn from(component: Pager) -> Self {
        component.0
    }
}

/// A tab strip above a swipeable pager.
#[derive(Clone, Debug, PartialEq)]
pub struct TabsView(pub(crate) Element);

impl TabsView {
    /// Creates a tabs view with the given labels and no pages.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let column = Column::new()
            .width_percent(1.0)
            .child(TabBar::new(tabs))
            .child(Pager::new().grow(1.0));
        Self(column.0)
    }

    /// Sets the selected tab and page.
    pub fn selected(mut self, index: usize) -> Self {
        if let Some(bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut bar.kind
        {
            props.selected = index;
        }
        if let Some(pager) = self.0.children.get_mut(1)
            && let ElementKind::Pager(props) = &mut pager.kind
        {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched for tab taps and page swipes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        let action = action.into();
        if let Some(bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut bar.kind
        {
            props.action = action.clone();
        }
        if let Some(pager) = self.0.children.get_mut(1)
            && let ElementKind::Pager(props) = &mut pager.kind
        {
            props.action = action;
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        if let Some(pager) = self.0.children.get_mut(1) {
            pager.children.push(child.into());
        }
        self
    }
}

impl From<TabsView> for Element {
    fn from(component: TabsView) -> Self {
        component.0
    }
}

style_builder!(Pager);
style_builder!(TabsView);
