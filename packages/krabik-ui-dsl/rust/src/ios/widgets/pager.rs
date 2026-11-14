//! The `pager` widget on iOS.

use crate::ios::events::dispatch_selected;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view, schedule};
use crate::tree::NodeId;
use crate::widgets::PagerProps;
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_ui_kit::{UIScrollView, UIScrollViewDelegate};

/// What a pager delegate remembers about its node.
pub(crate) struct PagerDelegateIvars {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) id: NodeId,
}

objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikPagerDelegate"]
    #[ivars = PagerDelegateIvars]
    pub(crate) struct PagerDelegate;

    unsafe impl NSObjectProtocol for PagerDelegate {}

    unsafe impl UIScrollViewDelegate for PagerDelegate {
        #[unsafe(method(scrollViewDidEndDecelerating:))]
        fn scroll_view_did_end_decelerating(&self, scroll_view: &UIScrollView) {
            report_pager_page(self.ivars(), scroll_view);
        }

        #[unsafe(method(scrollViewDidEndDragging:willDecelerate:))]
        fn scroll_view_did_end_dragging(&self, scroll_view: &UIScrollView, will_decelerate: bool) {
            if !will_decelerate {
                report_pager_page(self.ivars(), scroll_view);
            }
        }
    }
);

pub(crate) fn report_pager_page(ivars: &PagerDelegateIvars, scroll_view: &UIScrollView) {
    let width = scroll_view.bounds().size.width;
    if width <= 0.0 {
        return;
    }
    let pages = (scroll_view.contentSize().width / width).round().max(1.0) as isize;
    let page =
        ((scroll_view.contentOffset().x / width).round() as isize).clamp(0, pages - 1) as usize;
    let handle = ivars.handle;
    let host = ivars.host;
    let id = ivars.id;
    schedule(move || dispatch_selected(handle, host, id, page));
}

impl PagerDelegate {
    pub(crate) fn new(handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PagerDelegateIvars { handle, host, id });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }
}

/// Creates the native view of a pager.
pub(crate) fn create(props: &PagerProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let scroll = UIScrollView::new(site.mtm);
    scroll.setPagingEnabled(true);
    scroll.setShowsHorizontalScrollIndicator(false);
    scroll.setBounces(false);
    let delegate = PagerDelegate::new(site.handle, site.host, site.node, site.mtm);
    // SAFETY: the delegate is kept alive by the view handle.
    unsafe { scroll.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
    let view = cast_view(scroll.clone());
    ViewHandle::new(
        view,
        Content::Pager {
            scroll,
            _delegate: delegate,
            pages: props.pages,
            selected: props.selected,
        },
        node,
    )
}
