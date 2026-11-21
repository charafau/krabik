//! Loading bundled and remote images into image views.

use crate::ios::host::{Content, IosState};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{guard, schedule, with_state};
use crate::render::RenderNode;
use crate::tree::NodeId;
use crate::ui::{ImageProps, ImageScale, ImageSource};
use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::{NSBundle, NSData, NSError, NSString, NSURL, NSURLResponse, NSURLSession};
use objc2_ui_kit::{UIImage, UIImageRenderingMode, UIImageView, UIViewContentMode};
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
    /// The image nodes that show an SVG asset.
    #[cfg(feature = "svg")]
    pub(crate) svgs: HashMap<NodeId, SvgImage>,
}

/// An SVG asset an image view shows. Rust draws it at the size of the view,
/// so it stays sharp at any size, and again when that size changes.
#[cfg(feature = "svg")]
pub(crate) struct SvgImage {
    path: String,
    fill: bool,
    data: Vec<u8>,
    /// The size, in pixels, of the rendering the view shows.
    rendered: (u32, u32),
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
                #[cfg(feature = "svg")]
                if crate::svg::is_svg(path) {
                    self.load_svg(id, view, path, props.scale == ImageScale::Fill);
                    return;
                }
                #[cfg(feature = "svg")]
                self.images.svgs.remove(&id);
                view.setImage(load_asset(path).as_deref());
            }
            ImageSource::Url(url) => self.load_remote_image(id, view, url),
            ImageSource::Icon(icon) => {
                self.images.expected.remove(&id);
                #[cfg(feature = "svg")]
                self.images.svgs.remove(&id);
                view.setContentMode(UIViewContentMode::ScaleAspectFit);
                view.setImage(load_icon(icon.ios_name().unwrap_or_default()).as_deref());
            }
        }
        // Only a template image, which an icon is, is drawn in the tint.
        let tint = props.tint.map(crate::ios::color);
        // SAFETY: the color is retained by the view for as long as it is set.
        unsafe { view.setTintColor(tint.as_deref()) };
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

#[cfg(feature = "svg")]
impl IosState {
    /// Shows an SVG asset. It is drawn now if the view already has a size,
    /// and by [`IosState::fit_svg`] whenever a layout gives it one.
    fn load_svg(&mut self, id: NodeId, view: &UIImageView, path: &str, fill: bool) {
        let same = self
            .images
            .svgs
            .get(&id)
            .is_some_and(|svg| svg.path == path && svg.fill == fill);
        if !same {
            view.setImage(None);
            let Some(data) = asset_bytes(path) else {
                self.images.svgs.remove(&id);
                return;
            };
            self.images.svgs.insert(
                id,
                SvgImage {
                    path: path.to_owned(),
                    fill,
                    data,
                    rendered: (0, 0),
                },
            );
        }
        let size = view.bounds().size;
        self.fit_svg(id, view, size.width, size.height);
    }

    /// Draws the SVG of image node `id` for a view of `width` by `height`
    /// points, unless that is what the view shows already.
    pub(crate) fn fit_svg(&mut self, id: NodeId, view: &UIImageView, width: f64, height: f64) {
        let Some(svg) = self.images.svgs.get_mut(&id) else {
            return;
        };
        let scale = view.contentScaleFactor().max(1.0);
        let pixels = (
            (width * scale).round().max(0.0) as u32,
            (height * scale).round().max(0.0) as u32,
        );
        if pixels == svg.rendered || pixels.0 == 0 || pixels.1 == 0 {
            return;
        }
        svg.rendered = pixels;
        let image = crate::svg::render_png(&svg.data, pixels.0, pixels.1, svg.fill)
            .and_then(|png| UIImage::imageWithData_scale(&NSData::with_bytes(&png), scale));
        view.setImage(image.as_deref());
    }

    /// Forgets the SVG of a node whose view is gone.
    pub(crate) fn forget_svg(&mut self, id: NodeId) {
        self.images.svgs.remove(&id);
    }
}

/// The path of a bundled asset, which is a file at the root of the bundle.
pub(crate) fn asset_path(path: &str) -> Option<Retained<NSString>> {
    let (stem, extension) = match path.rsplit_once('.') {
        Some((stem, extension)) => (stem, Some(NSString::from_str(extension))),
        None => (path, None),
    };
    let stem = NSString::from_str(stem);
    NSBundle::mainBundle().pathForResource_ofType(Some(&stem), extension.as_deref())
}

/// The bytes of a bundled asset.
#[cfg(feature = "svg")]
pub(crate) fn asset_bytes(path: &str) -> Option<Vec<u8>> {
    std::fs::read(asset_path(path)?.to_string()).ok()
}

/// An icon by name: an SF Symbol, or else an image of the asset catalog,
/// which is then drawn in one color as a symbol is. A name that is neither
/// is logged once.
pub(crate) fn load_icon(name: &str) -> Option<Retained<UIImage>> {
    static MISSING: std::sync::Mutex<Option<HashSet<String>>> = std::sync::Mutex::new(None);
    if name.is_empty() {
        return None;
    }
    let ns_name = NSString::from_str(name);
    let image = UIImage::systemImageNamed(&ns_name).or_else(|| {
        UIImage::imageNamed(&ns_name)
            .map(|image| image.imageWithRenderingMode(UIImageRenderingMode::AlwaysTemplate))
    });
    if image.is_none() {
        let mut missing = MISSING.lock().expect("missing icons");
        if missing
            .get_or_insert_with(HashSet::new)
            .insert(name.to_owned())
        {
            crate::log::error(&format!("no symbol or image named \"{name}\" for an icon"));
        }
    }
    image
}

pub(crate) fn load_asset(path: &str) -> Option<Retained<UIImage>> {
    let path = asset_path(path)?;
    UIImage::imageWithContentsOfFile(&path)
}

/// Starts an `NSURLSession` download for a remote image.
///
/// The completion handler decodes the image off the main thread and hands it
/// to the main queue, where [`finish_download`] applies it.
pub(crate) fn start_download(handle: u64, host: u64, id: NodeId, url: &str) {
    let Some(url_object) = NSURL::URLWithString(&NSString::from_str(url)) else {
        crate::log::error(&format!("invalid image URL: {url}"));
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
                for row in state.rows.rows() {
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

impl IosState {
    /// Creates the native view of an image and starts loading it.
    pub(crate) fn create_image(
        &mut self,
        props: &ImageProps,
        node: &RenderNode,
        mtm: MainThreadMarker,
    ) -> ViewHandle {
        let image = UIImageView::new(mtm);
        self.load_image(node.id, &image, props);
        let view = cast_view(image.clone());
        ViewHandle::new(view, Content::Image(image), node)
    }
}
