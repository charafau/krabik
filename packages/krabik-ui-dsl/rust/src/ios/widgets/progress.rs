//! The `progress` widget on iOS.

use crate::ios::color;
use crate::ui::{ProgressProps, SpinnerProps};
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
