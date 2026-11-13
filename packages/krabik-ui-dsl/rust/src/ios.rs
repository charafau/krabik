//! iOS renderer driven directly through UIKit.
//!
//! The Swift side is a thin `UIView`: it forwards its bounds to Rust and hosts
//! the native widgets. Every widget, every property update, every frame change,
//! and every event is handled here on native `UILabel`/`UIButton`/… instances.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, c_char, c_void};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
use objc2_core_foundation::{CFTimeInterval, CGAffineTransform, CGPoint, CGRect, CGSize, CGVector};
use objc2_core_graphics::CGColor;
use objc2_foundation::{
    NSArray, NSBundle, NSCalendar, NSCalendarUnit, NSData, NSDate, NSDateComponents, NSError,
    NSIndexPath, NSNumber, NSRunLoop, NSRunLoopCommonModes, NSString, NSURL, NSURLResponse,
    NSURLSession, NSValue,
};
use objc2_quartz_core::{
    CABasicAnimation, CACornerMask, CACurrentMediaTime, CADisplayLink, CAGradientLayer, CALayer,
    CAMediaTiming, CAMediaTimingFunction, kCAMediaTimingFunctionEaseIn,
    kCAMediaTimingFunctionEaseInEaseOut, kCAMediaTimingFunctionEaseOut,
    kCAMediaTimingFunctionLinear,
};
use objc2_ui_kit::{
    NSIndexPathUIKitAdditions, NSLineBreakMode, NSObjectUIAccessibility, NSTextAlignment,
    UIAccessibilityIsReduceMotionEnabled, UIAction, UIActivityIndicatorView,
    UIActivityIndicatorViewStyle, UIButton, UIButtonType, UICollectionView, UICollectionViewCell,
    UICollectionViewDataSource, UICollectionViewDelegate, UICollectionViewDelegateFlowLayout,
    UICollectionViewFlowLayout, UICollectionViewLayout, UIColor, UIControl,
    UIControlContentHorizontalAlignment, UIControlEvents, UIControlState, UICubicTimingParameters,
    UIDatePicker, UIDatePickerMode, UIDatePickerStyle, UIEdgeInsets, UIFont, UIImage, UIImageView,
    UIKeyboardType, UILabel, UILayoutConstraintAxis, UIProgressView, UIReturnKeyType, UIScrollView,
    UIScrollViewDelegate, UISegmentedControl, UISlider, UISpringTimingParameters, UIStackView,
    UIStackViewAlignment, UIStackViewDistribution, UISwitch, UITextBorderStyle, UITextField,
    UITextInputTraits, UITimingCurveProvider, UIView, UIViewAnimating, UIViewAnimatingPosition,
    UIViewAnimationCurve, UIViewContentMode, UIViewPropertyAnimator,
};

use crate::animation::ease_curve;
use crate::app::{App, Event};
use crate::geometry::{Constraints, Rect, Size};
use crate::layout::LayoutResult;
use crate::measure::{DEFAULT_SWITCH_HEIGHT, DEFAULT_SWITCH_WIDTH, Measure};
use crate::render::{self, Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{self, NodeId, NodeKind, RecyclerProps, UiNode};
use crate::ui::{
    Animation, AnimationCurve, Axis, Border, ButtonProps, CheckProps, DatePickerMode,
    DatePickerProps, DatePickerValue, DateValue, EdgeInsets, Element, FontWeight, Gradient,
    GradientDirection, ImageProps, ImageScale, ImageSource, ImeAction, KeyboardType, ProgressProps,
    RadioGroupProps, SliderProps, SpinnerProps, TabBarProps, TextAlign, TextFieldProps, TextProps,
    TimeValue, dim_color, snap_slider_value,
};

/// Measures text with an off-screen `UILabel`.
///
/// Results are cached because Taffy asks for the same intrinsic sizes several
/// times per layout pass. UIKit objects are only touched from the main thread,
/// which is also where the runtime renders.
struct IosMeasure {
    label: Retained<UILabel>,
    cache: Mutex<HashMap<MeasureKey, Size>>,
    switch_size: Mutex<Option<Size>>,
    image_sizes: Mutex<HashMap<String, Option<Size>>>,
}

// SAFETY: every method is called from the main thread by `IosState::render`.
unsafe impl Send for IosMeasure {}
unsafe impl Sync for IosMeasure {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MeasureKey {
    content: String,
    size: u32,
    bold: bool,
    max_width: Option<u32>,
    max_lines: Option<u32>,
    ellipsis: bool,
}

impl IosMeasure {
    fn new(mtm: MainThreadMarker) -> Self {
        let label = UILabel::new(mtm);
        label.setAdjustsFontSizeToFitWidth(false);
        Self {
            label,
            cache: Mutex::new(HashMap::new()),
            switch_size: Mutex::new(None),
            image_sizes: Mutex::new(HashMap::new()),
        }
    }

    fn measure_with(&self, text: &TextProps, constraints: Constraints) -> Size {
        if text.content.is_empty() {
            return Size::ZERO;
        }

        let key = MeasureKey {
            content: text.content.clone(),
            size: text.size.to_bits(),
            bold: matches!(text.weight, FontWeight::Bold),
            max_width: constraints.max_width.map(f32::to_bits),
            max_lines: text.max_lines,
            ellipsis: text.ellipsis,
        };

        if let Some(cached) = self
            .cache
            .lock()
            .expect("measurement cache")
            .get(&key)
            .copied()
        {
            return cached;
        }

        let label = &self.label;
        label.setText(Some(&NSString::from_str(&text.content)));
        let font = system_font(text.size, text.weight);
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
        label.setNumberOfLines(text.max_lines.map_or(0, |lines| lines.max(1) as isize));
        label.setLineBreakMode(if text.ellipsis {
            NSLineBreakMode::ByTruncatingTail
        } else {
            NSLineBreakMode::ByWordWrapping
        });

        let max_width = constraints.max_width.map_or(f64::MAX, f64::from);
        let measured = label.sizeThatFits(CGSize::new(max_width, f64::MAX));
        let size = Size::new(measured.width.ceil() as f32, measured.height.ceil() as f32);

        self.cache
            .lock()
            .expect("measurement cache")
            .insert(key, size);
        size
    }
}

impl Measure for IosMeasure {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
        self.measure_with(text, constraints)
    }

    fn switch_size(&self) -> Size {
        if let Some(size) = *self.switch_size.lock().expect("switch size cache") {
            return size;
        }
        // The switch must be created on the main thread; layout already runs
        // there. A host build falls back to the Material default.
        let size = MainThreadMarker::new()
            .map(UISwitch::new)
            .map(|switch| switch.intrinsicContentSize())
            .filter(|size| size.width > 0.0 && size.height > 0.0)
            .map_or(
                Size::new(DEFAULT_SWITCH_WIDTH, DEFAULT_SWITCH_HEIGHT),
                |size| Size::new(size.width as f32, size.height as f32),
            );
        *self.switch_size.lock().expect("switch size cache") = Some(size);
        size
    }

    fn image_size(&self, props: &ImageProps) -> Option<Size> {
        let ImageSource::Asset(path) = &props.source else {
            return None;
        };
        if let Some(cached) = self
            .image_sizes
            .lock()
            .expect("image size cache")
            .get(path)
            .copied()
        {
            return cached;
        }
        let size = MainThreadMarker::new().and_then(|_| {
            load_asset(path).map(|image| {
                // SAFETY: the image was just loaded on the main thread.
                let size = unsafe { image.size() };
                Size::new(size.width as f32, size.height as f32)
            })
        });
        self.image_sizes
            .lock()
            .expect("image size cache")
            .insert(path.clone(), size);
        size
    }
}

fn system_font(size: f32, weight: FontWeight) -> Retained<UIFont> {
    match weight {
        FontWeight::Bold => UIFont::boldSystemFontOfSize(f64::from(size)),
        FontWeight::Normal => UIFont::systemFontOfSize(f64::from(size)),
    }
}

fn color(argb: u32) -> Retained<UIColor> {
    let alpha = f64::from((argb >> 24) & 0xFF) / 255.0;
    let red = f64::from((argb >> 16) & 0xFF) / 255.0;
    let green = f64::from((argb >> 8) & 0xFF) / 255.0;
    let blue = f64::from(argb & 0xFF) / 255.0;
    UIColor::colorWithRed_green_blue_alpha(red, green, blue, alpha)
}

fn cg_color(argb: u32) -> Retained<CGColor> {
    // SAFETY: the color is a valid `UIColor` and `CGColor` is retained.
    unsafe { color(argb).CGColor() }
}

/// The state shared between a scroll view's delegate and its deferred update.
///
/// `scrollViewDidScroll` fires for every frame of a gesture; the latest offset
/// is stored here and only one main-queue update is scheduled at a time.
#[derive(Default)]
struct ScrollShared {
    latest: AtomicU64,
    scheduled: AtomicBool,
}

/// What a scroll delegate remembers about its node.
struct ScrollDelegateIvars {
    handle: u64,
    host: u64,
    id: NodeId,
    horizontal: bool,
    shared: Arc<ScrollShared>,
}

objc2::define_class!(
    // Scroll callbacks arrive on the main thread, so the delegate is
    // main-thread-only and uses the main-thread `alloc`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikScrollDelegate"]
    #[ivars = ScrollDelegateIvars]
    struct ScrollDelegate;

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
    fn new(
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

/// The reuse identifier of a recycled-list row cell.
const ROW_CELL_ID: &str = "KrabikRow";

/// What a recycled-list data source remembers about its node.
///
/// Only the main thread touches the collection view, but the callbacks take
/// `&self`, so the mutable parts are atomics.
struct RecyclerIvars {
    handle: u64,
    host: u64,
    id: NodeId,
    /// The item count the collection view currently shows.
    count: AtomicU64,
    /// The latest item count requested by the application.
    target_count: AtomicU64,
    /// Fixed row height in points encoded as `f64` bits; `0` means automatic.
    item_height: AtomicU64,
    revision: AtomicU64,
    /// A sync block is already scheduled.
    pending: AtomicBool,
    /// The count changed and needs one insert/delete batch.
    needs_batch: AtomicBool,
    /// A row-height change needs a layout invalidation.
    needs_layout: AtomicBool,
    /// A revision change needs the visible cells rebound.
    needs_refresh: AtomicBool,
}

/// The changes one sync block should apply to a collection view.
struct RecyclerChanges {
    from: usize,
    to: usize,
    batch: bool,
    layout: bool,
    refresh: bool,
}

objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikRecyclerDataSource"]
    #[ivars = RecyclerIvars]
    struct RecyclerDataSource;

    unsafe impl NSObjectProtocol for RecyclerDataSource {}

    unsafe impl UIScrollViewDelegate for RecyclerDataSource {}

    unsafe impl UICollectionViewDataSource for RecyclerDataSource {
        #[unsafe(method(collectionView:numberOfItemsInSection:))]
        fn number_of_items_in_section(
            &self,
            _collection_view: &UICollectionView,
            _section: isize,
        ) -> isize {
            self.ivars()
                .count
                .load(Ordering::Relaxed)
                .min(isize::MAX as u64) as isize
        }

        #[unsafe(method_id(collectionView:cellForItemAtIndexPath:))]
        fn cell_for_item_at_index_path(
            &self,
            collection_view: &UICollectionView,
            index_path: &NSIndexPath,
        ) -> Retained<UICollectionViewCell> {
            let identifier = NSString::from_str(ROW_CELL_ID);
            let cell = collection_view
                .dequeueReusableCellWithReuseIdentifier_forIndexPath(&identifier, index_path);
            let ivars = self.ivars();
            let index = index_path.item().max(0) as usize;
            let width = collection_view.bounds().size.width;
            let _ = with_state(ivars.handle, ivars.host, |state| {
                state.bind_row(ivars.id, index, &cell, width);
            });
            cell
        }
    }

    unsafe impl UICollectionViewDelegate for RecyclerDataSource {
        #[unsafe(method(collectionView:didEndDisplayingCell:forItemAtIndexPath:))]
        fn did_end_displaying_cell(
            &self,
            _collection_view: &UICollectionView,
            cell: &UICollectionViewCell,
            _index_path: &NSIndexPath,
        ) {
            let key = cell as *const UICollectionViewCell as usize;
            let ivars = self.ivars();
            let _ = with_state(ivars.handle, ivars.host, |state| state.unbind_row(key));
        }
    }

    unsafe impl UICollectionViewDelegateFlowLayout for RecyclerDataSource {
        #[unsafe(method(collectionView:layout:sizeForItemAtIndexPath:))]
        fn size_for_item_at_index_path(
            &self,
            collection_view: &UICollectionView,
            _collection_view_layout: &UICollectionViewLayout,
            index_path: &NSIndexPath,
        ) -> CGSize {
            let ivars = self.ivars();
            let width = collection_view.bounds().size.width.max(1.0);
            let fixed = f64::from_bits(ivars.item_height.load(Ordering::Relaxed));
            if fixed > 0.0 {
                return CGSize::new(width, fixed);
            }

            let index = index_path.item().max(0) as usize;
            let height = with_state(ivars.handle, ivars.host, |state| {
                state.row_height(ivars.id, index, width as f32)
            })
            .unwrap_or(0.0);
            CGSize::new(width, f64::from(height).max(1.0))
        }
    }
);

impl RecyclerDataSource {
    fn new(
        handle: u64,
        host: u64,
        id: NodeId,
        props: &RecyclerProps,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RecyclerIvars {
            handle,
            host,
            id,
            count: AtomicU64::new(props.item_count as u64),
            target_count: AtomicU64::new(props.item_count as u64),
            item_height: AtomicU64::new(props.item_height.map_or(0.0, f64::from).to_bits()),
            revision: AtomicU64::new(props.revision),
            pending: AtomicBool::new(false),
            needs_batch: AtomicBool::new(false),
            needs_layout: AtomicBool::new(false),
            needs_refresh: AtomicBool::new(false),
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }

    /// Records new list properties and schedules one coalesced sync.
    ///
    /// This runs while the state lock is held, so it only touches atomics.
    /// The batch update itself is deferred: UIKit may ask the data source for
    /// cells synchronously, and those callbacks acquire the state lock.
    fn update(&self, props: &RecyclerProps) {
        let count = props.item_count as u64;
        let height = props.item_height.map_or(0.0, f64::from).to_bits();
        let ivars = self.ivars();
        let count_changed = ivars.target_count.swap(count, Ordering::Relaxed) != count;
        let height_changed = ivars.item_height.swap(height, Ordering::Relaxed) != height;
        let revision_changed =
            ivars.revision.swap(props.revision, Ordering::Relaxed) != props.revision;
        if count_changed {
            ivars.needs_batch.store(true, Ordering::Release);
        }
        if height_changed {
            ivars.needs_layout.store(true, Ordering::Release);
        }
        if revision_changed {
            ivars.needs_refresh.store(true, Ordering::Release);
        }
        if (count_changed || height_changed || revision_changed)
            && !ivars.pending.swap(true, Ordering::AcqRel)
        {
            let handle = ivars.handle;
            let host = ivars.host;
            let id = ivars.id;
            schedule(move || sync_recycler(handle, host, id));
        }
    }

    /// Atomically takes the pending work for one sync block.
    fn take_changes(&self) -> Option<RecyclerChanges> {
        let ivars = self.ivars();
        // Clear the scheduled flag first: if an update lands while the batch is
        // running, it schedules another sync instead of being dropped.
        ivars.pending.store(false, Ordering::Release);

        let batch_flag = ivars.needs_batch.swap(false, Ordering::AcqRel);
        let layout = ivars.needs_layout.swap(false, Ordering::AcqRel);
        let refresh = ivars.needs_refresh.swap(false, Ordering::AcqRel);

        let from = ivars.count.load(Ordering::Acquire) as usize;
        let target = ivars.target_count.load(Ordering::Acquire) as usize;
        let batch = batch_flag && from != target;
        if batch {
            // The data source must report the new count while the batch runs.
            ivars.count.store(target as u64, Ordering::Release);
        }

        if !batch && !layout && !refresh {
            return None;
        }
        Some(RecyclerChanges {
            from,
            to: target,
            batch,
            layout,
            refresh,
        })
    }
}

/// The typed content of a native view.
///
/// Leaves that need internal padding keep a separate content view inside the
/// styled outer view; containers keep a content host that children are added
/// to. Keeping the concrete class here avoids downcasting on every update.
enum Content {
    None,
    Label(Retained<UILabel>),
    Field(Retained<UITextField>),
    Button(Retained<UIButton>),
    Switch(Retained<UISwitch>),
    Check {
        button: Retained<UIButton>,
        radio: bool,
    },
    RadioGroup {
        stack: Retained<UIStackView>,
        buttons: Vec<Retained<UIButton>>,
        labels: Vec<String>,
    },
    DatePicker(Retained<UIDatePicker>),
    TabBar(Retained<UISegmentedControl>),
    Pager {
        scroll: Retained<UIScrollView>,
        _delegate: Retained<PagerDelegate>,
        pages: usize,
        selected: usize,
    },
    Slider(Retained<UISlider>),
    Progress(Retained<UIProgressView>),
    Spinner(Retained<UIActivityIndicatorView>),
    Image(Retained<UIImageView>),
    Host(Retained<UIView>),
    Scroll {
        scroll: Retained<UIScrollView>,
        host: Retained<UIView>,
        /// Keeps the delegate alive for as long as the scroll view exists.
        _delegate: Retained<ScrollDelegate>,
    },
    Recycler {
        collection: Retained<UICollectionView>,
        /// UIKit holds its data source weakly, so the handle owns it.
        data_source: Retained<RecyclerDataSource>,
    },
}

impl Content {
    /// A retained handle to the content view inset inside its wrapper.
    fn inset_handle(&self) -> Option<Retained<UIView>> {
        match self {
            Self::Label(view) => Some(cast_view(view.clone())),
            Self::Field(view) => Some(cast_view(view.clone())),
            _ => None,
        }
    }

    fn children_host(&self, fallback: &Retained<UIView>) -> Retained<UIView> {
        match self {
            Self::Host(host) => host.clone(),
            Self::Scroll { host, .. } => host.clone(),
            Self::Pager { scroll, .. } => cast_view(scroll.clone()),
            _ => fallback.clone(),
        }
    }
}

/// The native resources owned by one renderable node.
struct ViewHandle {
    /// The view that draws the node itself.
    view: Retained<UIView>,
    /// The inner content, when the node has one.
    content: Content,
    /// The gradient layer drawn behind the content, when the node has one.
    gradient: Option<Retained<CAGradientLayer>>,
    /// The nearest renderable ancestor the view lives under.
    parent: Option<NodeId>,
    /// The last relative frame applied to the view.
    frame: Option<Rect>,
    /// The node's padding, used to inset the content.
    padding: EdgeInsets,
    /// The display link driving a frame animation, when one is running.
    frame_link: Option<Retained<CADisplayLink>>,
}

impl ViewHandle {
    fn new(view: Retained<UIView>, content: Content, node: &RenderNode) -> Self {
        Self {
            view,
            content,
            gradient: None,
            parent: node.parent,
            frame: None,
            padding: node.style.padding,
            frame_link: None,
        }
    }

    /// The view that hosts the node's native children.
    fn children_host(&self) -> Retained<UIView> {
        self.content.children_host(&self.view)
    }
}

/// What a pager delegate remembers about its node.
struct PagerDelegateIvars {
    handle: u64,
    host: u64,
    id: NodeId,
}

objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikPagerDelegate"]
    #[ivars = PagerDelegateIvars]
    struct PagerDelegate;

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

fn report_pager_page(ivars: &PagerDelegateIvars, scroll_view: &UIScrollView) {
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
    fn new(handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(PagerDelegateIvars { handle, host, id });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }
}

/// Remote image state for one root view.
#[derive(Default)]
struct ImageLoader {
    /// Decoded images keyed by URL. Grows for the session's lifetime, which
    /// is fine for a demo-sized number of remote images.
    cache: HashMap<String, Retained<UIImage>>,
    /// URLs with a download in flight, so repeated renders do not refetch.
    in_flight: HashSet<String>,
    /// The URL each image node is currently expected to display. A download
    /// only updates views whose entry still matches its URL.
    expected: HashMap<NodeId, String>,
}

/// The actions of one recycled row, keyed by the node ids it created.
#[derive(Clone, Default)]
struct RowAction {
    click: Option<Cow<'static, str>>,
    text: Option<Cow<'static, str>>,
    submit: Option<Cow<'static, str>>,
    focus: Option<Cow<'static, str>>,
    toggle: Option<Cow<'static, str>>,
    selected: Option<Cow<'static, str>>,
    date: Option<Cow<'static, str>>,
    date_range: Option<(crate::ui::DateValue, crate::ui::DateValue)>,
    time_range: Option<(crate::ui::TimeValue, crate::ui::TimeValue)>,
    value: Option<Cow<'static, str>>,
    value_min: f32,
    value_max: f32,
    value_step: Option<f32>,
}

/// One recycled row currently bound to a collection-view cell.
///
/// Cells are reused by UIKit, so the state is keyed by the cell's identity and
/// diffed when that cell shows another item or new content.
struct IosRowState {
    list: NodeId,
    index: usize,
    tree: UiNode,
    layout: LayoutResult,
    views: HashMap<NodeId, ViewHandle>,
}

/// What one native iOS host renders.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum IosHostKind {
    Main,
    Screen(String),
    Modal,
}

/// The views and frames updated by one layout operation.
///
/// The values are owned so the same work can run directly or inside a UIKit
/// animation block.
struct LayoutFrames {
    view: Retained<UIView>,
    inset: Option<Retained<UIView>>,
    host: Option<Retained<UIView>>,
    scroll: Option<(Retained<UIScrollView>, Retained<UIView>, Size)>,
    gradient: Option<Retained<CAGradientLayer>>,
    padding: EdgeInsets,
}

impl LayoutFrames {
    /// Applies the frames with the view at `rect`.
    ///
    /// Frame animations call this on every display-link tick, so the model
    /// frames always match what the user sees and hit-testing stays exact.
    fn apply_rect(&self, rect: CGRect) {
        self.view.setFrame(rect);

        if let Some(content) = &self.inset {
            content.setFrame(inset(rect.size, self.padding));
        }
        if let Some(host) = &self.host {
            host.setFrame(inset(rect.size, self.padding));
        }
        if let Some((scroll, host, size)) = &self.scroll {
            host.setFrame(CGRect::new(
                CGPoint::new(f64::from(self.padding.left), f64::from(self.padding.top)),
                CGSize::new(
                    f64::from(size.width.max(0.0)),
                    f64::from(size.height.max(0.0)),
                ),
            ));
            scroll.setContentSize(CGSize::new(
                f64::from(self.padding.left + size.width + self.padding.right),
                f64::from(self.padding.top + size.height + self.padding.bottom),
            ));
        }
        if let Some(gradient) = &self.gradient {
            gradient.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), rect.size));
        }
    }
}

/// Interpolates between two frames.
fn lerp_rect(from: CGRect, to: CGRect, progress: f64) -> CGRect {
    let lerp = |a: f64, b: f64| a + (b - a) * progress;
    CGRect::new(
        CGPoint::new(
            lerp(from.origin.x, to.origin.x),
            lerp(from.origin.y, to.origin.y),
        ),
        CGSize::new(
            lerp(from.size.width, to.size.width),
            lerp(from.size.height, to.size.height),
        ),
    )
}

/// Drives a frame animation from the display link.
struct FrameTickerIvars {
    frames: LayoutFrames,
    from: CGRect,
    to: CGRect,
    start: CFTimeInterval,
    duration: f64,
    curve: AnimationCurve,
}

objc2::define_class!(
    // The ticker and its retained views live on the main thread.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikFrameTicker"]
    #[ivars = FrameTickerIvars]
    struct FrameTicker;

    unsafe impl NSObjectProtocol for FrameTicker {}

    impl FrameTicker {
        #[unsafe(method(tick:))]
        fn tick(&self, link: &CADisplayLink) {
            let ivars = self.ivars();
            let progress = if ivars.duration <= 0.0 {
                1.0
            } else {
                ((link.timestamp() - ivars.start) / ivars.duration).clamp(0.0, 1.0)
            };
            let eased = f64::from(ease_curve(&ivars.curve, progress as f32));
            ivars
                .frames
                .apply_rect(lerp_rect(ivars.from, ivars.to, eased));
            if progress >= 1.0 {
                link.invalidate();
            }
        }
    }
);

impl FrameTicker {
    /// Starts a frame tween and returns the display link that drives it.
    ///
    /// The link retains the ticker for as long as it runs, so the caller only
    /// keeps the link in order to cancel it.
    fn start(
        frames: LayoutFrames,
        from: CGRect,
        to: CGRect,
        duration_ms: u32,
        curve: AnimationCurve,
        mtm: MainThreadMarker,
    ) -> Retained<CADisplayLink> {
        let ticker = Self::alloc(mtm).set_ivars(FrameTickerIvars {
            frames,
            from,
            to,
            start: CACurrentMediaTime(),
            duration: f64::from(duration_ms) / 1000.0,
            curve,
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        let ticker: Retained<Self> = unsafe { objc2::msg_send![super(ticker), init] };
        // SAFETY: the ticker implements `tick:` and is retained by the link.
        let link =
            unsafe { CADisplayLink::displayLinkWithTarget_selector(&ticker, objc2::sel!(tick:)) };
        // SAFETY: the run loop and the static common mode both outlive the link.
        unsafe { link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes) };
        link
    }
}

/// Whether a transition with this duration should run on this device.
fn animation_is_enabled(duration_ms: u32) -> bool {
    duration_ms > 0 && !UIAccessibilityIsReduceMotionEnabled()
}

/// Builds UIKit timing parameters for one of the shared curves.
fn timing_parameters(
    curve: &AnimationCurve,
    mtm: MainThreadMarker,
) -> Retained<ProtocolObject<dyn UITimingCurveProvider>> {
    match curve {
        AnimationCurve::Linear => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::Linear,
            ))
        }
        AnimationCurve::EaseIn => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseIn,
            ))
        }
        AnimationCurve::EaseOut => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseOut,
            ))
        }
        AnimationCurve::EaseInOut => {
            ProtocolObject::from_retained(UICubicTimingParameters::initWithAnimationCurve(
                UICubicTimingParameters::alloc(mtm),
                UIViewAnimationCurve::EaseInOut,
            ))
        }
        AnimationCurve::CubicBezier { x1, y1, x2, y2 } => ProtocolObject::from_retained(
            UICubicTimingParameters::initWithControlPoint1_controlPoint2(
                UICubicTimingParameters::alloc(mtm),
                CGPoint::new(f64::from(*x1), f64::from(*y1)),
                CGPoint::new(f64::from(*x2), f64::from(*y2)),
            ),
        ),
        AnimationCurve::Spring { damping, response } => {
            let response = f64::from(*response).max(0.01);
            let stiffness = (2.0 * std::f64::consts::PI / response).powi(2);
            let damping = 2.0 * f64::from(*damping) * stiffness.sqrt();
            ProtocolObject::from_retained(
                UISpringTimingParameters::initWithMass_stiffness_damping_initialVelocity(
                    UISpringTimingParameters::alloc(mtm),
                    1.0,
                    stiffness,
                    damping,
                    CGVector::new(0.0, 0.0),
                ),
            )
        }
    }
}

/// Runs a UIKit property animation and optionally calls `completion`.
fn run_animation(
    duration_ms: u32,
    curve: &AnimationCurve,
    animations: RcBlock<dyn Fn()>,
    completion: Option<RcBlock<dyn Fn(UIViewAnimatingPosition)>>,
    mtm: MainThreadMarker,
) {
    let parameters = timing_parameters(curve, mtm);
    let animator = UIViewPropertyAnimator::initWithDuration_timingParameters(
        UIViewPropertyAnimator::alloc(mtm),
        f64::from(duration_ms) / 1000.0,
        &parameters,
    );
    animator.addAnimations(&animations);
    if let Some(completion) = completion {
        let block = RcBlock::new(move |position: UIViewAnimatingPosition| {
            completion.call((position,));
        });
        animator.addCompletion(&block);
    }
    UIViewAnimating::startAnimation(&*animator);
}

/// The Core Animation timing function for a curve.
fn timing_function(curve: &AnimationCurve) -> Retained<CAMediaTimingFunction> {
    match curve {
        AnimationCurve::Linear => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionLinear })
        }
        AnimationCurve::EaseIn => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseIn })
        }
        AnimationCurve::EaseOut => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseOut })
        }
        AnimationCurve::EaseInOut => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseInEaseOut })
        }
        AnimationCurve::CubicBezier { x1, y1, x2, y2 } => {
            CAMediaTimingFunction::functionWithControlPoints(*x1, *y1, *x2, *y2)
        }
        // Core Animation has no spring timing function in these bindings;
        // layer-property changes fall back to the default ease.
        AnimationCurve::Spring { .. } => {
            CAMediaTimingFunction::functionWithName(unsafe { kCAMediaTimingFunctionEaseInEaseOut })
        }
    }
}

/// Adds a Core Animation interpolation from `from` to `to` on a layer key.
fn add_layer_animation(
    layer: &CALayer,
    key_path: &str,
    from: Option<&AnyObject>,
    to: Option<&AnyObject>,
    duration_ms: u32,
    curve: &AnimationCurve,
) {
    let animation = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str(key_path)));
    // SAFETY: the values are toll-free bridged or boxed objects matching the
    // key path's value type.
    unsafe {
        animation.setFromValue(from);
        animation.setToValue(to);
    }
    animation.setDuration(f64::from(duration_ms) / 1000.0);
    animation.setTimingFunction(Some(&timing_function(curve)));
    layer.addAnimation_forKey(&animation, Some(&NSString::from_str(key_path)));
}

struct StyleSnapshot {
    background: Option<Retained<UIColor>>,
    border_color: Option<Retained<CGColor>>,
    border_width: f64,
    corner_radius: f64,
    gradient_colors: Option<Retained<NSArray>>,
    gradient_start: Option<CGPoint>,
    gradient_end: Option<CGPoint>,
    shadow_opacity: f32,
    shadow_radius: f64,
    shadow_offset: CGSize,
    alpha: f64,
}

fn style_snapshot(
    view: &Retained<UIView>,
    gradient: Option<&Retained<CAGradientLayer>>,
) -> StyleSnapshot {
    let layer = view.layer();
    StyleSnapshot {
        background: view.backgroundColor(),
        border_color: layer.borderColor(),
        border_width: layer.borderWidth(),
        corner_radius: layer.cornerRadius(),
        gradient_colors: gradient.and_then(|gradient| gradient.colors()),
        gradient_start: gradient.map(|gradient| gradient.startPoint()),
        gradient_end: gradient.map(|gradient| gradient.endPoint()),
        shadow_opacity: layer.shadowOpacity(),
        shadow_radius: layer.shadowRadius(),
        shadow_offset: layer.shadowOffset(),
        alpha: view.alpha(),
    }
}

/// Boxes a point for a Core Animation value.
fn box_point(point: CGPoint) -> Retained<NSValue> {
    // SAFETY: both pointers are valid for the duration of the call; the type
    // encoding matches the pointed-to `CGPoint`.
    unsafe {
        NSValue::valueWithBytes_objCType(
            NonNull::new_unchecked((&point as *const CGPoint).cast_mut().cast()),
            NonNull::new_unchecked(c"{CGPoint=dd}".as_ptr().cast_mut()),
        )
    }
}

/// Boxes a size for a Core Animation value.
fn box_size(size: CGSize) -> Retained<NSValue> {
    // SAFETY: both pointers are valid for the duration of the call; the type
    // encoding matches the pointed-to `CGSize`.
    unsafe {
        NSValue::valueWithBytes_objCType(
            NonNull::new_unchecked((&size as *const CGSize).cast_mut().cast()),
            NonNull::new_unchecked(c"{CGSize=dd}".as_ptr().cast_mut()),
        )
    }
}

/// Animates a style update: background and alpha through UIKit, layer
/// properties through Core Animation.
fn animate_style_change(
    view: &Retained<UIView>,
    gradient: Option<&Retained<CAGradientLayer>>,
    node: &RenderNode,
    snapshot: StyleSnapshot,
    animation: &Animation,
    mtm: MainThreadMarker,
) {
    let target_background = view.backgroundColor();
    let target_alpha = f64::from(node.style.opacity.unwrap_or(1.0));

    view.setBackgroundColor(snapshot.background.as_deref());
    view.setAlpha(snapshot.alpha);

    let view_for_block = view.clone();
    let block = RcBlock::new(move || {
        view_for_block.setBackgroundColor(target_background.as_deref());
        view_for_block.setAlpha(target_alpha);
    });
    run_animation(animation.duration_ms, &animation.curve, block, None, mtm);

    let layer = view.layer();

    let from_color = snapshot
        .border_color
        .map(|color| unsafe { Retained::cast_unchecked(color) });
    let to_color = layer
        .borderColor()
        .map(|color| unsafe { Retained::cast_unchecked(color) });
    add_layer_animation(
        &layer,
        "borderColor",
        from_color.as_deref(),
        to_color.as_deref(),
        animation.duration_ms,
        &animation.curve,
    );

    let from_width: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.border_width)) };
    let to_width: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.borderWidth())) };
    add_layer_animation(
        &layer,
        "borderWidth",
        Some(&from_width),
        Some(&to_width),
        animation.duration_ms,
        &animation.curve,
    );

    let from_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.corner_radius)) };
    let to_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.cornerRadius())) };
    add_layer_animation(
        &layer,
        "cornerRadius",
        Some(&from_radius),
        Some(&to_radius),
        animation.duration_ms,
        &animation.curve,
    );

    if let (Some(from_colors), Some(gradient)) = (snapshot.gradient_colors, gradient)
        && let Some(to_colors) = gradient.colors()
    {
        let from_colors: Retained<AnyObject> = unsafe { Retained::cast_unchecked(from_colors) };
        let to_colors: Retained<AnyObject> = unsafe { Retained::cast_unchecked(to_colors) };
        add_layer_animation(
            gradient,
            "colors",
            Some(&from_colors),
            Some(&to_colors),
            animation.duration_ms,
            &animation.curve,
        );

        // Gradient direction is expressed by its endpoints, so animating them
        // turns a direction change into a smooth sweep.
        if let Some(from) = snapshot.gradient_start {
            let from = box_point(from);
            let to = box_point(gradient.startPoint());
            add_layer_animation(
                gradient,
                "startPoint",
                Some(&from),
                Some(&to),
                animation.duration_ms,
                &animation.curve,
            );
        }
        if let Some(from) = snapshot.gradient_end {
            let from = box_point(from);
            let to = box_point(gradient.endPoint());
            add_layer_animation(
                gradient,
                "endPoint",
                Some(&from),
                Some(&to),
                animation.duration_ms,
                &animation.curve,
            );
        }
    }

    let from_shadow_opacity: Retained<AnyObject> = unsafe {
        Retained::cast_unchecked(NSNumber::numberWithDouble(f64::from(
            snapshot.shadow_opacity,
        )))
    };
    let to_shadow_opacity: Retained<AnyObject> = unsafe {
        Retained::cast_unchecked(NSNumber::numberWithDouble(f64::from(layer.shadowOpacity())))
    };
    add_layer_animation(
        &layer,
        "shadowOpacity",
        Some(&from_shadow_opacity),
        Some(&to_shadow_opacity),
        animation.duration_ms,
        &animation.curve,
    );

    let from_shadow_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(snapshot.shadow_radius)) };
    let to_shadow_radius: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(NSNumber::numberWithDouble(layer.shadowRadius())) };
    add_layer_animation(
        &layer,
        "shadowRadius",
        Some(&from_shadow_radius),
        Some(&to_shadow_radius),
        animation.duration_ms,
        &animation.curve,
    );

    let from_shadow_offset: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(box_size(snapshot.shadow_offset)) };
    let to_shadow_offset: Retained<AnyObject> =
        unsafe { Retained::cast_unchecked(box_size(layer.shadowOffset())) };
    add_layer_animation(
        &layer,
        "shadowOffset",
        Some(&from_shadow_offset),
        Some(&to_shadow_offset),
        animation.duration_ms,
        &animation.curve,
    );
}

struct IosState {
    handle: u64,
    host: u64,
    kind: IosHostKind,
    runtime: Runtime,
    /// A weak reference so the Rust state does not keep the Swift host view
    /// alive and prevent its `deinit` from destroying the state.
    root: Weak<UIView>,
    views: HashMap<NodeId, ViewHandle>,
    images: ImageLoader,
    /// Actions of the rows currently bound in recycled lists.
    row_actions: HashMap<NodeId, RowAction>,
    /// The node ids a (list, row) binding created, for clearing on rebind.
    row_action_ids: HashMap<(NodeId, usize), Vec<NodeId>>,
    /// The rows currently bound to collection-view cells, by cell identity.
    row_states: HashMap<usize, IosRowState>,
    /// The item each row node belongs to, for routing focus back to its row.
    row_nodes: HashMap<NodeId, (NodeId, usize)>,
    /// Views animating out after their node was removed, by node id.
    pending_exits: HashMap<NodeId, Retained<UIView>>,
    viewport: Size,
}

// SAFETY: all access is serialized by the global mutex and happens on the main
// thread. `Retained<UIView>` is not `Send` on its own, which is what this
// assertion documents.
unsafe impl Send for IosState {}

impl IosState {
    fn new(
        handle: u64,
        host: u64,
        kind: IosHostKind,
        root: &Retained<UIView>,
        mtm: MainThreadMarker,
    ) -> Self {
        Self {
            handle,
            host,
            kind,
            runtime: Runtime::new(Box::new(IosMeasure::new(mtm))),
            root: Weak::from_retained(root),
            views: HashMap::new(),
            images: ImageLoader::default(),
            row_actions: HashMap::new(),
            row_action_ids: HashMap::new(),
            row_states: HashMap::new(),
            row_nodes: HashMap::new(),
            pending_exits: HashMap::new(),
            viewport: Size::ZERO,
        }
    }

    fn render(&mut self, element: &crate::ui::Element) {
        let update = self.runtime.update(element, self.viewport);
        let Some(root) = self.root.load() else {
            return;
        };

        // The main views are taken out while the operations run so a recycled
        // row can apply its own operations without borrowing the state twice.
        let mut views = std::mem::take(&mut self.views);
        for op in update.ops {
            self.apply_to(op, &mut views, &root, true);
        }
        self.views = views;
        self.prune_rows();
    }

    /// Applies one renderer operation to a set of views.
    ///
    /// The main tree and a recycled row share this code; they only differ in
    /// the view that receives top-level widgets.
    fn apply_to(
        &mut self,
        op: Op,
        views: &mut HashMap<NodeId, ViewHandle>,
        root_host: &Retained<UIView>,
        animate: bool,
    ) {
        // SAFETY: the runtime renders on the main thread, and every entry point
        // checks that before locking the state.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        match op {
            Op::Create(node) => {
                // A deferred exit may still own this node id. Drop it before
                // the replacement appears.
                if let Some(old) = self.pending_exits.remove(&node.id) {
                    old.layer().removeAllAnimations();
                    old.removeFromSuperview();
                }

                let handle = self.create_handle(&node, mtm);
                let host = parent_host_in(views, root_host, node.parent);
                host.addSubview(&handle.view);
                let view = handle.view.clone();
                views.insert(node.id, handle);

                if animate
                    && let Some(enter) = node
                        .style
                        .enter
                        .as_ref()
                        .filter(|enter| animation_is_enabled(enter.duration_ms))
                {
                    let target_alpha = f64::from(node.style.opacity.unwrap_or(1.0));
                    view.setAlpha(0.0);
                    view.setTransform(CGAffineTransform {
                        a: 0.96,
                        b: 0.0,
                        c: 0.0,
                        d: 0.96,
                        tx: 0.0,
                        ty: 0.0,
                    });
                    let view_for_block = view.clone();
                    let block = RcBlock::new(move || {
                        view_for_block.setAlpha(target_alpha);
                        view_for_block.setTransform(CGAffineTransform {
                            a: 1.0,
                            b: 0.0,
                            c: 0.0,
                            d: 1.0,
                            tx: 0.0,
                            ty: 0.0,
                        });
                    });
                    // The view already occupies its final frame, so hits land
                    // where the user expects while it fades in.
                    run_animation(enter.duration_ms, &enter.curve, block, None, mtm);
                }
            }
            Op::Update(node) => {
                if let Some(handle) = views.get_mut(&node.id) {
                    let animation = if animate {
                        node.style
                            .animation
                            .as_ref()
                            .filter(|animation| animation_is_enabled(animation.duration_ms))
                    } else {
                        None
                    };
                    let snapshot =
                        animation.map(|_| style_snapshot(&handle.view, handle.gradient.as_ref()));
                    update_content(&mut handle.content, &node, self.handle, self.host, mtm);
                    apply_style(&handle.view, &node, &mut handle.gradient);
                    apply_accessibility(&handle.view, &node, mtm);
                    if let (Some(animation), Some(snapshot)) = (animation, snapshot) {
                        animate_style_change(
                            &handle.view,
                            handle.gradient.as_ref(),
                            &node,
                            snapshot,
                            animation,
                            mtm,
                        );
                    }
                }
                // Image updates can start downloads, which needs the state
                // itself, so they are handled separately from `update_content`.
                if let NodeKind::Image(props) = &node.kind {
                    let view = match views.get(&node.id) {
                        Some(handle) => match &handle.content {
                            Content::Image(image) => Some(image.clone()),
                            _ => None,
                        },
                        None => None,
                    };
                    if let Some(view) = view {
                        self.load_image(node.id, &view, props);
                    }
                }
                if let NodeKind::Recycler(props) = &node.kind
                    && let Some(handle) = views.get(&node.id)
                    && let Content::Recycler { data_source, .. } = &handle.content
                {
                    data_source.update(props);
                }
            }
            Op::Reparent { id, parent } => {
                let Some(old_parent) = views.get(&id).map(|handle| handle.parent) else {
                    return;
                };
                if old_parent != parent {
                    let view = views[&id].view.clone();
                    let host = parent_host_in(views, root_host, parent);
                    view.removeFromSuperview();
                    host.addSubview(&view);
                    if let Some(handle) = views.get_mut(&id) {
                        handle.parent = parent;
                    }
                }
            }
            Op::Remove { id, exit } => {
                self.images.expected.remove(&id);
                if let Some(handle) = views.remove(&id) {
                    if let Some(link) = &handle.frame_link {
                        link.invalidate();
                    }
                    if animate
                        && let Some(exit) = exit
                            .as_ref()
                            .filter(|exit| animation_is_enabled(exit.duration_ms))
                    {
                        let view = handle.view.clone();
                        self.pending_exits.insert(id, view.clone());
                        view.setUserInteractionEnabled(false);
                        let remove = view.clone();
                        let block = RcBlock::new(move || {
                            remove.setAlpha(0.0);
                            remove.setTransform(CGAffineTransform {
                                a: 0.96,
                                b: 0.0,
                                c: 0.0,
                                d: 0.96,
                                tx: 0.0,
                                ty: 0.0,
                            });
                        });
                        let completion = {
                            let view = view.clone();
                            let token = Retained::as_ptr(&view) as usize;
                            let handle = self.handle;
                            let host = self.host;
                            RcBlock::new(move |_: UIViewAnimatingPosition| {
                                view.removeFromSuperview();
                                schedule(move || {
                                    with_session(handle, |session| {
                                        if let Some(state) = session.hosts.get_mut(&host)
                                            && state.pending_exits.get(&id).is_some_and(|view| {
                                                Retained::as_ptr(view) as usize == token
                                            })
                                        {
                                            state.pending_exits.remove(&id);
                                        }
                                    });
                                });
                            })
                        };
                        run_animation(exit.duration_ms, &exit.curve, block, Some(completion), mtm);
                    } else {
                        handle.view.removeFromSuperview();
                    }
                }
            }
            Op::Layout {
                id,
                frame,
                content_size,
                animation,
            } => IosState::apply_layout(views, id, frame, content_size, animation, mtm),
        }
    }

    fn create_handle(&mut self, node: &RenderNode, mtm: MainThreadMarker) -> ViewHandle {
        let mut handle = match &node.kind {
            NodeKind::Text(props) => {
                let wrapper = UIView::new(mtm);
                let label = UILabel::new(mtm);
                update_label(&label, props);
                wrapper.addSubview(&label);
                ViewHandle::new(wrapper, Content::Label(label), node)
            }
            NodeKind::TextField(props) => {
                let wrapper = UIView::new(mtm);
                let field = UITextField::new(mtm);
                field.setBorderStyle(UITextBorderStyle::None);
                update_field(&field, props);
                wire_field(&field, self.handle, self.host, node.id, mtm);
                wrapper.addSubview(&field);
                ViewHandle::new(wrapper, Content::Field(field), node)
            }
            NodeKind::Button(props) => {
                let button = UIButton::buttonWithType(UIButtonType::System, mtm);
                update_button(&button, props);
                wire_click(&button, self.handle, self.host, node.id, mtm);
                let view = cast_view(button.clone());
                ViewHandle::new(view, Content::Button(button), node)
            }
            NodeKind::Switch(props) => {
                let switch = UISwitch::new(mtm);
                switch.setOn(props.checked);
                wire_switch(&switch, self.handle, self.host, node.id, mtm);
                let view = cast_view(switch.clone());
                ViewHandle::new(view, Content::Switch(switch), node)
            }
            NodeKind::Checkbox(props) => {
                let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
                update_check(&button, props, false);
                wire_check(&button, self.handle, self.host, node.id, mtm);
                let view = cast_view(button.clone());
                ViewHandle::new(
                    view,
                    Content::Check {
                        button,
                        radio: false,
                    },
                    node,
                )
            }
            NodeKind::RadioButton(props) => {
                let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
                update_check(&button, props, true);
                wire_check(&button, self.handle, self.host, node.id, mtm);
                let view = cast_view(button.clone());
                ViewHandle::new(
                    view,
                    Content::Check {
                        button,
                        radio: true,
                    },
                    node,
                )
            }
            NodeKind::RadioGroup(props) => {
                let stack = UIStackView::new(mtm);
                stack.setAxis(UILayoutConstraintAxis::Vertical);
                stack.setAlignment(UIStackViewAlignment::Fill);
                stack.setDistribution(UIStackViewDistribution::FillEqually);
                let mut buttons = Vec::new();
                let mut labels = Vec::new();
                update_radio_group(
                    &stack,
                    &mut buttons,
                    &mut labels,
                    props,
                    self.handle,
                    self.host,
                    node.id,
                    mtm,
                );
                let view = cast_view(stack.clone());
                ViewHandle::new(
                    view,
                    Content::RadioGroup {
                        stack,
                        buttons,
                        labels,
                    },
                    node,
                )
            }
            NodeKind::DatePicker(props) => {
                let picker = UIDatePicker::new(mtm);
                update_date_picker(&picker, props);
                wire_date_picker(&picker, self.handle, self.host, node.id, mtm);
                let view = cast_view(picker.clone());
                ViewHandle::new(view, Content::DatePicker(picker), node)
            }
            NodeKind::TabBar(props) => {
                let control = UISegmentedControl::new(mtm);
                update_tab_bar(&control, props);
                wire_tab_bar(&control, self.handle, self.host, node.id, mtm);
                let view = cast_view(control.clone());
                ViewHandle::new(view, Content::TabBar(control), node)
            }
            NodeKind::Pager(props) => {
                let scroll = UIScrollView::new(mtm);
                scroll.setPagingEnabled(true);
                scroll.setShowsHorizontalScrollIndicator(false);
                scroll.setBounces(false);
                let delegate = PagerDelegate::new(self.handle, self.host, node.id, mtm);
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
            NodeKind::Slider(props) => {
                let slider = UISlider::new(mtm);
                update_slider(&slider, props);
                wire_slider(&slider, self.handle, self.host, node.id, mtm);
                let view = cast_view(slider.clone());
                ViewHandle::new(view, Content::Slider(slider), node)
            }
            NodeKind::Progress(props) => {
                let progress = UIProgressView::new(mtm);
                update_progress(&progress, props);
                let view = cast_view(progress.clone());
                ViewHandle::new(view, Content::Progress(progress), node)
            }
            NodeKind::Spinner(props) => {
                let spinner = UIActivityIndicatorView::initWithActivityIndicatorStyle(
                    UIActivityIndicatorView::alloc(mtm),
                    UIActivityIndicatorViewStyle::Medium,
                );
                update_spinner(&spinner, props);
                let view = cast_view(spinner.clone());
                ViewHandle::new(view, Content::Spinner(spinner), node)
            }
            NodeKind::Image(props) => {
                let image = UIImageView::new(mtm);
                self.load_image(node.id, &image, props);
                let view = cast_view(image.clone());
                ViewHandle::new(view, Content::Image(image), node)
            }
            NodeKind::Scroll(props) => {
                let scroll = UIScrollView::new(mtm);
                scroll.setClipsToBounds(true);
                scroll.setShowsVerticalScrollIndicator(props.axis == Axis::Vertical);
                scroll.setShowsHorizontalScrollIndicator(props.axis == Axis::Horizontal);
                scroll.setAlwaysBounceVertical(props.axis == Axis::Vertical);
                scroll.setAlwaysBounceHorizontal(props.axis == Axis::Horizontal);
                let host = UIView::new(mtm);
                scroll.addSubview(&host);
                let delegate = ScrollDelegate::new(
                    self.handle,
                    self.host,
                    node.id,
                    props.axis == Axis::Horizontal,
                    mtm,
                );
                // SAFETY: the delegate is owned by the view handle below and
                // only reads the scroll view's offset.
                unsafe { scroll.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
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
            NodeKind::Recycler(props) => {
                let layout = UICollectionViewFlowLayout::new(mtm);
                layout.setMinimumLineSpacing(0.0);
                layout.setMinimumInteritemSpacing(0.0);
                layout.setSectionInset(UIEdgeInsets {
                    top: 0.0,
                    left: 0.0,
                    bottom: 0.0,
                    right: 0.0,
                });
                let collection = UICollectionView::initWithFrame_collectionViewLayout(
                    UICollectionView::alloc(mtm),
                    CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0)),
                    &layout,
                );
                collection.setShowsVerticalScrollIndicator(true);
                collection.setAlwaysBounceVertical(true);
                let identifier = NSString::from_str(ROW_CELL_ID);
                // SAFETY: the cell class is a `UICollectionViewCell`, which is
                // what the dequeue call below expects.
                unsafe {
                    collection.registerClass_forCellWithReuseIdentifier(
                        Some(UICollectionViewCell::class()),
                        &identifier,
                    );
                }
                let data_source =
                    RecyclerDataSource::new(self.handle, self.host, node.id, props, mtm);
                collection.setDataSource(Some(ProtocolObject::from_ref(&*data_source)));
                // SAFETY: the data source is kept alive by the view handle and
                // only reads the collection view's size and index paths.
                unsafe {
                    collection.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
                }
                let view = cast_view(collection.clone());
                ViewHandle::new(
                    view,
                    Content::Recycler {
                        collection,
                        data_source,
                    },
                    node,
                )
            }
            NodeKind::Column | NodeKind::Row => {
                let view = UIView::new(mtm);
                let host = UIView::new(mtm);
                view.addSubview(&host);
                ViewHandle::new(view, Content::Host(host), node)
            }
            // A decorated spacer needs a view for its background or border.
            _ => {
                let view = UIView::new(mtm);
                ViewHandle::new(view, Content::None, node)
            }
        };

        apply_style(&handle.view, node, &mut handle.gradient);
        apply_accessibility(&handle.view, node, mtm);
        handle
    }

    fn apply_layout(
        views: &mut HashMap<NodeId, ViewHandle>,
        id: NodeId,
        frame: Rect,
        content_size: Option<Size>,
        animation: Option<Animation>,
        mtm: MainThreadMarker,
    ) {
        let Some(handle) = views.get_mut(&id) else {
            return;
        };

        let rect = CGRect::new(
            CGPoint::new(f64::from(frame.x), f64::from(frame.y)),
            CGSize::new(
                f64::from(frame.width.max(0.0)),
                f64::from(frame.height.max(0.0)),
            ),
        );
        // The previous frame is the starting point of an implicit animation.
        let from = handle.frame.map_or(rect, |frame| {
            CGRect::new(
                CGPoint::new(f64::from(frame.x), f64::from(frame.y)),
                CGSize::new(
                    f64::from(frame.width.max(0.0)),
                    f64::from(frame.height.max(0.0)),
                ),
            )
        });
        handle.frame = Some(frame);

        let pager = match &handle.content {
            Content::Pager {
                scroll,
                pages,
                selected,
                ..
            } => Some((scroll.clone(), *pages, *selected)),
            _ => None,
        };

        let frames = LayoutFrames {
            view: handle.view.clone(),
            inset: handle.content.inset_handle(),
            host: match &handle.content {
                Content::Host(host) => Some(host.clone()),
                _ => None,
            },
            scroll: match &handle.content {
                Content::Scroll { scroll, host, .. } => Some((
                    scroll.clone(),
                    host.clone(),
                    content_size.unwrap_or(Size::ZERO),
                )),
                _ => None,
            },
            gradient: handle.gradient.clone(),
            padding: handle.padding,
        };

        if let Some(link) = handle.frame_link.take() {
            link.invalidate();
        }
        if let Some(animation) = animation
            .as_ref()
            .filter(|animation| animation_is_enabled(animation.duration_ms))
        {
            // A display link updates the model frames every tick, so a tap
            // during the animation hits the view where it actually is.
            let link = FrameTicker::start(
                frames,
                from,
                rect,
                animation.duration_ms,
                animation.curve.clone(),
                mtm,
            );
            handle.frame_link = Some(link);
        } else {
            frames.apply_rect(rect);
        }

        if let Some((scroll, pages, selected)) = pager {
            let previous = scroll.contentSize().width;
            let content = CGSize::new(rect.size.width * pages as f64, rect.size.height);
            scroll.setContentSize(content);
            if (previous - content.width).abs() > 0.5 {
                scroll.setContentOffset_animated(
                    CGPoint::new(rect.size.width * selected as f64, 0.0),
                    false,
                );
            }
        }
    }

    /// Configures an image view for the given properties.
    ///
    /// Assets load synchronously; remote URLs load through the cache and
    /// `NSURLSession`, updating the view when the download finishes.
    fn load_image(&mut self, id: NodeId, view: &UIImageView, props: &ImageProps) {
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
    fn load_remote_image(&mut self, id: NodeId, view: &UIImageView, url: &str) {
        self.images.expected.insert(id, url.to_string());
        if let Some(image) = self.images.cache.get(url).map(|image| &**image) {
            view.setImage(Some(image));
            return;
        }
        if self.images.in_flight.insert(url.to_string()) {
            start_download(self.handle, self.host, id, url);
        }
    }

    /// The element of one recycled row, or `None` when the item vanished.
    fn row_element(&self, list: NodeId, index: usize) -> Option<Element> {
        let source = self.runtime.row_source(list)?;
        (index < source.item_count).then(|| (source.rows)(index))
    }

    /// Builds and lays out one recycled row with its own root identifier.
    fn row_layout(&self, list: NodeId, index: usize, width: f32) -> Option<(UiNode, LayoutResult)> {
        let element = self.row_element(list, index)?;
        let root = tree::row_root(list, index);
        let (tree, layout) =
            self.runtime
                .layout_subtree(&element, root, Size::new(width.max(1.0), 100_000.0));
        Some((tree, layout))
    }

    /// The height a variable-height row needs at the given width.
    fn row_height(&self, list: NodeId, index: usize, width: f32) -> f32 {
        let Some((_, layout)) = self.row_layout(list, index, width) else {
            return 0.0;
        };
        layout
            .frames
            .get(&tree::row_root(list, index))
            .map_or(0.0, |frame| frame.height.max(0.0))
    }

    /// The collection view and data source of a recycled list.
    fn recycler_for(
        &self,
        id: NodeId,
    ) -> Option<(Retained<UICollectionView>, Retained<RecyclerDataSource>)> {
        match &self.views.get(&id)?.content {
            Content::Recycler {
                collection,
                data_source,
            } => Some((collection.clone(), data_source.clone())),
            _ => None,
        }
    }

    /// The collection view of a recycled list.
    fn collection_for(&self, id: NodeId) -> Option<Retained<UICollectionView>> {
        self.recycler_for(id).map(|(collection, _)| collection)
    }

    /// Diffs every visible cell in place after an application state change.
    fn rebind_visible_rows(&mut self, id: NodeId) {
        let Some(collection) = self.collection_for(id) else {
            return;
        };
        let width = collection.bounds().size.width;
        let paths = collection.indexPathsForVisibleItems();
        for path in paths.iter() {
            let index = path.item().max(0) as usize;
            if let Some(cell) = collection.cellForItemAtIndexPath(&path) {
                self.bind_row(id, index, &cell, width);
            }
        }
        // Content changes can alter a row's measured height.
        collection.collectionViewLayout().invalidateLayout();
    }

    /// Builds or updates one recycled row inside a collection-view cell.
    fn bind_row(&mut self, list: NodeId, index: usize, cell: &UICollectionViewCell, width: f64) {
        let Some((tree, layout)) = self.row_layout(list, index, width as f32) else {
            return;
        };
        let transition_ms = self
            .runtime
            .row_source(list)
            .map_or(0, |props| props.row_transition_ms);
        let content = cell.contentView();
        let key = cell as *const UICollectionViewCell as usize;

        let previous = self.row_states.remove(&key);
        let had_previous = previous.is_some();
        let rebinding = previous
            .as_ref()
            .is_some_and(|state| state.list == list && state.index == index);
        let (old_tree, old_layout, mut views) = if let Some(state) = previous {
            self.clear_row_actions(state.list, state.index);
            if !rebinding
                && let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                // The cell is about to show another item; keep the logical
                // focus for when the row comes back.
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            (state.tree, state.layout, state.views)
        } else {
            // The cell was recycled behind our back; start from an empty tree.
            remove_subviews(&content);
            (UiNode::empty(), LayoutResult::default(), HashMap::new())
        };

        let mut ops = render::diff(&old_tree, &tree);
        ops.extend(render::diff_layout(&old_tree, &tree, &old_layout, &layout));

        // The snapshot shows the old content while the new widgets are laid
        // out underneath. The cell stays available for reuse immediately.
        let snapshot = if animation_is_enabled(transition_ms) && had_previous && !ops.is_empty() {
            content.snapshotViewAfterScreenUpdates(false)
        } else {
            None
        };

        for op in ops {
            self.apply_to(op, &mut views, &content, false);
        }

        if let Some(snapshot) = snapshot
            && let Some(mtm) = MainThreadMarker::new()
        {
            content.addSubview(&snapshot);
            let fading = snapshot.clone();
            let block = RcBlock::new(move || fading.setAlpha(0.0));
            let removing = snapshot;
            let completion = RcBlock::new(move |_: UIViewAnimatingPosition| {
                removing.removeFromSuperview();
            });
            run_animation(
                transition_ms,
                &AnimationCurve::EaseOut,
                block,
                Some(completion),
                mtm,
            );
        }

        self.record_row_actions(list, index, &tree);
        self.register_row_nodes(list, index, &tree);
        self.row_states.insert(
            key,
            IosRowState {
                list,
                index,
                tree,
                layout,
                views,
            },
        );

        if !rebinding
            && let Some(focus) = self.runtime.row_focus()
            && focus.list == list
            && focus.index == index
        {
            self.try_focus_field(focus);
        }
    }

    /// Releases the widgets and actions of the row a cell currently shows.
    fn unbind_row(&mut self, key: usize) {
        if let Some(state) = self.row_states.remove(&key) {
            for handle in state.views.values() {
                if let Some(link) = &handle.frame_link {
                    link.invalidate();
                }
            }
            if let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            self.clear_row_actions(state.list, state.index);
        }
    }

    /// Maps every node of a bound row back to its item.
    fn register_row_nodes(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.insert(node.id, (list, index));
        }
    }

    /// Forgets the nodes of a row that is no longer bound.
    fn unregister_row_nodes(&mut self, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.remove(&node.id);
        }
    }

    /// Whether a row field is still in a window, and so still really focused.
    fn field_attached(&self, id: NodeId) -> bool {
        self.row_states.values().any(|row| {
            row.views
                .get(&id)
                .is_some_and(|handle| match &handle.content {
                    Content::Field(field) => field.window().is_some(),
                    _ => handle.view.window().is_some(),
                })
        })
    }

    /// Makes a row field the first responder, if it can be.
    fn focus_field(&self, id: NodeId) -> bool {
        for row in self.row_states.values() {
            if let Some(handle) = row.views.get(&id)
                && let Content::Field(field) = &handle.content
            {
                return field.becomeFirstResponder();
            }
        }
        false
    }

    /// Restores the logical focus of a row that was just (re)bound.
    fn try_focus_field(&mut self, focus: crate::runtime::RowFocus) {
        if self.focus_field(focus.field) {
            self.runtime
                .set_row_focus(focus.list, focus.index, focus.field, true);
        } else {
            // The cell is not in the window yet; try again next turn.
            let handle = self.handle;
            let host = self.host;
            let field = focus.field;
            schedule(move || focus_row_async(handle, host, field));
        }
    }

    /// Forgets actions and rows whose list or item no longer exists.
    fn prune_rows(&mut self) {
        let stale: Vec<((NodeId, usize), Vec<NodeId>)> = self
            .row_action_ids
            .iter()
            .filter(|((list, index), _)| {
                self.runtime
                    .recycled_item_count(*list)
                    .is_none_or(|count| *index >= count)
            })
            .map(|(key, ids)| (*key, ids.clone()))
            .collect();
        for (key, ids) in stale {
            self.row_action_ids.remove(&key);
            for id in ids {
                self.row_actions.remove(&id);
            }
        }

        let detached: Vec<usize> = self
            .row_states
            .iter()
            .filter(|(_, row)| self.runtime.recycled_item_count(row.list).is_none())
            .map(|(key, _)| *key)
            .collect();
        for key in detached {
            if let Some(state) = self.row_states.remove(&key) {
                self.unregister_row_nodes(&state.tree);
            }
        }
    }

    /// Drops the actions recorded for one row binding.
    fn clear_row_actions(&mut self, list: NodeId, index: usize) {
        if let Some(ids) = self.row_action_ids.remove(&(list, index)) {
            for id in ids {
                self.row_actions.remove(&id);
            }
        }
    }

    fn record_row_actions(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        let mut ids = Vec::new();
        for node in tree::preorder(tree) {
            let mut action = RowAction::default();
            let mut any = false;
            match &node.kind {
                NodeKind::Button(props) if !props.action.is_empty() => {
                    action.click = Some(props.action.clone());
                    any = true;
                }
                NodeKind::Switch(props) if !props.action.is_empty() => {
                    action.toggle = Some(props.action.clone());
                    any = true;
                }
                NodeKind::Checkbox(props) if !props.action.is_empty() => {
                    action.toggle = Some(props.action.clone());
                    any = true;
                }
                NodeKind::RadioButton(props) if !props.action.is_empty() => {
                    action.toggle = Some(props.action.clone());
                    any = true;
                }
                NodeKind::RadioGroup(props) if !props.action.is_empty() => {
                    action.selected = Some(props.action.clone());
                    any = true;
                }
                NodeKind::TabBar(props) if !props.action.is_empty() => {
                    action.selected = Some(props.action.clone());
                    any = true;
                }
                NodeKind::Pager(props) if !props.action.is_empty() => {
                    action.selected = Some(props.action.clone());
                    any = true;
                }
                NodeKind::DatePicker(props) if !props.action.is_empty() => {
                    action.date = Some(props.action.clone());
                    action.date_range = props.date_range;
                    action.time_range = props.time_range;
                    any = true;
                }
                NodeKind::Slider(props) if !props.action.is_empty() => {
                    action.value = Some(props.action.clone());
                    action.value_min = props.min;
                    action.value_max = props.max;
                    action.value_step = props.step;
                    any = true;
                }
                NodeKind::TextField(props) => {
                    if !props.action.is_empty() {
                        action.text = Some(props.action.clone());
                        any = true;
                    }
                    if !props.submit_action.is_empty() {
                        action.submit = Some(props.submit_action.clone());
                        any = true;
                    }
                    if !props.focus_action.is_empty() {
                        action.focus = Some(props.focus_action.clone());
                        any = true;
                    }
                }
                _ => {}
            }
            if any {
                self.row_actions.insert(node.id, action);
                ids.push(node.id);
            }
        }
        self.row_action_ids.insert((list, index), ids);
    }
}

/// The view that hosts the children of the given native parent.
fn parent_host_in(
    views: &HashMap<NodeId, ViewHandle>,
    root: &Retained<UIView>,
    parent: Option<NodeId>,
) -> Retained<UIView> {
    match parent.and_then(|id| views.get(&id)) {
        Some(handle) => handle.children_host(),
        None => root.clone(),
    }
}

/// Removes every subview, used before recreating a reused cell's row.
fn remove_subviews(view: &UIView) {
    for subview in view.subviews().iter() {
        subview.removeFromSuperview();
    }
}

fn cast_view<T: Message>(view: Retained<T>) -> Retained<UIView> {
    // SAFETY: every caller passes a `UIView` subclass.
    unsafe { Retained::cast_unchecked(view) }
}

fn inset(size: CGSize, padding: EdgeInsets) -> CGRect {
    let left = f64::from(padding.left);
    let top = f64::from(padding.top);
    let right = f64::from(padding.right);
    let bottom = f64::from(padding.bottom);
    CGRect::new(
        CGPoint::new(left, top),
        CGSize::new(
            (size.width - left - right).max(0.0),
            (size.height - top - bottom).max(0.0),
        ),
    )
}

fn update_content(
    content: &mut Content,
    node: &RenderNode,
    handle: u64,
    host: u64,
    mtm: MainThreadMarker,
) {
    match (content, &node.kind) {
        (Content::Label(label), NodeKind::Text(props)) => update_label(label, props),
        (Content::Field(field), NodeKind::TextField(props)) => update_field(field, props),
        (Content::Button(button), NodeKind::Button(props)) => update_button(button, props),
        (Content::Switch(switch), NodeKind::Switch(props)) => switch.setOn(props.checked),
        (Content::Check { button, radio }, NodeKind::Checkbox(props)) => {
            update_check(button, props, *radio);
        }
        (Content::Check { button, radio }, NodeKind::RadioButton(props)) => {
            update_check(button, props, *radio);
        }
        (
            Content::RadioGroup {
                stack,
                buttons,
                labels,
            },
            NodeKind::RadioGroup(props),
        ) => update_radio_group(stack, buttons, labels, props, handle, host, node.id, mtm),
        (Content::DatePicker(picker), NodeKind::DatePicker(props)) => {
            update_date_picker(picker, props);
        }
        (Content::TabBar(control), NodeKind::TabBar(props)) => update_tab_bar(control, props),
        (
            Content::Pager {
                scroll,
                pages,
                selected,
                ..
            },
            NodeKind::Pager(props),
        ) => {
            *pages = props.pages;
            let target = if props.pages == 0 {
                0
            } else {
                props.selected.min(props.pages - 1)
            };
            if *selected != target {
                *selected = target;
                let width = scroll.bounds().size.width;
                if width > 0.0 {
                    scroll
                        .setContentOffset_animated(CGPoint::new(width * target as f64, 0.0), true);
                }
            }
        }
        (Content::Slider(slider), NodeKind::Slider(props)) => update_slider(slider, props),
        (Content::Progress(progress), NodeKind::Progress(props)) => {
            update_progress(progress, props);
        }
        (Content::Spinner(spinner), NodeKind::Spinner(props)) => update_spinner(spinner, props),
        // Images are updated by `IosState::load_image`, which starts async
        // downloads and therefore needs the state itself.
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn update_radio_group(
    stack: &UIStackView,
    buttons: &mut Vec<Retained<UIButton>>,
    labels: &mut Vec<String>,
    props: &RadioGroupProps,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    if *labels != props.options {
        for button in buttons.iter() {
            button.removeFromSuperview();
        }
        buttons.clear();
        labels.clear();
        for (index, label) in props.options.iter().enumerate() {
            let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
            wire_radio_group_item(&button, handle, host, id, index, mtm);
            buttons.push(button);
            labels.push(label.clone());
        }
        for button in buttons.iter() {
            stack.addArrangedSubview(button);
        }
    }

    stack.setSpacing(f64::from(props.spacing));
    for (index, (button, label)) in buttons.iter().zip(props.options.iter()).enumerate() {
        update_check(
            button,
            &CheckProps {
                label: label.clone(),
                checked: props.selected == Some(index),
                action: Cow::Borrowed(""),
                size: props.size,
                color: props.color,
                weight: props.weight,
            },
            true,
        );
        let accessibility = NSString::from_str(label);
        button.setAccessibilityLabel(Some(&accessibility), mtm);
    }
}

fn update_label(label: &UILabel, props: &TextProps) {
    label.setText(Some(&NSString::from_str(&props.content)));
    let font = system_font(props.size, props.weight);
    // SAFETY: the font is retained by the label for as long as it is set.
    unsafe { label.setFont(Some(&font)) };
    let text_color = color(props.color);
    // SAFETY: the color is retained by the label for as long as it is set.
    unsafe { label.setTextColor(Some(&text_color)) };
    label.setNumberOfLines(props.max_lines.map_or(0, |lines| lines.max(1) as isize));
    label.setLineBreakMode(if props.ellipsis {
        NSLineBreakMode::ByTruncatingTail
    } else {
        NSLineBreakMode::ByWordWrapping
    });
    label.setTextAlignment(match props.align {
        TextAlign::Start => NSTextAlignment(0),
        TextAlign::Center => NSTextAlignment(1),
        TextAlign::End => NSTextAlignment(2),
    });
}

fn update_button(button: &UIButton, props: &ButtonProps) {
    let title = NSString::from_str(&props.label);
    button.setTitle_forState(Some(&title), UIControlState::Normal);
    let text_color = color(props.color);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Normal);

    if let Some(label) = button.titleLabel() {
        let font = system_font(props.size, props.weight);
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
    }
}

fn update_check(button: &UIButton, props: &CheckProps, radio: bool) {
    // The leading space separates the symbol from the label; the builder sets
    // the clean accessibility label.
    let title = NSString::from_str(&format!(" {}", props.label));
    button.setTitle_forState(Some(&title), UIControlState::Normal);
    let text_color = color(props.color);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Normal);
    button.setTitleColor_forState(Some(&text_color), UIControlState::Selected);
    if let Some(label) = button.titleLabel() {
        let font = system_font(props.size, props.weight);
        // SAFETY: the font is retained by the label for as long as it is set.
        unsafe { label.setFont(Some(&font)) };
    }

    let (unchecked, checked) = if radio {
        ("circle", "smallcircle.filled.circle")
    } else {
        ("square", "checkmark.square.fill")
    };
    let symbol = if props.checked { checked } else { unchecked };
    if let Some(image) = UIImage::systemImageNamed(&NSString::from_str(symbol)) {
        button.setImage_forState(Some(&image), UIControlState::Normal);
    }
    button.setImage_forState(None, UIControlState::Selected);
    button.setSelected(props.checked);
    button.setContentHorizontalAlignment(UIControlContentHorizontalAlignment::Left);
    // SAFETY: the color is retained by the view for as long as it is set.
    unsafe { button.setTintColor(Some(&text_color)) };
}

fn update_tab_bar(control: &UISegmentedControl, props: &TabBarProps) {
    let current: Vec<String> = (0..control.numberOfSegments())
        .filter_map(|index| {
            control
                .titleForSegmentAtIndex(index)
                .map(|title| title.to_string())
        })
        .collect();
    if current != props.tabs {
        control.removeAllSegments();
        for (index, tab) in props.tabs.iter().enumerate() {
            let title = NSString::from_str(tab);
            control.insertSegmentWithTitle_atIndex_animated(Some(&title), index, false);
        }
    }
    let selected = if props.tabs.is_empty() {
        -1
    } else {
        props.selected.min(props.tabs.len() - 1) as isize
    };
    if control.selectedSegmentIndex() != selected {
        control.setSelectedSegmentIndex(selected);
    }
    // SAFETY: the color is a valid `UIColor` for the lifetime of the call.
    unsafe { control.setTintColor(Some(&color(props.color))) };
}

fn update_date_picker(picker: &UIDatePicker, props: &DatePickerProps) {
    let mode = match props.value {
        DatePickerValue::Date(_) => UIDatePickerMode::Date,
        DatePickerValue::Time(_) => UIDatePickerMode::Time,
        DatePickerValue::DateTime { .. } => UIDatePickerMode::DateAndTime,
    };
    picker.setDatePickerMode(mode);
    picker.setPreferredDatePickerStyle(match props.mode {
        DatePickerMode::Compact => UIDatePickerStyle::Compact,
        DatePickerMode::Inline => UIDatePickerStyle::Inline,
    });

    let current = picker_value_from_nsdate(&picker.date(), mode);
    if current != Some(props.value)
        && let Some(date) = nsdate_from_picker_value(props.value)
    {
        picker.setDate(&date);
    }

    // A date picker bounds whole days, so local noon keeps the calendar day
    // stable. A date-and-time picker bounds the exact instants: the start of
    // the first day and the end of the last day unless a time range narrows it.
    let (minimum, maximum) = match props.value {
        DatePickerValue::Date(_) => (
            props
                .date_range
                .and_then(|(min, _)| nsdate_from_date_value(min)),
            props
                .date_range
                .and_then(|(_, max)| nsdate_from_date_value(max)),
        ),
        DatePickerValue::DateTime { .. } => (
            props
                .date_range
                .map(|(min, _)| DatePickerValue::DateTime {
                    date: min,
                    time: props
                        .time_range
                        .map_or(TimeValue::new(0, 0), |(min, _)| min),
                })
                .and_then(nsdate_from_picker_value),
            props
                .date_range
                .map(|(_, max)| DatePickerValue::DateTime {
                    date: max,
                    time: props
                        .time_range
                        .map_or(TimeValue::new(23, 59), |(_, max)| max),
                })
                .and_then(nsdate_from_picker_value),
        ),
        DatePickerValue::Time(_) => (None, None),
    };
    picker.setMinimumDate(minimum.as_deref());
    picker.setMaximumDate(maximum.as_deref());
}

/// Builds a `Date` at local noon so DST transitions cannot shift the calendar day.
fn nsdate_from_date_value(value: DateValue) -> Option<Retained<NSDate>> {
    let components = NSDateComponents::new();
    components.setYear(value.year as isize);
    components.setMonth(value.month as isize);
    components.setDay(value.day as isize);
    components.setHour(12);
    NSCalendar::currentCalendar().dateFromComponents(&components)
}

fn nsdate_from_picker_value(value: DatePickerValue) -> Option<Retained<NSDate>> {
    let components = NSDateComponents::new();
    match value {
        DatePickerValue::Date(date) => {
            components.setYear(date.year as isize);
            components.setMonth(date.month as isize);
            components.setDay(date.day as isize);
            components.setHour(12);
        }
        DatePickerValue::Time(time) => {
            components.setYear(2000);
            components.setMonth(1);
            components.setDay(1);
            components.setHour(time.hour as isize);
            components.setMinute(time.minute as isize);
        }
        DatePickerValue::DateTime { date, time } => {
            components.setYear(date.year as isize);
            components.setMonth(date.month as isize);
            components.setDay(date.day as isize);
            components.setHour(time.hour as isize);
            components.setMinute(time.minute as isize);
        }
    }
    NSCalendar::currentCalendar().dateFromComponents(&components)
}

fn picker_value_from_nsdate(date: &NSDate, mode: UIDatePickerMode) -> Option<DatePickerValue> {
    let units = NSCalendarUnit::Year
        | NSCalendarUnit::Month
        | NSCalendarUnit::Day
        | NSCalendarUnit::Hour
        | NSCalendarUnit::Minute;
    let components = NSCalendar::currentCalendar().components_fromDate(units, date);
    let date = DateValue::new(
        components.year() as i32,
        components.month() as u8,
        components.day() as u8,
    );
    let time = TimeValue::new(components.hour() as u8, components.minute() as u8);
    if mode == UIDatePickerMode::Time {
        Some(DatePickerValue::Time(time))
    } else if mode == UIDatePickerMode::DateAndTime {
        Some(DatePickerValue::DateTime { date, time })
    } else {
        Some(DatePickerValue::Date(date))
    }
}

fn update_slider(slider: &UISlider, props: &SliderProps) {
    let max = props.max.max(props.min + 0.001);
    slider.setMinimumValue(props.min);
    slider.setMaximumValue(max);
    let value = snap_slider_value(props.value, props.min, max, props.step);
    if (slider.value() - value).abs() > f32::EPSILON {
        slider.setValue(value);
    }
    let tint = color(props.color);
    slider.setMinimumTrackTintColor(Some(&tint));
}

fn update_progress(progress: &UIProgressView, props: &ProgressProps) {
    progress.setProgress(props.value.clamp(0.0, 1.0));
    let tint = color(props.color);
    progress.setProgressTintColor(Some(&tint));
}

fn update_spinner(spinner: &UIActivityIndicatorView, props: &SpinnerProps) {
    let tint = color(props.color);
    // SAFETY: the color is retained by the view for as long as it is set.
    unsafe { spinner.setColor(Some(&tint)) };
    spinner.startAnimating();
}

fn update_field(field: &UITextField, props: &TextFieldProps) {
    let current = field
        .text()
        .map(|text| text.to_string())
        .unwrap_or_default();
    if current != props.value {
        field.setText(Some(&NSString::from_str(&props.value)));
    }
    // The remaining setters reconfigure the text input system, which can
    // disturb active typing; skip them when the values already match so a
    // re-render between keystrokes stays a no-op.
    let placeholder = field.placeholder().map(|text| text.to_string());
    if placeholder.as_deref() != Some(props.placeholder.as_str()) {
        field.setPlaceholder(Some(&NSString::from_str(&props.placeholder)));
    }
    field.setFont(Some(&system_font(props.size, props.weight)));
    field.setTextColor(Some(&color(props.color)));
    let keyboard_type = match props.keyboard {
        KeyboardType::Text => UIKeyboardType::Default,
        KeyboardType::Email => UIKeyboardType::EmailAddress,
        KeyboardType::Number => UIKeyboardType::NumberPad,
        KeyboardType::Phone => UIKeyboardType::PhonePad,
    };
    let return_key_type = match props.ime_action {
        ImeAction::Default => UIReturnKeyType::Default,
        ImeAction::Done => UIReturnKeyType::Done,
        ImeAction::Next => UIReturnKeyType::Next,
        ImeAction::Search => UIReturnKeyType::Search,
        ImeAction::Send => UIReturnKeyType::Send,
    };
    let (current_keyboard, current_return_key, current_secure) = text_input_traits(field);
    if current_keyboard != keyboard_type.0 || current_return_key != return_key_type.0 {
        set_text_input_traits(field, keyboard_type, return_key_type);
    }
    if current_secure != props.secure {
        field.setSecureTextEntry(props.secure);
    }
}

/// Reads the keyboard, return-key, and secure-entry traits through the raw
/// runtime.
///
/// Mirrors [`set_text_input_traits`]: `UITextField` does not declare these
/// `UITextInputTraits` accessors itself, so objc2's debug-time method
/// verification rejects the generated getters even though messaging works.
fn text_input_traits(field: &UITextField) -> (isize, isize, bool) {
    let receiver = field as *const UITextField as *mut AnyObject;
    // SAFETY: all three getters take no arguments and return an integer
    // (`NSInteger` / `BOOL`), which arrives in `x0` on arm64.
    unsafe {
        let send_integer: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> isize =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        let send_bool: unsafe extern "C-unwind" fn(*mut AnyObject, Sel) -> i8 =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        (
            send_integer(receiver, objc2::sel!(keyboardType)),
            send_integer(receiver, objc2::sel!(returnKeyType)),
            send_bool(receiver, objc2::sel!(isSecureTextEntry)) != 0,
        )
    }
}

/// Sets the keyboard and return-key traits through the raw runtime.
///
/// UIKit implements these `UITextInputTraits` setters through message
/// forwarding, so objc2's debug-time method lookup does not find them even
/// though messaging works.
fn set_text_input_traits(
    field: &UITextField,
    keyboard_type: UIKeyboardType,
    return_key_type: UIReturnKeyType,
) {
    let receiver = field as *const UITextField as *mut AnyObject;
    // SAFETY: both setters take one `NSInteger` argument and return `void`.
    unsafe {
        let send: unsafe extern "C-unwind" fn(*mut AnyObject, Sel, isize) =
            std::mem::transmute(objc2::ffi::objc_msgSend as *const ());
        send(receiver, objc2::sel!(setKeyboardType:), keyboard_type.0);
        send(receiver, objc2::sel!(setReturnKeyType:), return_key_type.0);
    }
}

fn load_asset(path: &str) -> Option<Retained<UIImage>> {
    let (stem, extension) = match path.rsplit_once('.') {
        Some((stem, extension)) => (stem, Some(NSString::from_str(extension))),
        None => (path, None),
    };
    let stem = NSString::from_str(stem);
    let path = NSBundle::mainBundle().pathForResource_ofType(Some(&stem), extension.as_deref())?;
    UIImage::imageWithContentsOfFile(&path)
}

fn apply_style(view: &UIView, node: &RenderNode, gradient: &mut Option<Retained<CAGradientLayer>>) {
    let style = &node.style;
    // Native text and controls dim themselves; only the decoration the DSL
    // paints is dimmed here.
    let enabled = style.enabled.unwrap_or(true);
    let background = style.background.map(|value| dim_color(value, enabled));
    let gradient_style = style.gradient.map(|gradient| Gradient {
        start: dim_color(gradient.start, enabled),
        end: dim_color(gradient.end, enabled),
        ..gradient
    });
    let border_style = style.border.map(|border| Border {
        color: dim_color(border.color, enabled),
        ..border
    });
    let elevation = if enabled { style.elevation } else { 0.0 };
    let layer = view.layer();

    let radii = style.corner_radii();
    let radius = f64::from(
        radii
            .top_left
            .max(radii.top_right)
            .max(radii.bottom_right)
            .max(radii.bottom_left),
    );
    let mut corners = CACornerMask::empty();
    if radii.top_left > 0.0 {
        corners |= CACornerMask::LayerMinXMinYCorner;
    }
    if radii.top_right > 0.0 {
        corners |= CACornerMask::LayerMaxXMinYCorner;
    }
    if radii.bottom_right > 0.0 {
        corners |= CACornerMask::LayerMaxXMaxYCorner;
    }
    if radii.bottom_left > 0.0 {
        corners |= CACornerMask::LayerMinXMaxYCorner;
    }
    layer.setCornerRadius(radius);
    layer.setMaskedCorners(corners);

    let fills_frame =
        matches!(&node.kind, NodeKind::Image(props) if props.scale == ImageScale::Fill);
    let clips = matches!(&node.kind, NodeKind::Scroll(_) | NodeKind::Recycler(_))
        || fills_frame
        || (radius > 0.0 && elevation <= 0.0);
    view.setClipsToBounds(clips);

    if gradient_style.is_some() {
        view.setBackgroundColor(None);
    } else {
        view.setBackgroundColor(background.map(color).as_deref());
    }

    if let Some(shape) = gradient_style {
        let gradient_layer = gradient.get_or_insert_with(CAGradientLayer::new);
        if gradient_layer.superlayer().is_none() {
            view.layer().insertSublayer_atIndex(gradient_layer, 0);
        }

        // `CAGradientLayer.colors` is an `NSArray<id>` of toll-free bridged
        // `CGColor` objects, so the elements must be typed as `AnyObject` for
        // the runtime to see the `^@` element encoding.
        let start: Retained<AnyObject> = unsafe { Retained::cast_unchecked(cg_color(shape.start)) };
        let end: Retained<AnyObject> = unsafe { Retained::cast_unchecked(cg_color(shape.end)) };
        let colors: Retained<NSArray> = NSArray::from_retained_slice(&[start, end]);
        // SAFETY: the array holds two valid `CGColor` objects.
        unsafe { gradient_layer.setColors(Some(&colors)) };
        match shape.direction {
            GradientDirection::Horizontal => {
                gradient_layer.setStartPoint(CGPoint::new(0.0, 0.5));
                gradient_layer.setEndPoint(CGPoint::new(1.0, 0.5));
            }
            GradientDirection::Vertical => {
                gradient_layer.setStartPoint(CGPoint::new(0.5, 0.0));
                gradient_layer.setEndPoint(CGPoint::new(0.5, 1.0));
            }
        }
        gradient_layer.setCornerRadius(radius);
        gradient_layer.setMaskedCorners(corners);
        gradient_layer.setMasksToBounds(true);
        gradient_layer.setFrame(view.bounds());
    } else if let Some(gradient_layer) = gradient.take() {
        gradient_layer.removeFromSuperlayer();
    }

    if let Some(border) = border_style {
        let border_color = cg_color(border.color);
        layer.setBorderWidth(f64::from(border.width));
        layer.setBorderColor(Some(&border_color));
    } else {
        layer.setBorderWidth(0.0);
        layer.setBorderColor(None);
    }

    view.setAlpha(f64::from(style.opacity.unwrap_or(1.0)));

    if elevation > 0.0 {
        // SAFETY: `CGColor` is retained for as long as the layer keeps it.
        let shadow = unsafe { UIColor::blackColor().CGColor() };
        layer.setShadowColor(Some(&shadow));
        layer.setShadowOpacity(0.18);
        layer.setShadowRadius(f64::from(elevation));
        layer.setShadowOffset(CGSize::new(0.0, f64::from(elevation / 2.0)));
    } else {
        layer.setShadowOpacity(0.0);
    }
}

/// Applies interaction state and accessibility metadata to a native view.
fn apply_accessibility(view: &UIView, node: &RenderNode, mtm: MainThreadMarker) {
    if let Some(label) = &node.style.accessibility_label {
        let label = NSString::from_str(label);
        view.setAccessibilityLabel(Some(&label), mtm);
        // A radio group must stay a plain container so its options remain
        // individually reachable by accessibility services.
        view.setIsAccessibilityElement(
            !matches!(node.kind, NodeKind::RadioGroup(_) | NodeKind::TabBar(_)),
            mtm,
        );
    } else {
        view.setAccessibilityLabel(None, mtm);
    }

    let enabled = node.style.enabled.unwrap_or(true);
    view.setUserInteractionEnabled(enabled);
    if let Some(control) = as_any(view).downcast_ref::<UIControl>() {
        control.setEnabled(enabled);
    }
}

fn as_any(view: &UIView) -> &AnyObject {
    // SAFETY: every Objective-C object can be viewed as `AnyObject`.
    unsafe { &*(view as *const UIView as *const AnyObject) }
}

fn add_action(
    control: &UIControl,
    events: UIControlEvents,
    identifier: &str,
    mtm: MainThreadMarker,
    handler: impl Fn(&UIAction) + 'static,
) {
    let block: RcBlock<dyn Fn(NonNull<UIAction>)> =
        RcBlock::new(move |action: NonNull<UIAction>| {
            // SAFETY: the block is invoked with a live action by UIKit.
            handler(unsafe { action.as_ref() });
        });
    let title = NSString::from_str("");
    let identifier = NSString::from_str(identifier);
    // SAFETY: the block is a valid `UIActionHandler` and is retained by the
    // action, which is retained by the control.
    let action = unsafe {
        UIAction::actionWithTitle_image_identifier_handler(
            &title,
            None,
            Some(&identifier),
            RcBlock::as_ptr(&block),
            mtm,
        )
    };
    control.addAction_forControlEvents(&action, events);
}

fn wire_click(button: &UIButton, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.click",
        mtm,
        move |_| schedule(move || dispatch_click(handle, host, id)),
    );
}

fn wire_switch(switch: &UISwitch, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        switch,
        UIControlEvents::ValueChanged,
        "krabik.switch",
        mtm,
        move |action| {
            let checked = action
                .sender()
                .and_then(|sender| sender.downcast::<UISwitch>().ok())
                .map(|switch| switch.isOn())
                .unwrap_or(false);
            schedule(move || dispatch_checked(handle, host, id, checked));
        },
    );
}

fn wire_check(button: &UIButton, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.check",
        mtm,
        move |action| {
            let checked = action
                .sender()
                .and_then(|sender| sender.downcast::<UIButton>().ok())
                .map(|button| !button.isSelected())
                .unwrap_or(false);
            schedule(move || dispatch_checked(handle, host, id, checked));
        },
    );
}

fn wire_radio_group_item(
    button: &UIButton,
    handle: u64,
    host: u64,
    id: NodeId,
    index: usize,
    mtm: MainThreadMarker,
) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.radio-group",
        mtm,
        move |_| schedule(move || dispatch_selected(handle, host, id, index)),
    );
}

fn wire_tab_bar(
    control: &UISegmentedControl,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        control,
        UIControlEvents::ValueChanged,
        "krabik.tab-bar",
        mtm,
        move |action| {
            let index = action
                .sender()
                .and_then(|sender| sender.downcast::<UISegmentedControl>().ok())
                .map(|control| control.selectedSegmentIndex());
            if let Some(index) = index
                && index >= 0
            {
                schedule(move || dispatch_selected(handle, host, id, index as usize));
            }
        },
    );
}

fn wire_date_picker(
    picker: &UIDatePicker,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        picker,
        UIControlEvents::ValueChanged,
        "krabik.picker",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UIDatePicker>().ok())
                .and_then(|picker| {
                    picker_value_from_nsdate(&picker.date(), picker.datePickerMode())
                });
            if let Some(value) = value {
                schedule(move || dispatch_picker(handle, host, id, value));
            }
        },
    );
}

fn wire_slider(slider: &UISlider, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        slider,
        UIControlEvents::ValueChanged,
        "krabik.slider",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UISlider>().ok())
                .map(|slider| slider.value())
                .unwrap_or(0.0);
            schedule(move || dispatch_value(handle, host, id, f64::from(value)));
        },
    );
}

fn wire_field(field: &UITextField, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        field,
        UIControlEvents::EditingChanged,
        "krabik.text",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UITextField>().ok())
                .and_then(|field| field.text())
                .map(|text| text.to_string())
                .unwrap_or_default();
            schedule(move || dispatch_text(handle, host, id, &value));
        },
    );
    add_action(
        field,
        UIControlEvents::EditingDidEndOnExit,
        "krabik.submit",
        mtm,
        move |_| schedule(move || dispatch_submitted(handle, host, id)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidBegin,
        "krabik.focus-begin",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, host, id, true)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidEnd,
        "krabik.focus-end",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, host, id, false)),
    );
}

/// Defers an event to the next main-queue turn.
///
/// UIKit can deliver another action synchronously while we are rebuilding the
/// tree (for example `EditingDidEnd` when a focused field is removed). Running
/// the handler asynchronously keeps the state lock from being re-entered.
fn schedule(work: impl FnOnce() + Send + 'static) {
    DispatchQueue::main().exec_async(work);
}

/// Applies pending count, height and content changes to a recycled list.
///
/// UIKit may ask the data source for cells synchronously during a batch
/// update; those callbacks acquire the state lock themselves, so the sync runs
/// after the lock that produced it has been released.
fn sync_recycler(handle: u64, host: u64, id: NodeId) {
    guard("recycler", || {
        let Some((collection, data_source)) =
            with_state(handle, host, |state| state.recycler_for(id)).flatten()
        else {
            return;
        };
        let Some(changes) = data_source.take_changes() else {
            return;
        };

        if changes.batch {
            let paths: Vec<Retained<NSIndexPath>> = if changes.to > changes.from {
                (changes.from..changes.to)
                    .map(|index| NSIndexPath::indexPathForItem_inSection(index as isize, 0))
                    .collect()
            } else {
                (changes.to..changes.from)
                    .map(|index| NSIndexPath::indexPathForItem_inSection(index as isize, 0))
                    .collect()
            };
            let array = NSArray::from_retained_slice(&paths);
            let insert = changes.to > changes.from;
            let updates_collection = collection.clone();
            let updates = RcBlock::new(move || {
                if insert {
                    updates_collection.insertItemsAtIndexPaths(&array);
                } else {
                    updates_collection.deleteItemsAtIndexPaths(&array);
                }
            });
            let batch_collection = collection.clone();
            let batch = RcBlock::new(move || {
                batch_collection.performBatchUpdates_completion(Some(&*updates), None);
            });
            // SAFETY: the sync block runs on the main queue.
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            UIView::performWithoutAnimation(&batch, mtm);
        }

        if changes.layout {
            collection.collectionViewLayout().invalidateLayout();
        }
        if changes.refresh {
            with_state(handle, host, |state| state.rebind_visible_rows(id));
        }
    });
}

fn focus_row_async(handle: u64, host: u64, field: NodeId) {
    guard("focus", || {
        with_state(handle, host, |state| {
            let Some(focus) = state.runtime.row_focus() else {
                return;
            };
            if focus.field != field {
                return;
            }
            if state.focus_field(field) {
                state
                    .runtime
                    .set_row_focus(focus.list, focus.index, field, true);
            }
        });
    });
}

fn with_state<T>(handle: u64, host: u64, body: impl FnOnce(&mut IosState) -> T) -> Option<T> {
    let mut states = states().lock().expect("ios states");
    states
        .get_mut(&handle)
        .and_then(|session| session.hosts.get_mut(&host))
        .map(body)
}

fn with_session<T>(handle: u64, body: impl FnOnce(&mut IosSession) -> T) -> Option<T> {
    let mut states = states().lock().expect("ios states");
    states.get_mut(&handle).map(body)
}

/// Runs an event handler, reporting an Objective-C exception instead of
/// letting it unwind through the C ABI boundary and abort the process.
fn guard(label: &str, body: impl FnOnce()) {
    let result = objc2::exception::catch(std::panic::AssertUnwindSafe(body));
    if let Err(exception) = result {
        let message = match exception {
            Some(exception) => format!("{exception:?}"),
            None => "nil exception".to_string(),
        };
        eprintln!("krabik ios: {label} handler raised an exception: {message}");
    }
}

fn dispatch_click(handle: u64, host: u64, id: NodeId) {
    guard("click", || {
        with_session(handle, |session| {
            let action = session.hosts.get(&host).and_then(|state| {
                state
                    .runtime
                    .click(id)
                    .or_else(|| state.row_actions.get(&id).and_then(|row| row.click.clone()))
            });
            if let Some(action) = action
                && session.app.handle(Event::Click { action: &action })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_text(handle: u64, host: u64, id: NodeId, value: &str) {
    guard("text", || {
        with_session(handle, |session| {
            let action = session.hosts.get(&host).and_then(|state| {
                state
                    .runtime
                    .text_field_action(id)
                    .or_else(|| state.row_actions.get(&id).and_then(|row| row.text.clone()))
            });
            if let Some(action) = action
                && session.app.handle(Event::TextChanged {
                    action: &action,
                    value,
                })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_submitted(handle: u64, host: u64, id: NodeId) {
    guard("submitted", || {
        with_session(handle, |session| {
            let action = session.hosts.get(&host).and_then(|state| {
                state.runtime.text_field_submit_action(id).or_else(|| {
                    state
                        .row_actions
                        .get(&id)
                        .and_then(|row| row.submit.clone())
                })
            });
            if let Some(action) = action
                && session.app.handle(Event::Submitted { action: &action })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_focus(handle: u64, host: u64, id: NodeId, focused: bool) {
    guard("focus", || {
        with_session(handle, |session| {
            let Some(state) = session.hosts.get_mut(&host) else {
                return;
            };
            let row = state.row_nodes.get(&id).copied();
            let focus_changed = if focused {
                match row {
                    Some((list, index)) => state.runtime.set_row_focus(list, index, id, true),
                    None => state.runtime.set_focus(id, true),
                }
            } else if let Some((list, index)) = row {
                if state.field_attached(id) {
                    state.runtime.set_row_focus(list, index, id, false)
                } else {
                    // The cell left the hierarchy; keep the logical focus so
                    // it can be restored when the row comes back.
                    state.runtime.park_row_focus(id);
                    false
                }
            } else if state
                .runtime
                .row_focus()
                .is_some_and(|focus| focus.field == id)
            {
                state.runtime.park_row_focus(id);
                false
            } else {
                state.runtime.set_focus(id, false)
            };
            let action = state
                .runtime
                .text_field_focus_action(id)
                .or_else(|| state.row_actions.get(&id).and_then(|row| row.focus.clone()));
            let handled = action.is_some_and(|action| {
                session.app.handle(Event::FocusChanged {
                    action: &action,
                    focused,
                })
            });
            if focus_changed || handled {
                session.render_all();
            }
        });
    });
}

fn dispatch_checked(handle: u64, host: u64, id: NodeId, checked: bool) {
    guard("checked", || {
        with_session(handle, |session| {
            let action = session.hosts.get(&host).and_then(|state| {
                state.runtime.toggle_action(id).or_else(|| {
                    state
                        .row_actions
                        .get(&id)
                        .and_then(|row| row.toggle.clone())
                })
            });
            if let Some(action) = action
                && session.app.handle(Event::Toggled {
                    action: &action,
                    checked,
                })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_value(handle: u64, host: u64, id: NodeId, value: f64) {
    guard("slider", || {
        with_session(handle, |session| {
            let (action, config) = session
                .hosts
                .get(&host)
                .map(|state| {
                    let action = state
                        .runtime
                        .slider_action(id)
                        .or_else(|| state.row_actions.get(&id).and_then(|row| row.value.clone()));
                    let config = state.runtime.slider_config(id).or_else(|| {
                        state
                            .row_actions
                            .get(&id)
                            .map(|row| (row.value_min, row.value_max, row.value_step))
                    });
                    (action, config)
                })
                .unwrap_or((None, None));
            let value = match config {
                Some((min, max, step)) => snap_slider_value(value as f32, min, max, step),
                None => value as f32,
            };
            if let Some(action) = action
                && session.app.handle(Event::ValueChanged {
                    action: &action,
                    value,
                })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_picker(handle: u64, host: u64, id: NodeId, value: DatePickerValue) {
    guard("picker", || {
        with_session(handle, |session| {
            let config = session.hosts.get(&host).and_then(|state| {
                state.runtime.date_picker_config(id).or_else(|| {
                    state.row_actions.get(&id).and_then(|row| {
                        row.date
                            .clone()
                            .map(|action| (action, row.date_range, row.time_range))
                    })
                })
            });
            let Some((action, date_range, time_range)) = config else {
                return;
            };
            let value = value.clamped(date_range, time_range);
            let handled = match value {
                DatePickerValue::Date(date) => session.app.handle(Event::DateChanged {
                    action: &action,
                    date,
                }),
                DatePickerValue::Time(time) => session.app.handle(Event::TimeChanged {
                    action: &action,
                    time,
                }),
                DatePickerValue::DateTime { date, time } => {
                    session.app.handle(Event::DateTimeChanged {
                        action: &action,
                        date,
                        time,
                    })
                }
            };
            if handled {
                session.render_all();
            }
        });
    });
}

fn dispatch_selected(handle: u64, host: u64, id: NodeId, index: usize) {
    guard("selection", || {
        with_session(handle, |session| {
            let action = session.hosts.get(&host).and_then(|state| {
                state
                    .runtime
                    .tab_bar_action(id)
                    .or_else(|| state.runtime.pager_action(id))
                    .or_else(|| state.runtime.radio_group_action(id))
                    .or_else(|| {
                        state
                            .row_actions
                            .get(&id)
                            .and_then(|row| row.selected.clone())
                    })
            });
            if let Some(action) = action
                && session.app.handle(Event::Selected {
                    action: &action,
                    index,
                })
            {
                session.render_all();
            }
        });
    });
}

fn dispatch_scroll(handle: u64, host: u64, id: NodeId, offset: f64) {
    guard("scroll", || {
        with_session(handle, |session| {
            let changed = session
                .hosts
                .get_mut(&host)
                .is_some_and(|state| state.runtime.scroll(id, offset as f32));
            if changed {
                session.render_all();
            }
        });
    });
}

/// Starts an `NSURLSession` download for a remote image.
///
/// The completion handler decodes the image off the main thread and hands it
/// to the main queue, where [`finish_download`] applies it.
fn start_download(handle: u64, host: u64, id: NodeId, url: &str) {
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
fn finish_download(
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

/// One application instance shared by every native host in a scene.
struct IosSession {
    handle: u64,
    app: Box<dyn App>,
    hosts: HashMap<u64, IosState>,
    pending: HashMap<u64, IosHostKind>,
    next_host: u64,
    controller: Option<Retained<NSObject>>,
    /// The navigation stack last mirrored to the native controller.
    navigation: Vec<crate::navigation::Screen>,
    /// The alert the native controller is showing.
    dialog: Option<crate::presentation::Dialog>,
}

// SAFETY: every host and the controller are only touched on the main thread,
// serialized by the global session mutex.
unsafe impl Send for IosSession {}

impl IosSession {
    fn new(handle: u64, root: &Retained<UIView>, mtm: MainThreadMarker) -> Self {
        let main = IosState::new(handle, 0, IosHostKind::Main, root, mtm);
        let mut hosts = HashMap::new();
        hosts.insert(0, main);
        Self {
            handle,
            app: crate::app::create_app(),
            hosts,
            pending: HashMap::new(),
            next_host: 1,
            controller: None,
            navigation: Vec::new(),
            dialog: None,
        }
    }

    fn render_all(&mut self) {
        let has_navigation = self.app.navigation().is_some();
        let mut elements = Vec::new();
        for (id, host) in &self.hosts {
            let element = match &host.kind {
                IosHostKind::Main if has_navigation => None,
                IosHostKind::Main => Some(self.app.view()),
                IosHostKind::Screen(key) => self.app.screen(key),
                IosHostKind::Modal => self.app.sheet().map(|sheet| sheet.into_content()),
            };
            if let Some(element) = element {
                elements.push((*id, element));
            }
        }
        for (id, element) in elements {
            if let Some(host) = self.hosts.get_mut(&id) {
                host.render(&element);
            }
        }
        self.sync();
    }

    /// Reconciles the Rust state with the native iOS controller.
    fn sync(&mut self) {
        if self.controller.is_none() {
            return;
        }
        let navigation = self
            .app
            .navigation()
            .map(|nav| nav.screens)
            .unwrap_or_default();
        if navigation != self.navigation {
            self.sync_navigation(&navigation);
            self.sync_chrome(navigation.last());
            self.navigation = navigation;
        }
        let dialog = self.app.dialog();
        if dialog != self.dialog {
            self.sync_dialog(dialog.as_ref());
            self.dialog = dialog;
        }
        let sheet = self.app.sheet().is_some();
        if sheet != self.has_sheet() {
            self.sync_sheet(sheet);
        }
    }

    /// Mirrors the top screen's toolbar and bottom navigation.
    ///
    /// Sent on every navigation change so the native chrome never keeps a
    /// previous screen's configuration.
    fn sync_chrome(&mut self, screen: Option<&crate::navigation::Screen>) {
        let Some(controller) = self.controller.as_ref() else {
            return;
        };
        let toolbar = screen.and_then(|screen| screen.toolbar.as_ref());
        let title = screen.map_or("", |screen| screen.title.as_str());
        let subtitle = toolbar.and_then(|t| t.subtitle.as_deref()).unwrap_or("");
        let back_title = toolbar
            .and_then(|t| t.back_title.as_deref())
            .unwrap_or("Back");
        // A screen shows the navigation bar unless it explicitly hides it.
        let hidden = screen.is_none() || toolbar.is_some_and(|toolbar| toolbar.hidden);
        let actions = toolbar.map_or(&[][..], |toolbar| toolbar.actions.as_slice());
        controller_set_toolbar(controller, title, subtitle, back_title, hidden, actions);
        controller_set_bottom_navigation(
            controller,
            screen.and_then(|screen| screen.bottom_navigation.as_ref()),
        );
    }

    fn sync_navigation(&mut self, screens: &[crate::navigation::Screen]) {
        let Some(controller) = self.controller.as_ref() else {
            return;
        };
        let common = self
            .navigation
            .iter()
            .zip(screens.iter())
            .take_while(|(old, new)| old.key == new.key)
            .count();

        // Leaving several screens at once is one native pop, animated with
        // the transition of the screen the user is looking at.
        let popped = &self.navigation[common..];
        if let Some(top) = popped.last() {
            controller_pop(controller, popped.len(), top.transition.code());
            let hosts: Vec<u64> = popped
                .iter()
                .filter_map(|screen| self.screen_host(&screen.key))
                .collect();
            for host in hosts {
                self.hosts.remove(&host);
                self.pending.remove(&host);
            }
        }
        for screen in screens.iter().skip(common) {
            let host = self.next_host;
            self.next_host += 1;
            self.pending
                .insert(host, IosHostKind::Screen(screen.key.clone()));
            controller_push(controller, host, &screen.title, screen.transition.code());
        }

        controller_set_root_visible(controller, screens.is_empty());
    }

    /// Shows, replaces or closes the system alert.
    fn sync_dialog(&mut self, dialog: Option<&crate::presentation::Dialog>) {
        let Some(controller) = self.controller.as_ref() else {
            return;
        };
        if self.dialog.is_some() {
            controller_dismiss_dialog(controller);
        }
        if let Some(dialog) = dialog {
            controller_show_dialog(controller, dialog);
        }
    }

    /// Presents or dismisses the native bottom sheet.
    fn sync_sheet(&mut self, show: bool) {
        let Some(controller) = self.controller.as_ref() else {
            return;
        };
        if show {
            let host = self.next_host;
            self.next_host += 1;
            self.pending.insert(host, IosHostKind::Modal);
            controller_present_sheet(controller, host);
        } else {
            controller_dismiss_sheet(controller);
            self.forget_sheet();
        }
    }

    /// Whether a sheet is presented or about to be.
    fn has_sheet(&self) -> bool {
        self.hosts
            .values()
            .any(|host| matches!(host.kind, IosHostKind::Modal))
            || self
                .pending
                .values()
                .any(|kind| matches!(kind, IosHostKind::Modal))
    }

    /// Drops the sheet's host after the sheet closed.
    fn forget_sheet(&mut self) {
        self.hosts
            .retain(|_, host| !matches!(host.kind, IosHostKind::Modal));
        self.pending
            .retain(|_, kind| !matches!(kind, IosHostKind::Modal));
    }

    fn attach_host(&mut self, host: u64, root: &Retained<UIView>, mtm: MainThreadMarker) {
        let kind = if let Some(existing) = self.hosts.get(&host) {
            existing.kind.clone()
        } else if let Some(kind) = self.pending.remove(&host) {
            kind
        } else {
            return;
        };
        let state = IosState::new(self.handle, host, kind, root, mtm);
        self.hosts.insert(host, state);
    }

    fn screen_host(&self, key: &str) -> Option<u64> {
        self.hosts.iter().find_map(|(id, host)| match &host.kind {
            IosHostKind::Screen(screen) if screen == key => Some(*id),
            _ => None,
        })
    }
}

fn controller_push(controller: &NSObject, host: u64, title: &str, transition: i32) {
    let title = NSString::from_str(title);
    // SAFETY: the controller is a live Swift object with this @objc method.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikPush: host as i64,
            title: &*title,
            transition: transition as i64
        ]
    };
}

fn controller_set_toolbar(
    controller: &NSObject,
    title: &str,
    subtitle: &str,
    back_title: &str,
    hidden: bool,
    actions: &[crate::navigation::ToolbarAction],
) {
    let title = NSString::from_str(title);
    let subtitle = NSString::from_str(subtitle);
    let back_title = NSString::from_str(back_title);
    let ids: Vec<Retained<NSString>> = actions
        .iter()
        .map(|action| NSString::from_str(&action.action))
        .collect();
    let labels: Vec<Retained<NSString>> = actions
        .iter()
        .map(|action| NSString::from_str(&action.label))
        .collect();
    let icons: Vec<Retained<NSString>> = actions
        .iter()
        .map(|action| NSString::from_str(action.icon.ios_name().unwrap_or("")))
        .collect();
    let ids = NSArray::from_retained_slice(&ids);
    let labels = NSArray::from_retained_slice(&labels);
    let icons = NSArray::from_retained_slice(&icons);
    // SAFETY: the controller is a live Swift object with this @objc method.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikSetToolbar: &*title,
            subtitle: &*subtitle,
            backTitle: &*back_title,
            hidden: hidden,
            actionIds: &*ids,
            actionLabels: &*labels,
            actionIcons: &*icons
        ]
    };
}

fn controller_pop(controller: &NSObject, count: usize, transition: i32) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![controller, krabikPop: count as i64, transition: transition as i64]
    };
}

fn controller_set_root_visible(controller: &NSObject, visible: bool) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikSetRootVisible: visible] };
}

fn string_array<'a>(values: impl Iterator<Item = &'a str>) -> Retained<NSArray<NSString>> {
    let values: Vec<Retained<NSString>> = values.map(NSString::from_str).collect();
    NSArray::from_retained_slice(&values)
}

/// Applies the bottom navigation bar; `None` hides it.
fn controller_set_bottom_navigation(
    controller: &NSObject,
    bar: Option<&crate::navigation::BottomNavigation>,
) {
    let items = bar.map_or(&[][..], |bar| bar.items.as_slice());
    let labels = string_array(items.iter().map(|item| item.label.as_str()));
    let icons = string_array(items.iter().map(|item| item.icon.ios_name().unwrap_or("")));
    let selected = bar.map_or(0, |bar| bar.selected) as i64;
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikSetBottomNavigation: &*labels,
            icons: &*icons,
            selected: selected
        ]
    };
}

fn controller_show_dialog(controller: &NSObject, dialog: &crate::presentation::Dialog) {
    let title = NSString::from_str(&dialog.title);
    let message = NSString::from_str(dialog.message.as_deref().unwrap_or(""));
    let labels = string_array(dialog.buttons.iter().map(|b| b.label.as_str()));
    let actions = string_array(dialog.buttons.iter().map(|b| b.action.as_str()));
    let roles: Vec<Retained<NSNumber>> = dialog
        .buttons
        .iter()
        .map(|b| NSNumber::new_i32(b.role.code()))
        .collect();
    let roles = NSArray::from_retained_slice(&roles);
    // SAFETY: see `controller_push`.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikShowDialog: &*title,
            message: &*message,
            labels: &*labels,
            roles: &*roles,
            actions: &*actions
        ]
    };
}

fn controller_dismiss_dialog(controller: &NSObject) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikDismissDialog] };
}

fn controller_present_sheet(controller: &NSObject, host: u64) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikPresentSheet: host as i64] };
}

fn controller_dismiss_sheet(controller: &NSObject) {
    // SAFETY: see `controller_push`.
    let _: () = unsafe { objc2::msg_send![controller, krabikDismissSheet] };
}

static STATES: OnceLock<Mutex<HashMap<u64, IosSession>>> = OnceLock::new();
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn states() -> &'static Mutex<HashMap<u64, IosSession>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Creates the Rust-side state for a `RustRootView`.
///
/// # Safety
///
/// `root` must be a live `UIView` and the call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_create(root: *mut c_void, width: f64, height: f64) -> u64 {
    let Some(mtm) = MainThreadMarker::new() else {
        return 0;
    };
    // SAFETY: the caller guarantees a live view for the duration of the call.
    let Some(root) = (unsafe { Retained::retain(root.cast::<UIView>()) }) else {
        return 0;
    };

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let mut session = IosSession::new(handle, &root, mtm);
    guard("create", || {
        if let Some(main) = session.hosts.get_mut(&0) {
            main.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
        }
        session.render_all();
    });
    states().lock().expect("ios states").insert(handle, session);
    handle
}

/// Rebuilds the tree for the new viewport and applies the resulting operations.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_render(handle: u64, host: u64, width: f64, height: f64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("render", || {
        with_session(handle, |session| {
            if let Some(state) = session.hosts.get_mut(&host) {
                state.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
            }
            session.render_all();
        });
    });
}

/// Releases every widget and the Rust state of a scene.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_destroy(handle: u64) {
    let Some(session) = states().lock().expect("ios states").remove(&handle) else {
        return;
    };
    for host in session.hosts.values() {
        for view in host.views.values() {
            view.view.removeFromSuperview();
        }
    }
}

/// Attaches a new native host view to a session.
///
/// # Safety
///
/// `root` must be a live `UIView` and the call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_attach_host(handle: u64, host: u64, root: *mut c_void) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // SAFETY: the caller guarantees a live view for the duration of the call.
    let Some(root) = (unsafe { Retained::retain(root.cast::<UIView>()) }) else {
        return;
    };
    guard("attach host", || {
        with_session(handle, |session| {
            session.attach_host(host, &root, mtm);
            session.render_all();
        });
    });
}

/// Drops one host after its native view is destroyed.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_destroy_host(handle: u64, host: u64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("destroy host", || {
        with_session(handle, |session| {
            session.hosts.remove(&host);
            session.pending.remove(&host);
        });
    });
}

/// Registers the native navigation/modal controller.
///
/// # Safety
///
/// `controller` must be a live Objective-C object and the call must happen on
/// the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_controller(handle: u64, controller: *mut c_void) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let _ = mtm;
    // SAFETY: the caller guarantees a live Objective-C object.
    let Some(controller) = (unsafe { Retained::retain(controller.cast::<NSObject>()) }) else {
        return;
    };
    guard("controller", || {
        with_session(handle, |session| {
            session.controller = Some(controller);
            session.render_all();
        });
    });
}

/// Dispatches the native dismissal of the bottom sheet (swipe or scrim tap).
///
/// The platform has already closed the sheet, so the session forgets it
/// first: if the application still returns a sheet, it is presented again.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_modal_dismiss(handle: u64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("sheet dismiss", || {
        with_session(handle, |session| {
            session.forget_sheet();
            if let Some(sheet) = session.app.sheet() {
                session.app.handle(Event::Click {
                    action: &sheet.dismiss_action,
                });
            }
            session.render_all();
        });
    });
}

/// Dispatches a button press of the system alert.
///
/// The platform has already closed the alert, so the session forgets it
/// first: if the application still returns a dialog, it is shown again.
///
/// # Safety
///
/// `action` must point to a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_dialog_action(handle: u64, action: *const c_char) {
    if action.is_null() || MainThreadMarker::new().is_none() {
        return;
    }
    // SAFETY: the caller guarantees a valid NUL-terminated string.
    let action = unsafe { CStr::from_ptr(action) }
        .to_string_lossy()
        .into_owned();
    guard("dialog action", || {
        with_session(handle, |session| {
            session.dialog = None;
            if !action.is_empty() {
                session.app.handle(Event::Click { action: &action });
            }
            session.render_all();
        });
    });
}

/// Dispatches a selection in the host-level bottom navigation bar.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_bottom_navigation_selected(handle: u64, index: u64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("bottom navigation", || {
        with_session(handle, |session| {
            let action = session
                .navigation
                .last()
                .and_then(|screen| screen.bottom_navigation.as_ref())
                .map(|bar| bar.action.clone())
                .filter(|action| !action.is_empty());
            if let Some(action) = action
                && session.app.handle(Event::Selected {
                    action: &action,
                    index: index as usize,
                })
            {
                session.render_all();
            }
        });
    });
}

/// The height, in points, of the content a host rendered last.
///
/// Lets a content-sized native container, such as a sheet, wrap it.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_content_height(handle: u64, host: u64) -> f64 {
    if MainThreadMarker::new().is_none() {
        return 0.0;
    }
    with_session(handle, |session| {
        session
            .hosts
            .get(&host)
            .map_or(0.0, |state| f64::from(state.runtime.content_height()))
    })
    .unwrap_or(0.0)
}

/// Dispatches a click on a native widget back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_click(handle: u64, host: u64, node_id: u64) {
    dispatch_click(handle, host, NodeId(node_id));
}

/// Dispatches a text edit on a native input back into the Rust tree.
///
/// # Safety
///
/// `text` must be a valid, NUL-terminated UTF-8 string, and the call must
/// happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_text_changed(
    handle: u64,
    host: u64,
    node_id: u64,
    text: *const c_char,
) {
    if text.is_null() {
        return;
    }
    // SAFETY: the caller guarantees a valid NUL-terminated string.
    let value = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    dispatch_text(handle, host, NodeId(node_id), &value);
}

/// Dispatches a submit on a native input back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_submitted(handle: u64, host: u64, node_id: u64) {
    dispatch_submitted(handle, host, NodeId(node_id));
}

/// Dispatches a focus change on a native input back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_focus_changed(
    handle: u64,
    host: u64,
    node_id: u64,
    focused: bool,
) {
    dispatch_focus(handle, host, NodeId(node_id), focused);
}

/// Dispatches a switch, checkbox or radio toggle back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_checked_changed(
    handle: u64,
    host: u64,
    node_id: u64,
    checked: bool,
) {
    dispatch_checked(handle, host, NodeId(node_id), checked);
}

/// Dispatches a slider value back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_slider_changed(
    handle: u64,
    host: u64,
    node_id: u64,
    value: f64,
) {
    dispatch_value(handle, host, NodeId(node_id), value);
}

/// Dispatches a native toolbar action into the application.
///
/// # Safety
///
/// `action` must point to a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_toolbar_action(handle: u64, action: *const c_char) {
    if action.is_null() || MainThreadMarker::new().is_none() {
        return;
    }
    // SAFETY: the caller guarantees a valid NUL-terminated string.
    let action = unsafe { CStr::from_ptr(action) }
        .to_string_lossy()
        .into_owned();
    guard("toolbar action", || {
        with_session(handle, |session| {
            if session.app.handle(Event::Click { action: &action }) {
                session.render_all();
            }
        });
    });
}

/// Dispatches the native back action to the application.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_back(handle: u64) -> bool {
    if MainThreadMarker::new().is_none() {
        return false;
    }
    with_session(handle, |session| {
        let handled = session.app.handle(Event::Back);
        if handled {
            session.render_all();
        }
        handled
    })
    .unwrap_or(false)
}
