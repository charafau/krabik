//! One native root view: applying engine operations to UIKit views.

use crate::geometry::{Rect, Size};
use crate::host::{Actions, HostKind};
use crate::ios::images::ImageLoader;
use crate::ios::measure::IosMeasure;
use crate::ios::motion::{
    FrameTicker, LayoutFrames, animate_style_change, animation_is_enabled, run_animation,
    style_snapshot,
};
use crate::ios::rows::{IosRowState, ROW_CELL_ID, RecyclerDataSource};
use crate::ios::style::{apply_accessibility, apply_style};
use crate::ios::widgets::button::{update_button, wire_click};
use crate::ios::widgets::check::{update_check, wire_check};
use crate::ios::widgets::date_picker::{update_date_picker, wire_date_picker};
use crate::ios::widgets::pager::PagerDelegate;
use crate::ios::widgets::progress::{update_progress, update_spinner};
use crate::ios::widgets::radio_group::update_radio_group;
use crate::ios::widgets::scroll::ScrollDelegate;
use crate::ios::widgets::slider::{update_slider, wire_slider};
use crate::ios::widgets::switch::wire_switch;
use crate::ios::widgets::tab_bar::{update_tab_bar, wire_tab_bar};
use crate::ios::widgets::text::update_label;
use crate::ios::widgets::text_field::{update_field, wire_field};
use crate::ios::widgets::update_content;
use crate::ios::{cast_view, parent_host_in, schedule, with_session};
use crate::render::{Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{NodeId, NodeKind};
use crate::ui::{Animation, Axis, EdgeInsets};
use crate::widgets::Widget;
use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::ProtocolObject;
use objc2::{ClassType, MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::{CGAffineTransform, CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;
use objc2_quartz_core::{CADisplayLink, CAGradientLayer};
use objc2_ui_kit::{
    UIActivityIndicatorView, UIActivityIndicatorViewStyle, UIButton, UIButtonType,
    UICollectionView, UICollectionViewCell, UICollectionViewFlowLayout, UIDatePicker, UIEdgeInsets,
    UIImageView, UILabel, UILayoutConstraintAxis, UIProgressView, UIScrollView, UISegmentedControl,
    UISlider, UIStackView, UIStackViewAlignment, UIStackViewDistribution, UISwitch,
    UITextBorderStyle, UITextField, UIView, UIViewAnimatingPosition,
};
use std::collections::HashMap;

/// The typed content of a native view.
///
/// Leaves that need internal padding keep a separate content view inside the
/// styled outer view; containers keep a content host that children are added
/// to. Keeping the concrete class here avoids downcasting on every update.
pub(crate) enum Content {
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
    pub(crate) fn inset_handle(&self) -> Option<Retained<UIView>> {
        match self {
            Self::Label(view) => Some(cast_view(view.clone())),
            Self::Field(view) => Some(cast_view(view.clone())),
            _ => None,
        }
    }

    pub(crate) fn children_host(&self, fallback: &Retained<UIView>) -> Retained<UIView> {
        match self {
            Self::Host(host) => host.clone(),
            Self::Scroll { host, .. } => host.clone(),
            Self::Pager { scroll, .. } => cast_view(scroll.clone()),
            _ => fallback.clone(),
        }
    }
}

/// The native resources owned by one renderable node.
pub(crate) struct ViewHandle {
    /// The view that draws the node itself.
    pub(crate) view: Retained<UIView>,
    /// The inner content, when the node has one.
    pub(crate) content: Content,
    /// The gradient layer drawn behind the content, when the node has one.
    pub(crate) gradient: Option<Retained<CAGradientLayer>>,
    /// The nearest renderable ancestor the view lives under.
    pub(crate) parent: Option<NodeId>,
    /// The last relative frame applied to the view.
    pub(crate) frame: Option<Rect>,
    /// The node's padding, used to inset the content.
    pub(crate) padding: EdgeInsets,
    /// The display link driving a frame animation, when one is running.
    pub(crate) frame_link: Option<Retained<CADisplayLink>>,
}

impl ViewHandle {
    pub(crate) fn new(view: Retained<UIView>, content: Content, node: &RenderNode) -> Self {
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
    pub(crate) fn children_host(&self) -> Retained<UIView> {
        self.content.children_host(&self.view)
    }
}

pub(crate) struct IosState {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) kind: HostKind,
    pub(crate) runtime: Runtime,
    /// A weak reference so the Rust state does not keep the Swift host view
    /// alive and prevent its `deinit` from destroying the state.
    pub(crate) root: Weak<UIView>,
    pub(crate) views: HashMap<NodeId, ViewHandle>,
    pub(crate) images: ImageLoader,
    /// Actions of the rows currently bound in recycled lists.
    pub(crate) row_actions: HashMap<NodeId, Actions>,
    /// The node ids a (list, row) binding created, for clearing on rebind.
    pub(crate) row_action_ids: HashMap<(NodeId, usize), Vec<NodeId>>,
    /// The rows currently bound to collection-view cells, by cell identity.
    pub(crate) row_states: HashMap<usize, IosRowState>,
    /// The item each row node belongs to, for routing focus back to its row.
    pub(crate) row_nodes: HashMap<NodeId, (NodeId, usize)>,
    /// Views animating out after their node was removed, by node id.
    pub(crate) pending_exits: HashMap<NodeId, Retained<UIView>>,
    pub(crate) viewport: Size,
}

// SAFETY: all access is serialized by the global mutex and happens on the main
// thread. `Retained<UIView>` is not `Send` on its own, which is what this
// assertion documents.
unsafe impl Send for IosState {}

impl IosState {
    pub(crate) fn new(
        handle: u64,
        host: u64,
        kind: HostKind,
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

    pub(crate) fn render(&mut self, element: &crate::ui::RawElement) {
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
    pub(crate) fn apply_to(
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
                if let NodeKind::Widget(Widget::Image(props)) = &node.kind {
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
                                        if let Some(state) = session.host_mut(host)
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

    pub(crate) fn create_handle(&mut self, node: &RenderNode, mtm: MainThreadMarker) -> ViewHandle {
        let mut handle = match &node.kind {
            NodeKind::Widget(Widget::Text(props)) => {
                let wrapper = UIView::new(mtm);
                let label = UILabel::new(mtm);
                update_label(&label, props);
                wrapper.addSubview(&label);
                ViewHandle::new(wrapper, Content::Label(label), node)
            }
            NodeKind::Widget(Widget::TextField(props)) => {
                let wrapper = UIView::new(mtm);
                let field = UITextField::new(mtm);
                field.setBorderStyle(UITextBorderStyle::None);
                update_field(&field, props);
                wire_field(&field, self.handle, self.host, node.id, mtm);
                wrapper.addSubview(&field);
                ViewHandle::new(wrapper, Content::Field(field), node)
            }
            NodeKind::Widget(Widget::Button(props)) => {
                let button = UIButton::buttonWithType(UIButtonType::System, mtm);
                update_button(&button, props);
                wire_click(&button, self.handle, self.host, node.id, mtm);
                let view = cast_view(button.clone());
                ViewHandle::new(view, Content::Button(button), node)
            }
            NodeKind::Widget(Widget::Switch(props)) => {
                let switch = UISwitch::new(mtm);
                switch.setOn(props.checked);
                wire_switch(&switch, self.handle, self.host, node.id, mtm);
                let view = cast_view(switch.clone());
                ViewHandle::new(view, Content::Switch(switch), node)
            }
            NodeKind::Widget(Widget::Checkbox(props)) => {
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
            NodeKind::Widget(Widget::RadioButton(props)) => {
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
            NodeKind::Widget(Widget::RadioGroup(props)) => {
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
            NodeKind::Widget(Widget::DatePicker(props)) => {
                let picker = UIDatePicker::new(mtm);
                update_date_picker(&picker, props);
                wire_date_picker(&picker, self.handle, self.host, node.id, mtm);
                let view = cast_view(picker.clone());
                ViewHandle::new(view, Content::DatePicker(picker), node)
            }
            NodeKind::Widget(Widget::TabBar(props)) => {
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
            NodeKind::Widget(Widget::Slider(props)) => {
                let slider = UISlider::new(mtm);
                update_slider(&slider, props);
                wire_slider(&slider, self.handle, self.host, node.id, mtm);
                let view = cast_view(slider.clone());
                ViewHandle::new(view, Content::Slider(slider), node)
            }
            NodeKind::Widget(Widget::Progress(props)) => {
                let progress = UIProgressView::new(mtm);
                update_progress(&progress, props);
                let view = cast_view(progress.clone());
                ViewHandle::new(view, Content::Progress(progress), node)
            }
            NodeKind::Widget(Widget::Spinner(props)) => {
                let spinner = UIActivityIndicatorView::initWithActivityIndicatorStyle(
                    UIActivityIndicatorView::alloc(mtm),
                    UIActivityIndicatorViewStyle::Medium,
                );
                update_spinner(&spinner, props);
                let view = cast_view(spinner.clone());
                ViewHandle::new(view, Content::Spinner(spinner), node)
            }
            NodeKind::Widget(Widget::Image(props)) => {
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
            NodeKind::Spacer | NodeKind::Root => {
                let view = UIView::new(mtm);
                ViewHandle::new(view, Content::None, node)
            }
        };

        apply_style(&handle.view, node, &mut handle.gradient);
        apply_accessibility(&handle.view, node, mtm);
        handle
    }

    pub(crate) fn apply_layout(
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
}
