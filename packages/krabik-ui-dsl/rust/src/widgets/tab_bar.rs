//! The `tab_bar` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::engine::geometry::Size;
use crate::style::FontWeight;
use crate::widgets::CONTROL_WIDTH;
use crate::widgets::Column;
use crate::widgets::Widget;
use std::borrow::Cow;

/// Platform-independent properties of a native tab bar.
#[derive(Clone, Debug, PartialEq)]
pub struct TabBarProps {
    /// The labels of the tabs, in display order.
    pub tabs: Vec<String>,
    /// The index of the selected tab.
    pub selected: usize,
    /// The action dispatched whenever the selection changes.
    pub action: Cow<'static, str>,
    /// Font size of the labels in logical points.
    pub size: f32,
    /// ARGB color of the labels.
    pub color: u32,
    /// Font weight of the labels.
    pub weight: FontWeight,
}

impl TabBarProps {
    /// Creates a tab bar with the given labels.
    pub fn new(tabs: Vec<String>) -> Self {
        Self {
            tabs,
            selected: 0,
            action: Cow::Borrowed(""),
            size: 14.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native tab bar that reports the tapped tab through [`Event::Selected`].
///
/// The application owns the selection: it stores the index and passes it back
/// with [`TabBar::selected`]. Use [`Tabs`] to pair the bar with a content view.
///
/// [`Event::Selected`]: crate::app::Event::Selected
#[derive(Clone, Debug, PartialEq)]
pub struct TabBar(pub(crate) Element);

impl TabBar {
    /// Creates a tab bar with the given labels and the first tab selected.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let tabs: Vec<String> = tabs.into_iter().map(Into::into).collect();
        Self(Element::leaf(ElementKind::Widget(Widget::TabBar(
            TabBarProps::new(tabs),
        ))))
    }

    /// Sets the index of the selected tab.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Widget(Widget::TabBar(props)) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched whenever the selection changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::TabBar(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<TabBar> for Element {
    fn from(component: TabBar) -> Self {
        component.0
    }
}

/// A tab bar stacked above application-owned content.
///
/// This is a plain composition of [`TabBar`] and one child; it does not create
/// a native tab controller. The application chooses which content to build for
/// the current index and handles [`Event::Selected`](crate::app::Event::Selected).
#[derive(Clone, Debug, PartialEq)]
pub struct Tabs(pub(crate) Element);

impl Tabs {
    /// Creates a tab bar with the given labels above empty content.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut column = Column::new();
        column.0.children.push(TabBar::new(tabs).into());
        Self(column.0)
    }

    /// Sets the index of the selected tab.
    pub fn selected(mut self, index: usize) -> Self {
        if let Some(tab_bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut tab_bar.kind
        {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched whenever the selection changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let Some(tab_bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut tab_bar.kind
        {
            props.action = action.into();
        }
        self
    }

    /// Appends the content shown below the tab bar.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl From<Tabs> for Element {
    fn from(component: Tabs) -> Self {
        component.0
    }
}

style_builder!(TabBar);

/// The height of a native tab bar.
const TAB_BAR_HEIGHT: f32 = 48.0;

impl TabBarProps {
    /// A tab bar fills the available width and has a fixed touch height.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), TAB_BAR_HEIGHT)
    }
}
