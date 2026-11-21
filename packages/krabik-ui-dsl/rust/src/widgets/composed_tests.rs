//! Tests of the widgets that are assembled from other widgets.

use crate::engine::geometry::{Rect, Size};
use crate::engine::measure::ApproximateMeasure;
use crate::engine::runtime::Runtime;
use crate::engine::tree::{NodeKind, UiNode};
use crate::host::{Actions, NodeEvent};
use crate::ui::*;

/// A built and laid out element.
struct Built(Runtime);

fn build<M: 'static>(element: impl Into<Element<M>>) -> Built {
    let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
    runtime.update(&element.into(), Size::new(320.0, 600.0));
    Built(runtime)
}

fn walk<'a>(node: &'a UiNode, visit: &mut dyn FnMut(&'a UiNode)) {
    visit(node);
    for child in &node.children {
        walk(child, visit);
    }
}

impl Built {
    fn nodes(&self) -> Vec<&UiNode> {
        let mut nodes = Vec::new();
        walk(self.0.tree(), &mut |node| nodes.push(node));
        nodes
    }

    /// The texts on screen, in tree order.
    fn texts(&self) -> Vec<String> {
        self.nodes()
            .into_iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Widget(Widget::Text(props)) => Some(props.content.clone()),
                _ => None,
            })
            .collect()
    }

    fn text(&self, content: &str) -> &UiNode {
        self.nodes()
            .into_iter()
            .find(|node| matches!(&node.kind, NodeKind::Widget(Widget::Text(props)) if props.content == content))
            .unwrap_or_else(|| panic!("no text {content:?}"))
    }

    /// The node an accessibility label names.
    fn labelled(&self, label: &str) -> &UiNode {
        self.nodes()
            .into_iter()
            .find(|node| node.style.accessibility_label.as_deref() == Some(label))
            .unwrap_or_else(|| panic!("nothing is labelled {label:?}"))
    }

    fn frame(&self, node: &UiNode) -> Rect {
        self.0.frames()[&node.id]
    }

    /// The nearest tappable node around `inner`, itself included.
    fn tappable_around(&self, inner: &UiNode) -> Option<&UiNode> {
        fn path<'a>(node: &'a UiNode, target: &UiNode, trail: &mut Vec<&'a UiNode>) -> bool {
            trail.push(node);
            if node.id == target.id || node.children.iter().any(|child| path(child, target, trail))
            {
                return true;
            }
            trail.pop();
            false
        }
        let mut trail = Vec::new();
        path(self.0.tree(), inner, &mut trail);
        trail
            .into_iter()
            .rev()
            .find(|node| Actions::of(node).is_some_and(|actions| actions.click.is_set()))
    }
}

/// The message a tap on `node` sends.
fn tap<M: Clone + 'static>(node: &UiNode) -> Option<M> {
    Actions::of(node)?
        .message(&NodeEvent::Click)?
        .downcast::<M>()
        .ok()
        .map(|message| *message)
}

#[test]
fn a_grid_puts_equally_wide_cells_on_each_line() {
    let items: Vec<String> = ["a", "bbbbbb", "c", "d", "e"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let built = build::<()>(
        Column::new().width(210).height(400).child(
            Grid::new(items)
                .columns(2)
                .spacing(10)
                .key(|item| item.clone())
                .cell(|item| Text::new(item.clone()).into()),
        ),
    );
    assert_eq!(built.texts(), ["a", "bbbbbb", "c", "d", "e"]);
    let cell = |content: &str| built.frame(built.text(content));
    // Two cells of 100 points with 10 between them, whatever they hold.
    assert_eq!((cell("a").x, cell("a").width), (0.0, 100.0));
    assert_eq!((cell("bbbbbb").x, cell("bbbbbb").width), (110.0, 100.0));
    // The next line is below the first, with the same space between.
    assert_eq!(cell("c").x, 0.0);
    assert_eq!(cell("c").y, cell("a").y + cell("a").height + 10.0);
    // A line that is not full keeps its cell at the width of the others.
    assert_eq!((cell("e").x, cell("e").width), (0.0, 100.0));
}

#[test]
fn a_list_tile_is_a_tappable_row_of_at_least_56_points() {
    let built = build(
        Column::new().child(
            ListTile::new("Wi-Fi")
                .subtitle("Home")
                .trailing(Text::new("On"))
                .on_tap("open"),
        ),
    );
    assert_eq!(built.texts(), ["Wi-Fi", "Home", "On"]);
    let tile = built
        .tappable_around(built.text("Wi-Fi"))
        .expect("a tappable tile");
    assert_eq!(tap::<&str>(tile), Some("open"));
    let frame = built.frame(tile);
    assert_eq!(frame.width, 320.0);
    assert!(frame.height >= 56.0);
    // What trails ends at the padding of the far side.
    let trailing = built.frame(built.text("On"));
    assert_eq!(trailing.x + trailing.width, 320.0 - 16.0);

    // One line of text is centred in the minimum height.
    let built = build::<()>(Column::new().child(ListTile::new("About")));
    let title = built.frame(built.text("About"));
    assert_eq!(title.y + title.height / 2.0, 28.0);
}

#[test]
fn a_number_stepper_reports_the_next_value_inside_its_range() {
    let stepper = |value| build(NumberStepper::new(value).min(1).max(3).on_change(|v| v));
    let built = stepper(2.0);
    assert_eq!(built.texts(), ["2"]);
    assert_eq!(tap::<f32>(built.labelled("Decrease")), Some(1.0));
    assert_eq!(tap::<f32>(built.labelled("Increase")), Some(3.0));
    // At an end of the range the button that would leave it does nothing.
    let built = stepper(1.0);
    assert_eq!(tap::<f32>(built.labelled("Decrease")), None);
    assert_eq!(built.labelled("Decrease").style.opacity, Some(0.38));
    assert_eq!(tap::<f32>(built.labelled("Increase")), Some(2.0));

    // A fractional step shows its digits and does not drift.
    let built = build(NumberStepper::new(0.2).step(0.1).max(1).on_change(|v| v));
    assert_eq!(built.texts(), ["0.2"]);
    assert_eq!(tap::<f32>(built.labelled("Increase")), Some(0.3));
}

#[test]
fn a_chip_reports_its_tap_and_its_removal_apart() {
    let built = build(Chip::new("draft").on_tap("pick").on_delete("remove"));
    let chip = built
        .tappable_around(built.text("draft"))
        .expect("a tappable chip");
    assert_eq!(tap::<&str>(chip), Some("pick"));
    assert_eq!(tap::<&str>(built.labelled("Remove draft")), Some("remove"));
    // Unselected it has an outline, selected it is filled.
    assert!(chip.style.border.is_some() && chip.style.background.is_none());

    let built = build::<()>(Chip::new("Rust").selected(true));
    let chip = built.0.tree().children.first().expect("the chip");
    assert!(chip.style.border.is_none() && chip.style.background.is_some());
}

#[test]
fn a_badge_shows_a_count_unless_it_is_zero() {
    let badge = |count| build::<()>(Row::new().child(Badge::new(Text::new("Inbox")).count(count)));
    assert_eq!(badge(0).texts(), ["Inbox"]);
    assert_eq!(badge(7).texts(), ["Inbox", "7"]);
    assert_eq!(badge(120).texts(), ["Inbox", "99+"]);

    // The mark sits on the corner and does not change the size of the child.
    let built = badge(7);
    let child = built.frame(built.text("Inbox"));
    let stack = built.frame(&built.0.tree().children[0].children[0]);
    assert_eq!((stack.width, stack.height), (child.width, child.height));
    let mark = built.frame(built.text("7"));
    assert!(mark.x + mark.width > child.x + child.width - 8.0);
}

#[test]
fn an_expansion_tile_shows_its_content_only_while_expanded() {
    let tile = |expanded| {
        build(
            ExpansionTile::new("Advanced")
                .expanded(expanded)
                .on_toggle("toggle")
                .child(Text::new("Verbose logs")),
        )
    };
    let closed = tile(false);
    assert_eq!(closed.texts(), ["Advanced"]);
    assert_eq!(tap::<&str>(closed.labelled("Advanced")), Some("toggle"));
    assert_eq!(tile(true).texts(), ["Advanced", "Verbose logs"]);
}

#[test]
fn a_stepper_shows_the_current_step_and_reports_the_tapped_one() {
    let built = build(
        Stepper::new()
            .current(1)
            .on_step(|index| index)
            .step("Address", Text::new("Street"))
            .step("Payment", Text::new("Card"))
            .step("Confirm", Text::new("Summary")),
    );
    // A check mark for the step that is done, numbers for the others, and
    // only the current step's content.
    assert_eq!(
        built.texts(),
        ["Address", "2", "Payment", "Card", "3", "Confirm"]
    );
    assert_eq!(tap::<usize>(built.labelled("Confirm")), Some(2));
    assert_eq!(tap::<usize>(built.labelled("Address")), Some(0));
}

#[test]
fn a_search_bar_clears_through_on_change() {
    let field = |built: &Built| {
        built
            .nodes()
            .into_iter()
            .find(|node| matches!(node.kind, NodeKind::Widget(Widget::TextField(_))))
            .expect("the field")
            .clone()
    };
    let bar = |value: &str| {
        build(
            SearchBar::new(value)
                .on_change(|text| format!("query:{text}"))
                .on_submit("go".to_owned()),
        )
    };
    let built = bar("tea");
    assert_eq!(
        tap::<String>(built.labelled("Clear")),
        Some("query:".to_owned())
    );
    let actions = Actions::of(&field(&built)).expect("the field's handlers");
    let message = |event| {
        actions
            .message(&event)
            .and_then(|message| message.downcast::<String>().ok())
            .map(|message| *message)
    };
    assert_eq!(
        message(NodeEvent::Text("teas".into())),
        Some("query:teas".to_owned())
    );
    assert_eq!(message(NodeEvent::Submit), Some("go".to_owned()));
    // Nothing to clear, no button.
    let built = bar("");
    assert!(
        built
            .nodes()
            .into_iter()
            .all(|node| node.style.accessibility_label.as_deref() != Some("Clear"))
    );
}

#[test]
fn an_autocomplete_offers_matches_and_reports_the_picked_one() {
    let field = |value: &str, select: bool| {
        let field = Autocomplete::new(value, ["Kyoto", "Tokyo", "Oslo"])
            .on_change(|text| format!("typed:{text}"));
        build(if select {
            field.on_select(|option| format!("picked:{option}"))
        } else {
            field
        })
    };
    assert_eq!(field("", true).texts(), Vec::<String>::new());
    let built = field("kyo", true);
    assert_eq!(built.texts(), ["Kyoto", "Tokyo"]);
    let option = built
        .tappable_around(built.text("Tokyo"))
        .expect("a tappable option");
    assert_eq!(tap::<String>(option), Some("picked:Tokyo".to_owned()));
    // Without `on_select` a pick is reported as the new text.
    let built = field("kyo", false);
    let option = built
        .tappable_around(built.text("Tokyo"))
        .expect("a tappable option");
    assert_eq!(tap::<String>(option), Some("typed:Tokyo".to_owned()));
}

#[test]
fn the_button_looks_are_styles_of_the_same_button() {
    let filled: Element = Button::filled("Save").into();
    assert_eq!(filled.style.background, Some(Color::ACCENT));
    let outlined: Element = Button::outlined("Cancel").into();
    assert!(outlined.style.border.is_some() && outlined.style.background.is_none());
    // A style method after the look replaces what the look set.
    let custom: Element = Button::filled("Delete").background(0xFFD3_2F2F).into();
    assert_eq!(custom.style.background, Some(Color::fixed(0xFFD3_2F2F)));
}

#[test]
fn an_avatar_is_a_circle() {
    let built = build::<()>(
        Column::new()
            .align_items(Align::Start)
            .child(Avatar::initials("RW").radius(16)),
    );
    let avatar = &built.0.tree().children[0].children[0];
    let frame = built.frame(avatar);
    assert_eq!((frame.width, frame.height), (32.0, 32.0));
    assert_eq!(avatar.style.corner_radius, 16.0);
    // The initials are in the middle.
    let text = built.frame(built.text("RW"));
    assert!((text.x + text.width / 2.0 - 16.0).abs() < 0.01);
    assert!((text.y + text.height / 2.0 - 16.0).abs() < 0.01);
}

#[test]
fn a_divider_is_a_line_across_or_down() {
    let built = build::<()>(
        Column::new().child(Divider::new().indent(16)).child(
            Row::new()
                .height(40)
                .child(Divider::new().vertical().thickness(2)),
        ),
    );
    let column = &built.0.tree().children[0];
    assert_eq!(
        built.frame(&column.children[0]),
        Rect::new(16.0, 0.0, 304.0, 1.0)
    );
    let vertical = built.frame(&column.children[1].children[0]);
    assert_eq!((vertical.width, vertical.height), (2.0, 40.0));
}

/// The message a pick of line `index` of the menu on `node` sends.
fn pick<M: Clone + 'static>(node: &UiNode, index: usize) -> Option<M> {
    Actions::of(node)?
        .message(&NodeEvent::Select(index))?
        .downcast::<M>()
        .ok()
        .map(|message| *message)
}

fn menu_of(node: &UiNode) -> &MenuProps {
    match &node.kind {
        NodeKind::Container(props) => props.menu.as_ref().expect("a menu"),
        _ => panic!("not a container"),
    }
}

#[test]
fn a_menu_reports_the_line_that_was_picked() {
    let menu = || {
        Menu::new()
            .item("Rename", "rename")
            .item_with_icon("Share", Icon::add(), "share")
            .destructive("Delete", "delete")
    };
    let built = build(MenuButton::new("Actions", menu()));
    let button = &built.0.tree().children[0];
    let props = menu_of(button);
    assert_eq!(props.items.len(), 3);
    assert!(props.items[2].destructive && !props.on_long_press);
    assert_eq!(props.items[1].icon, Some(Icon::add()));
    assert_eq!(pick::<&str>(button, 0), Some("rename"));
    assert_eq!(pick::<&str>(button, 2), Some("delete"));
    // An index the menu does not have is dropped, not mapped to a line.
    assert_eq!(pick::<&str>(button, 3), None);
    // The button itself reports no tap: the platform opens the menu.
    assert_eq!(tap::<&str>(button), None);
    assert_eq!(built.texts(), ["Actions"]);

    // A context menu opens on a long press and leaves the tap alone.
    let built = build(Container::new().on_tap("open").context_menu(menu()));
    let container = &built.0.tree().children[0];
    assert!(menu_of(container).on_long_press);
    assert_eq!(tap::<&str>(container), Some("open"));
    assert_eq!(pick::<&str>(container, 1), Some("share"));

    // A menu without lines is no menu.
    let built = build(Container::new().menu(Menu::<&str>::new()));
    assert!(Actions::of(&built.0.tree().children[0]).is_none());
}

#[test]
fn a_dropdown_shows_the_picked_option_or_its_placeholder() {
    let dropdown = |selected: Option<usize>| {
        build(
            Dropdown::new(["Poland", "Japan"])
                .selected(selected)
                .placeholder("Country")
                .on_select(|index| index),
        )
    };
    let built = dropdown(None);
    assert_eq!(built.texts(), ["Country"]);
    let field = &built.0.tree().children[0];
    assert_eq!(menu_of(field).selected, None);
    assert_eq!(pick::<usize>(field, 1), Some(1));

    let built = dropdown(Some(1));
    assert_eq!(built.texts(), ["Japan"]);
    assert_eq!(menu_of(&built.0.tree().children[0]).selected, Some(1));
    // A selection the options do not have shows the placeholder.
    assert_eq!(dropdown(Some(9)).texts(), ["Country"]);
}

#[test]
fn a_segmented_control_shares_its_width_and_reports_the_other_segments() {
    let built = build(
        Column::new().width(302).child(
            SegmentedControl::new(["Day", "Week", "Month"])
                .selected(1)
                .on_select(|index| index),
        ),
    );
    let width = |label: &str| built.frame(built.labelled(label)).width;
    // 302 less the padding of 2 on each side and two gaps of 2, by three.
    assert_eq!(width("Day"), 98.0);
    assert_eq!(width("Week"), 98.0);
    assert_eq!(width("Month"), 98.0);
    assert_eq!(tap::<usize>(built.labelled("Month")), Some(2));
    // The picked segment is marked and does not report a tap.
    assert_eq!(tap::<usize>(built.labelled("Week")), None);
    assert!(built.labelled("Week").style.background.is_some());
    assert!(built.labelled("Day").style.background.is_none());
}

#[test]
fn a_tooltip_gives_a_layout_node_a_view() {
    let plain: Element = Row::new().into();
    assert!(!plain.style.needs_view());
    let explained: Element = Row::new().tooltip("Search every note").into();
    assert_eq!(
        explained.style.tooltip.as_deref(),
        Some("Search every note")
    );
    assert!(explained.style.needs_view());
}
