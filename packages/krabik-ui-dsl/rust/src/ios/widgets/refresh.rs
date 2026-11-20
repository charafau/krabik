//! Pull-to-refresh on iOS: a `UIRefreshControl` on a scroll view or on the
//! collection view of a recycled list.

use crate::host::NodeEvent;
use crate::ios::events::dispatch;
use crate::ios::widgets::Site;
use crate::ios::{add_action, schedule};
use crate::ui::Refresh;
use objc2_ui_kit::{UIControlEvents, UIRefreshControl, UIScrollView};

/// Gives a refreshable container its refresh control. Whether a container
/// is refreshable is part of its identity, so this happens once, when the
/// view is created.
pub(crate) fn attach(scroll: &UIScrollView, refresh: &Refresh, site: &Site) {
    if !refresh.is_enabled() {
        return;
    }
    let control = UIRefreshControl::new(site.mtm);
    let (handle, host, id) = (site.handle, site.host, site.node);
    add_action(
        &control,
        UIControlEvents::ValueChanged,
        "krabik.refresh",
        site.mtm,
        move |_| schedule(move || dispatch(handle, host, id, NodeEvent::Refresh)),
    );
    scroll.setRefreshControl(Some(&control));
}

/// Shows the indicator while the application says a refresh is going on.
pub(crate) fn update(scroll: &UIScrollView, refresh: &Refresh) {
    let Some(control) = scroll.refreshControl() else {
        return;
    };
    if refresh.refreshing && !control.isRefreshing() {
        control.beginRefreshing();
    } else if !refresh.refreshing && control.isRefreshing() {
        control.endRefreshing();
    }
}
