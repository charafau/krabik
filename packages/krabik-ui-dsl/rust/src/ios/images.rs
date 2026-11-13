//! Loading bundled and remote images into image views.

use crate::ios::host::{Content, IosState};
use crate::ios::{guard, schedule, with_state};
use crate::tree::NodeId;
use crate::ui::{ImageProps, ImageScale, ImageSource};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2_foundation::{NSBundle, NSData, NSError, NSString, NSURL, NSURLResponse, NSURLSession};
use objc2_ui_kit::{UIImage, UIImageView, UIViewContentMode};
use std::collections::{HashMap, HashSet};

/// Remote image state for one root view.
#[derive(Default)]
pub(crate) struct ImageLoader {
    /// Decoded images keyed by URL. Grows for the session's lifetime, which
    /// is fine for a demo-sized number of remote images.
    pub(crate) cache: HashMap<String, Retained<UIImage>>,
    /// URLs with a download in flight, so repeated renders do not refetch.
    pub(crate) in_flight: HashSet<String>,
    /// The URL each image node is currently expected to display. A download
    /// only updates views whose entry still matches its URL.
    pub(crate) expected: HashMap<NodeId, String>,
}

impl IosState {
    /// Configures an image view for the given properties.
    ///
    /// Assets load synchronously; remote URLs load through the cache and
    /// `NSURLSession`, updating the view when the download finishes.
    pub(crate) fn load_image(&mut self, id: NodeId, view: &UIImageView, props: &ImageProps) {
        view.setContentMode(match props.scale {
            ImageScale::Fit => UIViewContentMode::ScaleAspectFit,
            ImageScale::Fill => UIViewContentMode::ScaleAspectFill,
        });
        match &props.source {
            ImageSource::Asset(path) => {
                self.images.expected.remove(&id);
                view.setImage(load_asset(path).as_deref());
            }
            ImageSource::Url(url) => self.load_remote_image(id, view, url),
        }
    }

    /// Shows a remote image, downloading it only once per URL.
    ///
    /// The view is updated immediately on a cache hit. Otherwise the URL
    /// starts a single download, and the view is filled in by
    /// [`finish_download`] when it completes.
    pub(crate) fn load_remote_image(&mut self, id: NodeId, view: &UIImageView, url: &str) {
        self.images.expected.insert(id, url.to_string());
        if let Some(image) = self.images.cache.get(url).map(|image| &**image) {
            view.setImage(Some(image));
            return;
        }
        if self.images.in_flight.insert(url.to_string()) {
            start_download(self.handle, self.host, id, url);
        }
    }
}

pub(crate) fn load_asset(path: &str) -> Option<Retained<UIImage>> {
    let (stem, extension) = match path.rsplit_once('.') {
        Some((stem, extension)) => (stem, Some(NSString::from_str(extension))),
        None => (path, None),
    };
    let stem = NSString::from_str(stem);
    let path = NSBundle::mainBundle().pathForResource_ofType(Some(&stem), extension.as_deref())?;
    UIImage::imageWithContentsOfFile(&path)
}

/// Starts an `NSURLSession` download for a remote image.
///
/// The completion handler decodes the image off the main thread and hands it
/// to the main queue, where [`finish_download`] applies it.
pub(crate) fn start_download(handle: u64, host: u64, id: NodeId, url: &str) {
    let Some(url_object) = NSURL::URLWithString(&NSString::from_str(url)) else {
        eprintln!("krabik ios: invalid image URL: {url}");
        return;
    };
    let url = url.to_string();
    let block = RcBlock::new(
        move |data: *mut NSData, _: *mut NSURLResponse, _: *mut NSError| {
            let image = unsafe { data.as_ref() }.and_then(UIImage::imageWithData);
            let url = url.clone();
            schedule(move || finish_download(handle, host, id, url, image));
        },
    );
    // SAFETY: the block is sendable and retained by the task until the
    // download completes.
    let task = unsafe {
        NSURLSession::sharedSession().dataTaskWithURL_completionHandler(&url_object, &block)
    };
    task.resume();
}

/// Applies a finished download to every view that still expects the image.
pub(crate) fn finish_download(
    handle: u64,
    host: u64,
    id: NodeId,
    url: String,
    image: Option<Retained<UIImage>>,
) {
    guard("image", || {
        with_state(handle, host, |state| {
            state.images.in_flight.remove(&url);
            let Some(image) = image else {
                // Allow a later render to retry the failed URL, but only if
                // the node has not moved on to a different one meanwhile.
                if state.images.expected.get(&id).map(String::as_str) == Some(url.as_str()) {
                    state.images.expected.remove(&id);
                }
                return;
            };
            state.images.cache.insert(url.clone(), image.clone());
            let waiting: Vec<NodeId> = state
                .images
                .expected
                .iter()
                .filter(|(_, expected)| expected.as_str() == url.as_str())
                .map(|(node_id, _)| *node_id)
                .collect();
            for node_id in waiting {
                let mut targets: Vec<Retained<UIImageView>> = Vec::new();
                if let Some(view) = state.views.get(&node_id)
                    && let Content::Image(image_view) = &view.content
                {
                    targets.push(image_view.clone());
                }
                for row in state.row_states.values() {
                    if let Some(view) = row.views.get(&node_id)
                        && let Content::Image(image_view) = &view.content
                    {
                        targets.push(image_view.clone());
                    }
                }
                for image_view in targets {
                    image_view.setImage(Some(&*image));
                }
            }
            state.images.expected.retain(|_, expected| expected != &url);
        });
    });
}
