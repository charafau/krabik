//! The `tab_bar` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::message::Wrap;
use crate::style::FontWeight;
use crate::widgets::CONTROL_WIDTH;
use crate::widgets::Column;
use crate::widgets::IntrinsicSize;
use crate::widgets::TextStyleMut;
use crate::widgets::Widget;
use crate::widgets::WidgetProps;

/// Platform-independent properties of a native tab bar.
#[derive(Clone, Debug, PartialEq)]
pub struct TabBarProps {
    /// The labels of the tabs, in display order.
    pub tabs: Vec<String>,
    /// The index of the selected tab.
    pub selected: usize,
    /// Reported with the index of the tab the user picked.
    pub on_select: Handler<usize>,
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
            on_select: Handler::none(),
            size: 14.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native tab bar that reports the tapped tab through [`TabBar::on_select`].
///
/// The application owns the selection: it stores the index and passes it back
/// with [`TabBar::selected`]. Use [`Tabs`] to pair the bar with a content view.
pub struct TabBar<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> TabBar<M> {
    /// Creates a tab bar with the given labels and the first tab selected.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let tabs: Vec<String> = tabs.into_iter().map(Into::into).collect();
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::TabBar(TabBarProps::new(tabs)))),
            PhantomData,
        )
    }

    /// Sets the index of the selected tab.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Widget(Widget::TabBar(props)) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Reports the index of the tab the user picked.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::TabBar(props)) = &mut self.0.kind {
            props.on_select = Handler::new(message);
        }
        self
    }
}

impl<M> From<TabBar<M>> for Element<M> {
    fn from(component: TabBar<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// A tab bar stacked above application-owned content.
///
/// This is a plain composition of [`TabBar`] and one child; it does not create
/// a native tab controller. The application chooses which content to build for
/// the current index and updates it from [`Tabs::on_select`].
pub struct Tabs<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Tabs<M> {
    /// Creates a tab bar with the given labels above empty content.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut column = Column::<M>::new();
        column.0.children.push(TabBar::<M>::new(tabs).0);
        Self(column.0, PhantomData)
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

    /// Reports the index of the tab the user picked.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let Some(tab_bar) = self.0.children.first_mut()
            && let ElementKind::Widget(Widget::TabBar(props)) = &mut tab_bar.kind
        {
            props.on_select = Handler::new(message);
        }
        self
    }

    /// Appends the content shown below the tab bar.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> From<Tabs<M>> for Element<M> {
    fn from(component: Tabs<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(TabBar);

/// The height of a native tab bar.
const TAB_BAR_HEIGHT: f32 = 48.0;

impl WidgetProps for TabBarProps {
    fn map_messages(&mut self, wrap: &Wrap) {
        self.on_select.map(wrap);
    }

    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        Some(TextStyleMut {
            size: &mut self.size,
            color: &mut self.color,
            weight: &mut self.weight,
        })
    }
}

impl IntrinsicSize for TabBarProps {
    /// A tab bar fills the available width and has a fixed touch height.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), TAB_BAR_HEIGHT)
    }
}
