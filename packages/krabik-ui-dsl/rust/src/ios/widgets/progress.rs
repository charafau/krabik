//! The `progress` widget on iOS.

use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view, color};
use crate::ui::{ProgressProps, SpinnerProps};
use objc2::MainThreadOnly;
use objc2_ui_kit::UIActivityIndicatorViewStyle;
use objc2_ui_kit::{UIActivityIndicatorView, UIProgressView};

pub(crate) fn update_progress(progress: &UIProgressView, props: &ProgressProps) {
    progress.setProgress(props.value.clamp(0.0, 1.0));
    let tint = color(props.color);
    progress.setProgressTintColor(Some(&tint));
}

pub(crate) fn update_spinner(spinner: &UIActivityIndicatorView, props: &SpinnerProps) {
    let tint = color(props.color);
    // SAFETY: the color is retained by the view for as long as it is set.
    unsafe { spinner.setColor(Some(&tint)) };
    spinner.startAnimating();
}

/// Creates the native view of a determinate progress bar.
pub(crate) fn create(props: &ProgressProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let progress = UIProgressView::new(site.mtm);
    update_progress(&progress, props);
    let view = cast_view(progress.clone());
    ViewHandle::new(view, Content::Progress(progress), node)
}

/// Creates the native view of an indeterminate spinner.
pub(crate) fn create_spinner(props: &SpinnerProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let spinner = UIActivityIndicatorView::initWithActivityIndicatorStyle(
        UIActivityIndicatorView::alloc(site.mtm),
        UIActivityIndicatorViewStyle::Medium,
    );
    update_spinner(&spinner, props);
    let view = cast_view(spinner.clone());
    ViewHandle::new(view, Content::Spinner(spinner), node)
}
