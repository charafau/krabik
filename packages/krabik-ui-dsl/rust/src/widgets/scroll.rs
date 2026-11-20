//! The `scroll` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::message::Handler;

/// The axis a scroll container scrolls along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Scroll vertically.
    #[default]
    Vertical,
    /// Scroll horizontally.
    Horizontal,
}

/// Pull-to-refresh on a vertical scroll container or a list.
///
/// The gesture and its indicator are the platform's own. The application
/// decides how long the indicator stays: it is shown while `refreshing` is
/// true, so an application that sets it when the refresh starts and clears
/// it when the data arrives gets the usual behaviour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Refresh {
    /// Sends its message when the user pulls down past the top.
    pub on_refresh: Handler<()>,
    /// Whether the refresh indicator is showing.
    pub refreshing: bool,
}

impl Refresh {
    /// Whether the container offers the gesture at all.
    pub fn is_enabled(&self) -> bool {
        self.on_refresh.is_set()
    }
}

/// Platform-independent properties of a scroll container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollProps {
    /// The axis the container scrolls along.
    pub axis: Axis,
    /// Pull-to-refresh, for a vertical container.
    pub refresh: Refresh,
}

/// A native scroll container.
///
/// Give it a bounded size (`.grow(1)`, a fixed height, or a parent-provided
/// constraint) so its content can actually overflow.
pub struct ScrollView<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> ScrollView<M> {
    /// Creates a vertical scroll container.
    pub fn new() -> Self {
        Self(
            RawElement::container(ElementKind::Scroll(ScrollProps {
                axis: Axis::Vertical,
                refresh: Refresh::default(),
            })),
            PhantomData,
        )
    }

    /// Creates a horizontal scroll container.
    pub fn horizontal() -> Self {
        Self(
            RawElement::container(ElementKind::Scroll(ScrollProps {
                axis: Axis::Horizontal,
                refresh: Refresh::default(),
            })),
            PhantomData,
        )
    }

    /// The axis the container scrolls along.
    pub fn axis(&self) -> Axis {
        match &self.0.kind {
            ElementKind::Scroll(props) => props.axis,
            _ => Axis::Vertical,
        }
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }

    /// Sends `message` when the user pulls the content down past its top:
    /// the platform's pull-to-refresh. A horizontal container ignores it.
    ///
    /// Show the indicator for as long as the refresh takes with
    /// [`refreshing`](Self::refreshing).
    pub fn on_refresh(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Scroll(props) = &mut self.0.kind
            && props.axis == Axis::Vertical
        {
            props.refresh.on_refresh = Handler::new(move |()| message.clone());
        }
        self
    }

    /// Shows the refresh indicator while `refreshing` is true.
    pub fn refreshing(mut self, refreshing: bool) -> Self {
        if let ElementKind::Scroll(props) = &mut self.0.kind {
            props.refresh.refreshing = refreshing;
        }
        self
    }
}

impl<M> Default for ScrollView<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<ScrollView<M>> for Element<M> {
    fn from(component: ScrollView<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(ScrollView);
