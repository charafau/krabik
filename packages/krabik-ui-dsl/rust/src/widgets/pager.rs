//! The `pager` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::message::Handler;
use crate::style::Length;
use crate::widgets::Widget;
use crate::widgets::{Column, TabBar};
use std::sync::Arc;

/// Platform-independent properties of a swipeable page container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PagerProps {
    /// The number of pages.
    pub pages: usize,
    /// The index of the selected page.
    pub selected: usize,
    /// Reported with the index of the page the user swiped to.
    pub on_select: Handler<usize>,
}

/// A native horizontally paged container.
pub struct Pager<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Pager<M> {
    /// Creates an empty pager that fills the available width.
    pub fn new() -> Self {
        let mut element = RawElement::container(ElementKind::Pager(PagerProps::default()));
        element.style.width = Length::Percent(1.0);
        Self(element, PhantomData)
    }

    /// Sets the selected page index. Changing it scrolls the native pager
    /// to that page in place; the pages keep their state.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Reports the index of the page the user swiped to.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.on_select = Handler::new(message);
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for Pager<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Pager<M>> for Element<M> {
    fn from(component: Pager<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// A tab strip above a swipeable pager.
pub struct TabsView<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> TabsView<M> {
    /// Creates a tabs view with the given labels and no pages.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let column = Column::<M>::new()
            .width_percent(1.0)
            .child(TabBar::new(tabs))
            .child(Pager::new().grow(1.0));
        Self(column.0, PhantomData)
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

    /// Reports the index the user picked, by tapping a tab or swiping a page.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        // The tab bar and the pager report the same selection.
        let message = Arc::new(message);
        if let Some(bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut bar.kind
        {
            let message = Arc::clone(&message);
            props.on_select = Handler::new(move |index| message(index));
        }
        if let Some(pager) = self.0.children.get_mut(1)
            && let ElementKind::Pager(props) = &mut pager.kind
        {
            props.on_select = Handler::new(move |index| message(index));
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        if let Some(pager) = self.0.children.get_mut(1) {
            pager.children.push(child.into().into_raw());
        }
        self
    }
}

impl<M> From<TabsView<M>> for Element<M> {
    fn from(component: TabsView<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Pager);
style_builder!(TabsView);
