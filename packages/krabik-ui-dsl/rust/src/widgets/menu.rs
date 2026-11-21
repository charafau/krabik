//! Menus: the platform's popup menu on a container, and the two buttons
//! that are made of it.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::{Handler, MessageMap};
use crate::style::{Align, Border, Color, EdgeInsets, Icon, Justify};
use crate::widgets::{Container, Image, Row, Text};

/// One line of a menu, as the platform shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuItem {
    /// The text of the line.
    pub label: String,
    /// An icon beside the text, where the platform shows one.
    pub icon: Option<Icon>,
    /// Whether the line removes or destroys something; shown in red.
    pub destructive: bool,
}

/// The menu a container opens; see [`Container::menu`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuProps {
    /// The lines, top to bottom.
    pub items: Vec<MenuItem>,
    /// The line shown as the current choice, if any.
    pub selected: Option<usize>,
    /// Whether a long press opens the menu; otherwise a tap does.
    pub on_long_press: bool,
    /// Reports the index of the line that was picked.
    pub on_select: Handler<usize>,
}

impl MenuProps {
    pub(crate) fn map_messages(&mut self, wrap: &MessageMap) {
        self.on_select.map(wrap);
    }
}

/// The lines of a popup menu and the message each one sends.
///
/// ```ignore
/// Menu::new()
///     .item("Rename", Msg::Rename(id))
///     .item_with_icon("Share", Icon::new().drawable("ic_share").sf("square.and.arrow.up"), Msg::Share(id))
///     .destructive("Delete", Msg::Delete(id))
/// ```
///
/// A menu is shown by the platform: a `PopupMenu` on Android, a `UIMenu` on
/// iOS. Give it to a [`MenuButton`], or to any container with
/// [`Container::menu`] (a tap opens it) or [`Container::context_menu`] (a
/// long press does).
pub struct Menu<M = ()> {
    items: Vec<MenuItem>,
    messages: Vec<M>,
}

impl<M> Menu<M> {
    /// Creates a menu with no lines.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            messages: Vec::new(),
        }
    }

    fn push(mut self, item: MenuItem, message: M) -> Self {
        self.items.push(item);
        self.messages.push(message);
        self
    }

    /// Appends a line that sends `message`.
    pub fn item(self, label: impl Into<String>, message: M) -> Self {
        let label = label.into();
        self.push(
            MenuItem {
                label,
                ..MenuItem::default()
            },
            message,
        )
    }

    /// Appends a line with an icon.
    pub fn item_with_icon(self, label: impl Into<String>, icon: Icon, message: M) -> Self {
        let label = label.into();
        self.push(
            MenuItem {
                label,
                icon: Some(icon),
                destructive: false,
            },
            message,
        )
    }

    /// Appends a line for an action that removes or destroys something.
    pub fn destructive(self, label: impl Into<String>, message: M) -> Self {
        let label = label.into();
        self.push(
            MenuItem {
                label,
                icon: None,
                destructive: true,
            },
            message,
        )
    }
}

impl<M> Default for Menu<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + Send + Sync + 'static> Menu<M> {
    pub(crate) fn into_props(self, on_long_press: bool) -> MenuProps {
        let messages = self.messages;
        MenuProps {
            on_select: if self.items.is_empty() {
                Handler::none()
            } else {
                // `Actions::message` drops an index this menu does not have.
                Handler::new(move |index: usize| messages[index].clone())
            },
            items: self.items,
            selected: None,
            on_long_press,
        }
    }
}

impl<M: Clone + Send + Sync + 'static> Container<M> {
    /// Opens `menu` when the container is tapped, anchored to it. The
    /// container then reports no tap of its own.
    pub fn menu(self, menu: Menu<M>) -> Self {
        self.menu_props(menu.into_props(false))
    }

    /// Opens `menu` when the container is pressed and held. On iOS this is
    /// the system's context menu, which lifts the container while the menu
    /// is open. The container then reports no long press of its own.
    pub fn context_menu(self, menu: Menu<M>) -> Self {
        self.menu_props(menu.into_props(true))
    }
}

impl<M> Container<M> {
    /// Sets the erased menu; for widgets that are built from a container.
    pub(crate) fn menu_props(mut self, menu: MenuProps) -> Self {
        if let ElementKind::Container(props) = &mut self.0.kind {
            props.menu = (!menu.items.is_empty()).then_some(menu);
        }
        self
    }
}

/// A button that opens a menu.
///
/// ```ignore
/// MenuButton::new("Sort", Menu::new().item("By name", Msg::ByName).item("By date", Msg::ByDate))
/// MenuButton::icon(Icon::more(), "More actions", menu)
/// ```
///
/// The first form shows the text and a small arrow. The second shows only
/// the icon; its text says what the button is for and is announced by a
/// screen reader, as with [`IconButton`](crate::ui::IconButton).
pub struct MenuButton<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    MenuButtonParts,
);

struct MenuButtonParts {
    label: Option<String>,
    icon: Option<Icon>,
    menu: MenuProps,
    tint: Color,
}

impl<M: Clone + Send + Sync + 'static> MenuButton<M> {
    /// Creates a button with a text that opens `menu`.
    pub fn new(label: impl Into<String>, menu: Menu<M>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.min_height = Some(44.0);
        shell.style.padding = EdgeInsets::symmetric(12.0, 0.0);
        shell.style.corner_radius = 8.0;
        shell.style.justify_content = Justify::Center;
        Self(
            shell,
            PhantomData,
            MenuButtonParts {
                label: Some(label.into()),
                icon: None,
                menu: menu.into_props(false),
                tint: Color::TEXT,
            },
        )
    }

    /// Creates a button that is an icon and opens `menu`.
    pub fn icon(icon: Icon, label: impl Into<String>, menu: Menu<M>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.min_width = Some(44.0);
        shell.style.min_height = Some(44.0);
        shell.style.corner_radius = 22.0;
        shell.style.align_items = Align::Center;
        shell.style.justify_content = Justify::Center;
        shell.style.accessibility_label = Some(label.into());
        Self(
            shell,
            PhantomData,
            MenuButtonParts {
                label: None,
                icon: Some(icon),
                menu: menu.into_props(false),
                tint: Color::TEXT,
            },
        )
    }
}

impl<M> MenuButton<M> {
    /// Sets the color of the text and the icons.
    pub fn tint(mut self, color: impl Into<Color>) -> Self {
        self.2.tint = color.into();
        self
    }
}

impl<M: 'static> From<MenuButton<M>> for Element<M> {
    fn from(component: MenuButton<M>) -> Self {
        let MenuButton(shell, _, parts) = component;
        let container = Container::<M>::new().menu_props(parts.menu);
        let content: Element<M> = match (parts.label, parts.icon) {
            (Some(label), _) => Row::new()
                .spacing(4)
                .align_items(Align::Center)
                .child(Text::new(label).color(parts.tint))
                .child(Image::icon(Icon::chevron_down()).size(20).tint(parts.tint))
                .into(),
            (None, icon) => Image::icon(icon.unwrap_or_else(Icon::more))
                .tint(parts.tint)
                .into(),
        };
        finish(shell, container.child(content))
    }
}

style_builder!(MenuButton);

/// A field that shows one of several options and opens a menu to pick
/// another.
///
/// ```ignore
/// Dropdown::new(["Poland", "Japan", "Norway"])
///     .selected(self.country)
///     .placeholder("Country")
///     .on_select(Msg::Country)
/// ```
///
/// The choice is the application's state: `on_select` reports the index
/// that was picked and the field shows whatever `selected` says. The menu
/// marks the current option.
pub struct Dropdown<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    DropdownParts<M>,
);

struct DropdownParts<M> {
    options: Vec<String>,
    selected: Option<usize>,
    placeholder: String,
    on_select: Option<Arc<dyn Fn(usize) -> M + Send + Sync>>,
}

impl<M> Dropdown<M> {
    /// Creates a dropdown over the given options, with none picked.
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.min_height = Some(48.0);
        shell.style.min_width = Some(120.0);
        shell.style.padding = EdgeInsets {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 12.0,
        };
        shell.style.corner_radius = 8.0;
        shell.style.border = Some(Border::new(1.0, Color::OUTLINE));
        shell.style.justify_content = Justify::Center;
        Self(
            shell,
            PhantomData,
            DropdownParts {
                options: options.into_iter().map(Into::into).collect(),
                selected: None,
                placeholder: String::new(),
                on_select: None,
            },
        )
    }

    /// Sets which option is shown, counting from zero; `None` shows the
    /// placeholder.
    pub fn selected(mut self, index: impl Into<Option<usize>>) -> Self {
        self.2.selected = index.into();
        self
    }

    /// Sets the text shown while no option is picked.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.2.placeholder = placeholder.into();
        self
    }

    /// Reports the index of the option that was picked.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self {
        self.2.on_select = Some(Arc::new(message));
        self
    }
}

impl<M: 'static> From<Dropdown<M>> for Element<M> {
    fn from(component: Dropdown<M>) -> Self {
        let Dropdown(shell, _, parts) = component;
        let selected = parts.selected.filter(|index| *index < parts.options.len());
        let shown = match selected {
            Some(index) => Text::new(parts.options[index].clone()),
            None => Text::new(parts.placeholder).color(Color::SECONDARY_TEXT),
        };
        let menu = MenuProps {
            items: parts
                .options
                .into_iter()
                .map(|label| MenuItem {
                    label,
                    ..MenuItem::default()
                })
                .collect(),
            selected,
            on_long_press: false,
            on_select: match parts.on_select {
                Some(on_select) => Handler::new(move |index| on_select(index)),
                None => Handler::none(),
            },
        };
        finish(
            shell,
            Container::<M>::new().menu_props(menu).child(
                Row::new()
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(shown.grow(1).key("value"))
                    .child(
                        Image::icon(Icon::chevron_down())
                            .tint(Color::SECONDARY_TEXT)
                            .key("arrow"),
                    ),
            ),
        )
    }
}

style_builder!(Dropdown);
