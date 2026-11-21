//! The `native_view` widget: a view the application's own platform code
//! makes, and the web view the framework makes with it.

use std::marker::PhantomData;

use krabik_ui_dsl_macros::Widget;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::widgets::{IntrinsicSize, Widget};

/// Platform-independent properties of a native view node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = NativeView,
    about = "A view made by the application's own Kotlin and Swift code.\n\nThe framework draws what both platforms have in common. For anything else, a map, a video player, a chart from a platform library, the application registers a factory under a name on each platform and places its view here:\n\n```ignore\nNativeView::new(\"video\")\n    .prop(\"url\", &self.video_url)\n    .height(220)\n    .on_event(Msg::Player)\n```\n\nRust lays the view out and tells it its properties, as strings; the view reports events as strings. A name nothing is registered under shows an empty view and is logged. Give the view a size: without one it is 200 points square.",
    new = "Creates a view from the factory registered under `name`."
)]
pub struct NativeViewProps {
    /// The name the factory is registered under on each platform.
    #[widget(arg)]
    pub name: String,
    /// What the view is told, in the order it was said.
    pub props: Vec<(String, String)>,
    /// Reports a string the view sent.
    pub on_event: Handler<String>,
}

impl<M> NativeView<M> {
    /// Tells the view a property. A later value for the same key replaces
    /// the earlier one.
    pub fn prop(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let ElementKind::Widget(Widget::NativeView(props)) = &mut self.0.kind {
            let (key, value) = (key.into(), value.into());
            match props
                .props
                .iter_mut()
                .find(|(existing, _)| *existing == key)
            {
                Some(entry) => entry.1 = value,
                None => props.props.push((key, value)),
            }
        }
        self
    }
}

/// The side of a native view that was given no size, in logical points.
const NATIVE_VIEW_SIZE: f32 = 200.0;

impl IntrinsicSize for NativeViewProps {
    /// Rust cannot ask a view it does not know how large it wants to be.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(max_width.unwrap_or(NATIVE_VIEW_SIZE), NATIVE_VIEW_SIZE)
    }
}

/// The name the framework registers its web view under on both platforms.
pub(crate) const WEB_VIEW: &str = "krabik.webview";

/// A web page inside the application: the platform's web view.
///
/// ```ignore
/// WebView::new("https://example.com/help")
///     .height(400)
///     .on_navigate(Msg::Visited)
/// ```
///
/// It shows `url` and loads another page when the application says another
/// address. `on_navigate` reports the address of every page that finished
/// loading, including the ones the user reached by following links.
///
/// An Android application needs the `INTERNET` permission in its manifest
/// to load anything from the network.
pub struct WebView<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> WebView<M> {
    /// Creates a web view that shows the page at `url`.
    pub fn new(url: impl Into<String>) -> Self {
        let view = NativeView::<M>::new(WEB_VIEW).prop("url", url);
        Self(view.0, PhantomData)
    }

    /// Turns JavaScript off; it is on unless this is called.
    pub fn without_javascript(mut self) -> Self {
        if let ElementKind::Widget(Widget::NativeView(props)) = &mut self.0.kind {
            props
                .props
                .push(("javascript".to_owned(), "false".to_owned()));
        }
        self
    }

    /// Reports the address of each page that finished loading.
    pub fn on_navigate(mut self, message: impl Fn(String) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::NativeView(props)) = &mut self.0.kind {
            props.on_event = Handler::new(message);
        }
        self
    }
}

impl<M> From<WebView<M>> for Element<M> {
    fn from(component: WebView<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(WebView);

#[cfg(test)]
mod tests {
    use super::*;

    fn props(element: &Element) -> &NativeViewProps {
        match &element.kind {
            ElementKind::Widget(Widget::NativeView(props)) => props,
            _ => panic!("expected a native view"),
        }
    }

    #[test]
    fn a_native_view_carries_a_name_and_its_latest_properties() {
        let element: Element = NativeView::new("video")
            .prop("url", "a.mp4")
            .prop("muted", "true")
            .prop("url", "b.mp4")
            .into();
        let props = props(&element);
        assert_eq!(props.name, "video");
        assert_eq!(
            props.props,
            [
                ("url".to_owned(), "b.mp4".to_owned()),
                ("muted".to_owned(), "true".to_owned())
            ]
        );
        assert!(!props.on_event.is_set());
    }

    #[test]
    fn a_web_view_is_the_frameworks_own_native_view() {
        let element: Element<String> = WebView::new("https://example.com")
            .on_navigate(|url| url)
            .into();
        let ElementKind::Widget(Widget::NativeView(props)) = &element.kind else {
            panic!("expected a native view");
        };
        assert_eq!(props.name, WEB_VIEW);
        assert_eq!(
            props.props[0],
            ("url".to_owned(), "https://example.com".to_owned())
        );
        assert!(props.on_event.is_set());
    }
}
