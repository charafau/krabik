//! The `scroll` widget on iOS.

use crate::ios::events::dispatch_scroll;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view, schedule};
use crate::tree::NodeId;
use crate::widgets::{Axis, ScrollProps};
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_ui_kit::UIView;
use objc2_ui_kit::{UIScrollView, UIScrollViewDelegate};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The state shared between a scroll view's delegate and its deferred update.
///
/// `scrollViewDidScroll` fires for every frame of a gesture; the latest offset
/// is stored here and only one main-queue update is scheduled at a time.
#[derive(Default)]
pub(crate) struct ScrollShared {
    pub(crate) latest: AtomicU64,
    pub(crate) scheduled: AtomicBool,
}

/// What a scroll delegate remembers about its node.
pub(crate) struct ScrollDelegateIvars {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) id: NodeId,
    pub(crate) horizontal: bool,
    pub(crate) shared: Arc<ScrollShared>,
}

objc2::define_class!(
    // Scroll callbacks arrive on the main thread, so the delegate is
    // main-thread-only and uses the main-thread `alloc`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikScrollDelegate"]
    #[ivars = ScrollDelegateIvars]
    pub(crate) struct ScrollDelegate;

    unsafe impl NSObjectProtocol for ScrollDelegate {}

    unsafe impl UIScrollViewDelegate for ScrollDelegate {
        #[unsafe(method(scrollViewDidScroll:))]
        fn scroll_view_did_scroll(&self, scroll_view: &UIScrollView) {
            let ivars = self.ivars();
            let offset = scroll_view.contentOffset();
            let value = if ivars.horizontal { offset.x } else { offset.y }.max(0.0);
            ivars
                .shared
                .latest
                .store(value.to_bits(), Ordering::Relaxed);
            if ivars.shared.scheduled.swap(true, Ordering::AcqRel) {
                return;
            }

            let shared = Arc::clone(&ivars.shared);
            let handle = ivars.handle;
            let host = ivars.host;
            let id = ivars.id;
            schedule(move || {
                shared.scheduled.store(false, Ordering::Release);
                let offset = f64::from_bits(shared.latest.load(Ordering::Acquire));
                dispatch_scroll(handle, host, id, offset);
            });
        }
    }
);

impl ScrollDelegate {
    pub(crate) fn new(
        handle: u64,
        host: u64,
        id: NodeId,
        horizontal: bool,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ScrollDelegateIvars {
            handle,
            host,
            id,
            horizontal,
            shared: Arc::new(ScrollShared::default()),
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }
}

/// Creates the native view of a scroll container.
pub(crate) fn create(props: &ScrollProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let scroll = UIScrollView::new(site.mtm);
    scroll.setClipsToBounds(true);
    scroll.setShowsVerticalScrollIndicator(props.axis == Axis::Vertical);
    scroll.setShowsHorizontalScrollIndicator(props.axis == Axis::Horizontal);
    scroll.setAlwaysBounceVertical(props.axis == Axis::Vertical);
    scroll.setAlwaysBounceHorizontal(props.axis == Axis::Horizontal);
    let host = UIView::new(site.mtm);
    scroll.addSubview(&host);
    let delegate = ScrollDelegate::new(
        site.handle,
        site.host,
        site.node,
        props.axis == Axis::Horizontal,
        site.mtm,
    );
    // SAFETY: the delegate is owned by the view handle below and
    // only reads the scroll view's offset.
    unsafe { scroll.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
    crate::ios::widgets::refresh::attach(&scroll, &props.refresh, site);
    let view = cast_view(scroll.clone());
    ViewHandle::new(
        view,
        Content::Scroll {
            scroll,
            host,
            _delegate: delegate,
        },
        node,
    )
}
