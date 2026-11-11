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
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGColor;
use objc2_foundation::{
    NSArray, NSBundle, NSData, NSError, NSIndexPath, NSString, NSURL, NSURLResponse, NSURLSession,
};
use objc2_quartz_core::{CACornerMask, CAGradientLayer};
use objc2_ui_kit::{
    NSIndexPathUIKitAdditions, NSLineBreakMode, NSObjectUIAccessibility, NSTextAlignment, UIAction,
    UIButton, UIButtonType, UICollectionView, UICollectionViewCell, UICollectionViewDataSource,
    UICollectionViewDelegate, UICollectionViewDelegateFlowLayout, UICollectionViewFlowLayout,
    UICollectionViewLayout, UIColor, UIControl, UIControlContentHorizontalAlignment,
    UIControlEvents, UIControlState, UIEdgeInsets, UIFont, UIImage, UIImageView, UIKeyboardType,
    UILabel, UIProgressView, UIReturnKeyType, UIScrollView, UIScrollViewDelegate, UISlider,
    UISwitch, UITextBorderStyle, UITextField, UITextInputTraits, UIView, UIViewContentMode,
};

use crate::app::{App, Event};
use crate::geometry::{Constraints, Rect, Size};
use crate::layout::LayoutResult;
use crate::measure::Measure;
use crate::render::{self, Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{self, NodeId, NodeKind, RecyclerProps, UiNode};
use crate::ui::{
    Axis, ButtonProps, CheckProps, EdgeInsets, Element, FontWeight, GradientDirection, ImageProps,
    ImageScale, ImageSource, ImeAction, KeyboardType, ProgressProps, SliderProps, TextAlign,
    TextFieldProps, TextProps,
};

/// Measures text with an off-screen `UILabel`.
///
/// Results are cached because Taffy asks for the same intrinsic sizes several
/// times per layout pass. UIKit objects are only touched from the main thread,
/// which is also where the runtime renders.
struct IosMeasure {
    label: Retained<UILabel>,
    cache: Mutex<HashMap<MeasureKey, Size>>,
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
            let id = ivars.id;
            schedule(move || {
                shared.scheduled.store(false, Ordering::Release);
                let offset = f64::from_bits(shared.latest.load(Ordering::Acquire));
                dispatch_scroll(handle, id, offset);
            });
        }
    }
);

impl ScrollDelegate {
    fn new(handle: u64, id: NodeId, horizontal: bool, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ScrollDelegateIvars {
            handle,
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
    id: NodeId,
    count: AtomicU64,
    /// Fixed row height in points encoded as `f64` bits; `0` means automatic.
    item_height: AtomicU64,
    revision: AtomicU64,
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
            let _ = with_state(ivars.handle, |state| {
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
            let _ = with_state(self.ivars().handle, |state| state.unbind_row(key));
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
            let height = with_state(ivars.handle, |state| {
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
        id: NodeId,
        props: &RecyclerProps,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RecyclerIvars {
            handle,
            id,
            count: AtomicU64::new(props.item_count as u64),
            item_height: AtomicU64::new(props.item_height.map_or(0.0, f64::from).to_bits()),
            revision: AtomicU64::new(props.revision),
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }

    /// Applies new list properties, asking UIKit to rebind when anything moved.
    fn update(&self, props: &RecyclerProps) {
        let count = props.item_count as u64;
        let height = props.item_height.map_or(0.0, f64::from).to_bits();
        let ivars = self.ivars();
        let count_changed = ivars.count.swap(count, Ordering::Relaxed) != count;
        let height_changed = ivars.item_height.swap(height, Ordering::Relaxed) != height;
        let revision_changed =
            ivars.revision.swap(props.revision, Ordering::Relaxed) != props.revision;
        if count_changed || height_changed {
            // The layout needs new sizes and cells; reload after the lock that
            // produced the update has been released.
            let handle = ivars.handle;
            let id = ivars.id;
            schedule(move || reload_recycler(handle, id));
        } else if revision_changed {
            // Only the content changed: diff the visible cells in place so
            // their widgets (and any focus) survive the update.
            let handle = ivars.handle;
            let id = ivars.id;
            schedule(move || refresh_recycler(handle, id));
        }
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
    Slider(Retained<UISlider>),
    Progress(Retained<UIProgressView>),
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
    fn inset_view(&self) -> Option<&UIView> {
        match self {
            Self::Label(view) => Some(view),
            Self::Field(view) => Some(view),
            _ => None,
        }
    }

    fn children_host(&self, fallback: &Retained<UIView>) -> Retained<UIView> {
        match self {
            Self::Host(host) => host.clone(),
            Self::Scroll { host, .. } => host.clone(),
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
        }
    }

    /// The view that hosts the node's native children.
    fn children_host(&self) -> Retained<UIView> {
        self.content.children_host(&self.view)
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
    value: Option<Cow<'static, str>>,
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

/// The state of one `RustRootView`.
struct IosState {
    handle: u64,
    runtime: Runtime,
    app: Box<dyn App>,
    /// A weak reference so the Rust state does not keep the Swift root view
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
    viewport: Size,
}

// SAFETY: all access is serialized by the global mutex and happens on the main
// thread. `Retained<UIView>` is not `Send` on its own, which is what this
// assertion documents.
unsafe impl Send for IosState {}

impl IosState {
    fn new(handle: u64, root: &Retained<UIView>, mtm: MainThreadMarker) -> Self {
        Self {
            handle,
            runtime: Runtime::new(Box::new(IosMeasure::new(mtm))),
            app: crate::app::create_app(),
            root: Weak::from_retained(root),
            views: HashMap::new(),
            images: ImageLoader::default(),
            row_actions: HashMap::new(),
            row_action_ids: HashMap::new(),
            row_states: HashMap::new(),
            row_nodes: HashMap::new(),
            viewport: Size::ZERO,
        }
    }

    fn render(&mut self) {
        let element = self.app.view();
        let update = self.runtime.update(&element, self.viewport);
        let Some(root) = self.root.load() else {
            return;
        };

        // The main views are taken out while the operations run so a recycled
        // row can apply its own operations without borrowing the state twice.
        let mut views = std::mem::take(&mut self.views);
        for op in update.ops {
            self.apply_to(op, &mut views, &root);
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
    ) {
        // SAFETY: the runtime renders on the main thread, and every entry point
        // checks that before locking the state.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        match op {
            Op::Create(node) => {
                let handle = self.create_handle(&node, mtm);
                let host = parent_host_in(views, root_host, node.parent);
                host.addSubview(&handle.view);
                views.insert(node.id, handle);
            }
            Op::Update(node) => {
                if let Some(handle) = views.get_mut(&node.id) {
                    update_content(&handle.content, &node);
                    apply_style(&handle.view, &node, &mut handle.gradient);
                    apply_accessibility(&handle.view, &node, mtm);
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
            Op::Remove { id } => {
                self.images.expected.remove(&id);
                if let Some(handle) = views.remove(&id) {
                    handle.view.removeFromSuperview();
                }
            }
            Op::Layout {
                id,
                frame,
                content_size,
            } => IosState::apply_layout(views, id, frame, content_size),
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
                wire_field(&field, self.handle, node.id, mtm);
                wrapper.addSubview(&field);
                ViewHandle::new(wrapper, Content::Field(field), node)
            }
            NodeKind::Button(props) => {
                let button = UIButton::buttonWithType(UIButtonType::System, mtm);
                update_button(&button, props);
                wire_click(&button, self.handle, node.id, mtm);
                let view = cast_view(button.clone());
                ViewHandle::new(view, Content::Button(button), node)
            }
            NodeKind::Switch(props) => {
                let switch = UISwitch::new(mtm);
                switch.setOn(props.checked);
                wire_switch(&switch, self.handle, node.id, mtm);
                let view = cast_view(switch.clone());
                ViewHandle::new(view, Content::Switch(switch), node)
            }
            NodeKind::Checkbox(props) => {
                let button = UIButton::buttonWithType(UIButtonType::Custom, mtm);
                update_check(&button, props, false);
                wire_check(&button, self.handle, node.id, mtm);
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
                wire_check(&button, self.handle, node.id, mtm);
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
            NodeKind::Slider(props) => {
                let slider = UISlider::new(mtm);
                update_slider(&slider, props);
                wire_slider(&slider, self.handle, node.id, mtm);
                let view = cast_view(slider.clone());
                ViewHandle::new(view, Content::Slider(slider), node)
            }
            NodeKind::Progress(props) => {
                let progress = UIProgressView::new(mtm);
                update_progress(&progress, props);
                let view = cast_view(progress.clone());
                ViewHandle::new(view, Content::Progress(progress), node)
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
                let delegate =
                    ScrollDelegate::new(self.handle, node.id, props.axis == Axis::Horizontal, mtm);
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
                let data_source = RecyclerDataSource::new(self.handle, node.id, props, mtm);
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
        handle.view.setFrame(rect);
        handle.frame = Some(frame);

        if let Some(content) = handle.content.inset_view() {
            content.setFrame(inset(rect.size, handle.padding));
        }

        match &handle.content {
            Content::Host(host) => host.setFrame(inset(rect.size, handle.padding)),
            Content::Scroll { scroll, host, .. } => {
                let size = content_size.unwrap_or(Size::ZERO);
                host.setFrame(CGRect::new(
                    CGPoint::new(
                        f64::from(handle.padding.left),
                        f64::from(handle.padding.top),
                    ),
                    CGSize::new(
                        f64::from(size.width.max(0.0)),
                        f64::from(size.height.max(0.0)),
                    ),
                ));
                scroll.setContentSize(CGSize::new(
                    f64::from(handle.padding.left + size.width + handle.padding.right),
                    f64::from(handle.padding.top + size.height + handle.padding.bottom),
                ));
            }
            _ => {}
        }

        if let Some(gradient) = &handle.gradient {
            gradient.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), rect.size));
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
            start_download(self.handle, id, url);
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

    /// The collection view of a recycled list, for reloads outside the lock.
    fn collection_for(&self, id: NodeId) -> Option<Retained<UICollectionView>> {
        match &self.views.get(&id)?.content {
            Content::Recycler { collection, .. } => Some(collection.clone()),
            _ => None,
        }
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
        let content = cell.contentView();
        let key = cell as *const UICollectionViewCell as usize;

        let previous = self.row_states.remove(&key);
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
        for op in ops {
            self.apply_to(op, &mut views, &content);
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
            let field = focus.field;
            schedule(move || focus_row_async(handle, field));
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
                NodeKind::Slider(props) if !props.action.is_empty() => {
                    action.value = Some(props.action.clone());
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

fn update_content(content: &Content, node: &RenderNode) {
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
        (Content::Slider(slider), NodeKind::Slider(props)) => update_slider(slider, props),
        (Content::Progress(progress), NodeKind::Progress(props)) => {
            update_progress(progress, props);
        }
        // Images are updated by `IosState::load_image`, which starts async
        // downloads and therefore needs the state itself.
        _ => {}
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

fn update_slider(slider: &UISlider, props: &SliderProps) {
    let max = props.max.max(props.min + 0.001);
    slider.setMinimumValue(props.min);
    slider.setMaximumValue(max);
    let value = props.value.clamp(props.min, max);
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
        || (radius > 0.0 && style.elevation <= 0.0);
    view.setClipsToBounds(clips);

    if style.gradient.is_some() {
        view.setBackgroundColor(None);
    } else {
        view.setBackgroundColor(style.background.map(color).as_deref());
    }

    if let Some(shape) = style.gradient {
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

    if let Some(border) = style.border {
        let border_color = cg_color(border.color);
        layer.setBorderWidth(f64::from(border.width));
        layer.setBorderColor(Some(&border_color));
    } else {
        layer.setBorderWidth(0.0);
        layer.setBorderColor(None);
    }

    view.setAlpha(f64::from(style.opacity.unwrap_or(1.0)));

    if style.elevation > 0.0 {
        // SAFETY: `CGColor` is retained for as long as the layer keeps it.
        let shadow = unsafe { UIColor::blackColor().CGColor() };
        layer.setShadowColor(Some(&shadow));
        layer.setShadowOpacity(0.18);
        layer.setShadowRadius(f64::from(style.elevation));
        layer.setShadowOffset(CGSize::new(0.0, f64::from(style.elevation / 2.0)));
    } else {
        layer.setShadowOpacity(0.0);
    }
}

/// Applies interaction state and accessibility metadata to a native view.
fn apply_accessibility(view: &UIView, node: &RenderNode, mtm: MainThreadMarker) {
    if let Some(label) = &node.style.accessibility_label {
        let label = NSString::from_str(label);
        view.setIsAccessibilityElement(true, mtm);
        view.setAccessibilityLabel(Some(&label), mtm);
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

fn wire_click(button: &UIButton, handle: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.click",
        mtm,
        move |_| schedule(move || dispatch_click(handle, id)),
    );
}

fn wire_switch(switch: &UISwitch, handle: u64, id: NodeId, mtm: MainThreadMarker) {
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
            schedule(move || dispatch_checked(handle, id, checked));
        },
    );
}

fn wire_check(button: &UIButton, handle: u64, id: NodeId, mtm: MainThreadMarker) {
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
            schedule(move || dispatch_checked(handle, id, checked));
        },
    );
}

fn wire_slider(slider: &UISlider, handle: u64, id: NodeId, mtm: MainThreadMarker) {
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
            schedule(move || dispatch_value(handle, id, f64::from(value)));
        },
    );
}

fn wire_field(field: &UITextField, handle: u64, id: NodeId, mtm: MainThreadMarker) {
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
            schedule(move || dispatch_text(handle, id, &value));
        },
    );
    add_action(
        field,
        UIControlEvents::EditingDidEndOnExit,
        "krabik.submit",
        mtm,
        move |_| schedule(move || dispatch_submitted(handle, id)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidBegin,
        "krabik.focus-begin",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, id, true)),
    );
    add_action(
        field,
        UIControlEvents::EditingDidEnd,
        "krabik.focus-end",
        mtm,
        move |_| schedule(move || dispatch_focus(handle, id, false)),
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

/// Reloads a recycled list without holding the state lock.
///
/// UIKit may ask the data source for cells synchronously during `reloadData`;
/// those callbacks acquire the state lock themselves, so the reload must run
/// after the lock that produced it has been released.
fn focus_row_async(handle: u64, field: NodeId) {
    guard("focus", || {
        with_state(handle, |state| {
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

fn reload_recycler(handle: u64, id: NodeId) {
    guard("reload", || {
        let collection = with_state(handle, |state| state.collection_for(id)).flatten();
        if let Some(collection) = collection {
            collection.reloadData();
        }
    });
}

/// Diffs the visible recycled rows without recreating their cells.
fn refresh_recycler(handle: u64, id: NodeId) {
    guard("refresh", || {
        with_state(handle, |state| state.rebind_visible_rows(id));
    });
}

fn with_state<T>(handle: u64, body: impl FnOnce(&mut IosState) -> T) -> Option<T> {
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

fn dispatch_click(handle: u64, id: NodeId) {
    guard("click", || {
        with_state(handle, |state| {
            let action = state
                .runtime
                .click(id)
                .or_else(|| state.row_actions.get(&id).and_then(|row| row.click.clone()));
            if let Some(action) = action
                && state.app.handle(Event::Click { action: &action })
            {
                state.render();
            }
        });
    });
}

fn dispatch_text(handle: u64, id: NodeId, value: &str) {
    guard("text", || {
        with_state(handle, |state| {
            let action = state
                .runtime
                .text_field_action(id)
                .or_else(|| state.row_actions.get(&id).and_then(|row| row.text.clone()));
            if let Some(action) = action
                && state.app.handle(Event::TextChanged {
                    action: &action,
                    value,
                })
            {
                state.render();
            }
        });
    });
}

fn dispatch_submitted(handle: u64, id: NodeId) {
    guard("submitted", || {
        with_state(handle, |state| {
            let action = state.runtime.text_field_submit_action(id).or_else(|| {
                state
                    .row_actions
                    .get(&id)
                    .and_then(|row| row.submit.clone())
            });
            if let Some(action) = action
                && state.app.handle(Event::Submitted { action: &action })
            {
                state.render();
            }
        });
    });
}

fn dispatch_focus(handle: u64, id: NodeId, focused: bool) {
    guard("focus", || {
        with_state(handle, |state| {
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
            let handled = state
                .runtime
                .text_field_focus_action(id)
                .or_else(|| state.row_actions.get(&id).and_then(|row| row.focus.clone()))
                .is_some_and(|action| {
                    state.app.handle(Event::FocusChanged {
                        action: &action,
                        focused,
                    })
                });
            if focus_changed || handled {
                state.render();
            }
        });
    });
}

fn dispatch_checked(handle: u64, id: NodeId, checked: bool) {
    guard("checked", || {
        with_state(handle, |state| {
            let action = state.runtime.toggle_action(id).or_else(|| {
                state
                    .row_actions
                    .get(&id)
                    .and_then(|row| row.toggle.clone())
            });
            if let Some(action) = action
                && state.app.handle(Event::Toggled {
                    action: &action,
                    checked,
                })
            {
                state.render();
            }
        });
    });
}

fn dispatch_value(handle: u64, id: NodeId, value: f64) {
    guard("slider", || {
        with_state(handle, |state| {
            let action = state
                .runtime
                .slider_action(id)
                .or_else(|| state.row_actions.get(&id).and_then(|row| row.value.clone()));
            if let Some(action) = action
                && state.app.handle(Event::ValueChanged {
                    action: &action,
                    value: value as f32,
                })
            {
                state.render();
            }
        });
    });
}

fn dispatch_scroll(handle: u64, id: NodeId, offset: f64) {
    guard("scroll", || {
        with_state(handle, |state| {
            if state.runtime.scroll(id, offset as f32) {
                state.render();
            }
        });
    });
}

/// Starts an `NSURLSession` download for a remote image.
///
/// The completion handler decodes the image off the main thread and hands it
/// to the main queue, where [`finish_download`] applies it.
fn start_download(handle: u64, id: NodeId, url: &str) {
    let Some(url_object) = NSURL::URLWithString(&NSString::from_str(url)) else {
        eprintln!("krabik ios: invalid image URL: {url}");
        return;
    };
    let url = url.to_string();
    let block = RcBlock::new(
        move |data: *mut NSData, _: *mut NSURLResponse, _: *mut NSError| {
            let image = unsafe { data.as_ref() }.and_then(UIImage::imageWithData);
            let url = url.clone();
            schedule(move || finish_download(handle, id, url, image));
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
fn finish_download(handle: u64, id: NodeId, url: String, image: Option<Retained<UIImage>>) {
    guard("image", || {
        with_state(handle, |state| {
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

static STATES: OnceLock<Mutex<HashMap<u64, IosState>>> = OnceLock::new();
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

fn states() -> &'static Mutex<HashMap<u64, IosState>> {
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
    let mut state = IosState::new(handle, &root, mtm);
    guard("create", || {
        state.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
        state.render();
    });
    states().lock().expect("ios states").insert(handle, state);
    handle
}

/// Rebuilds the tree for the new viewport and applies the resulting operations.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_render(handle: u64, width: f64, height: f64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("render", || {
        with_state(handle, |state| {
            state.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
            state.render();
        });
    });
}

/// Releases the widgets and the Rust state of a `RustRootView`.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_destroy(handle: u64) {
    let Some(state) = states().lock().expect("ios states").remove(&handle) else {
        return;
    };
    for view in state.views.values() {
        view.view.removeFromSuperview();
    }
}

/// Dispatches a click on a native widget back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_click(handle: u64, node_id: u64) {
    dispatch_click(handle, NodeId(node_id));
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
    node_id: u64,
    text: *const c_char,
) {
    if text.is_null() {
        return;
    }
    // SAFETY: the caller guarantees a valid NUL-terminated string.
    let value = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    dispatch_text(handle, NodeId(node_id), &value);
}

/// Dispatches a submit on a native input back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_submitted(handle: u64, node_id: u64) {
    dispatch_submitted(handle, NodeId(node_id));
}

/// Dispatches a focus change on a native input back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_focus_changed(handle: u64, node_id: u64, focused: bool) {
    dispatch_focus(handle, NodeId(node_id), focused);
}

/// Dispatches a switch toggle back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_checked_changed(handle: u64, node_id: u64, checked: bool) {
    dispatch_checked(handle, NodeId(node_id), checked);
}

/// Dispatches a slider value back into the Rust tree.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_slider_changed(handle: u64, node_id: u64, value: f64) {
    dispatch_value(handle, NodeId(node_id), value);
}
