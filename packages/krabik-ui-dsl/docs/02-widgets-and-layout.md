# 2. Widgets, layout and styling

A view is a tree of widgets. Containers arrange their children; leaf widgets
are drawn by one native view each.

## Containers

| Widget | What it does |
|---|---|
| `Column` | Stacks children top to bottom. `.spacing(n)` adds a gap. |
| `Row` | Places children left to right. `.spacing(n)` adds a gap. |
| `Spacer` | Empty space that grows to push its neighbours apart. |
| `ScrollView` | Scrolls its single child vertically, or `.horizontal()`. |
| `List` | A virtualized list for hundreds or thousands of rows. |
| `Pager` | Swipeable pages, one child per page. |
| `Tabs` / `TabsView` | A tab bar above content / above a pager. |

```rust
Column::new()
    .padding(16)
    .spacing(8)
    .child(Text::new("Title").size(20).bold())
    .child(
        Row::new()
            .spacing(8)
            .child(Button::new("Cancel").on_click(Msg::Cancel))
            .child(Spacer::new())
            .child(Button::new("Save").on_click(Msg::Save)),
    )
```

A container with no background, border or other decoration creates no native
view at all: it only positions its children. Nesting containers for layout is
therefore free.

## Leaf widgets

| Widget | Native view (Android / iOS) | Reports |
|---|---|---|
| `Text` | `TextView` / `UILabel` | – |
| `Button` | `Button` / `UIButton` | `on_click` |
| `TextField` | `EditText` / `UITextField` | `on_change`, `on_submit`, `on_focus` |
| `Switch` | `Switch` / `UISwitch` | `on_toggle` |
| `Checkbox`, `RadioButton` | `CheckBox`, `RadioButton` / `UIButton` | `on_toggle` |
| `RadioGroup` | `RadioGroup` / `UIStackView` | `on_select` |
| `TabBar` | custom strip / `UISegmentedControl` | `on_select` |
| `DatePicker` | date and time pickers / `UIDatePicker` | `on_change` |
| `Slider` | `SeekBar` / `UISlider` | `on_change` |
| `ProgressIndicator` | `ProgressBar` / `UIProgressView`, `UIActivityIndicatorView` | – |
| `Image` | `ImageView` / `UIImageView` | – |

The application owns every value. A `Switch` does not remember that it is on:
you pass `Switch::new(self.dark)`, the user toggles it, you receive
`Msg::DarkMode(true)`, store it, and the next `view()` passes `true` back.

## Layout

Layout is flexbox, computed by [Taffy](https://github.com/DioxusLabs/taffy).
Every widget has these methods:

| Method | Meaning |
|---|---|
| `.width(n)`, `.height(n)` | A fixed size in logical points. |
| `.width_percent(f)`, `.height_percent(f)` | A fraction of the parent, `1.0` is all of it. |
| `.grow(f)` | Take a share of the space left over along the parent's direction. |
| `.aspect_ratio(f)` | Width divided by height. |
| `.padding(n)`, `.padding_each(EdgeInsets)` | Space inside the widget. |
| `.margin(n)`, `.margin_each(EdgeInsets)` | Space outside the widget. |
| `.align_items(Align)` | How a container aligns children across its direction. |
| `.justify_content(Justify)` | How a container distributes children along its direction. |

Sizes are logical points: density-independent pixels on Android, points on
iOS. A widget without a size takes the size of its content; text is measured
by the platform itself, so layout matches what the native view will draw.

## Decoration

| Method | Meaning |
|---|---|
| `.background(0xAARRGGBB)` | A solid background. |
| `.gradient(Gradient)` | A two-colour gradient. |
| `.corner_radius(n)`, `.corner_radii(CornerRadii)` | Rounded corners. |
| `.border(width, colour)` | A border. |
| `.opacity(f)` | Transparency from `0.0` to `1.0`. |
| `.elevation(n)` | A shadow. |
| `.enabled(false)` | Dims the widget and ignores input. |
| `.accessibility_label(text)` | The label a screen reader announces. |

Text-bearing widgets also have `.size(n)`, `.color(0xAARRGGBB)` and `.bold()`.

## Animation

A change of frame or decoration can animate instead of jumping:

```rust
Column::new()
    .height(if self.open { 200 } else { 0 })
    .animate(250)                      // animate changes over 250 ms
    .animation_curve(AnimationCurve::EaseInOut)
```

`.animate_enter(ms)` and `.animate_exit(ms)` fade a widget in when it first
appears and out when it is removed.

## Keys

When children are added, removed or reordered, the framework matches old and
new children by position. Give a child a `.key("...")` when it should keep its
native view (and with it focus, scroll position and running animations) even
though its position changes:

```rust
for todo in &self.todos {
    column = column.child(self.todo_row(todo).key(todo.id.to_string()));
}
```

Keys must be unique among siblings.

## Lists

A `Column` in a `ScrollView` builds every row. For long lists use `List`,
which only builds the rows near the viewport:

```rust
List::new(self.rows.clone())            // Arc<Vec<Row>>, cheap to clone
    .item_height(56)                     // optional: fixed row height
    .key(|row| row.id.to_string())
    .row(|row| {
        Row::new()
            .child(Text::new(&row.title).grow(1.0))
            .child(Button::new("Delete").on_click(Msg::Delete(row.id)))
            .into()
    })
```

Add `.recycled()` to let the platform's own recycler (`RecyclerView`,
`UICollectionView`) own scrolling and reuse row views; this is the right
choice for thousands of rows.

Next: [components and composition](03-components.md).
