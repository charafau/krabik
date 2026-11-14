//! Procedural macros for `krabik-ui-dsl`.
//!
//! * [`view!`] lets an application write a widget tree declaratively. It is
//!   only syntax: it expands to the same builder calls you could write by
//!   hand.
//! * `#[derive(Widget)]` is used inside the framework. It generates the
//!   builder that every widget's properties struct needs.
//!
//! The logic lives in the `view` and `widget` modules as functions over
//! `proc_macro2` token streams, so it can be unit-tested without a compiler
//! in the loop.

mod view;
mod widget;

use proc_macro::TokenStream;

/// Builds a widget tree.
///
/// ```ignore
/// view! {
///     Column(padding: 16, spacing: 8) {
///         Text("Hello", size: 20, bold)
///         Button("+1", on_click: Msg::Increment)
///         if self.show_hint {
///             Text("Tap the button")
///         }
///         for item in &self.items {
///             row(item)
///         }
///     }
/// }
/// ```
///
/// # Rules
///
/// * `Name(...)` calls `Name::new(...)`. Arguments written as plain
///   expressions go to `new`; `name: value` calls `.name(value)` on the
///   result; a bare name after the first `name: value` calls `.name()`.
/// * `Name::constructor(...)` uses another constructor, for example
///   `DatePicker::time(self.time)`.
/// * Any chain of method calls may follow: `Text("Hello").bold()`.
/// * A `{ ... }` block after a node adds children with `.child(...)`.
/// * Capitalized names are builders. Anything else inside a block, such as
///   `self.footer()` or `row(item)`, is ordinary Rust that produces an
///   element and is added as a child too.
/// * `if`, `if let`, `for` and `match` work as in Rust, with nodes in their
///   bodies.
///
/// The result is an `Element`.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates a widget's builder from its properties struct.
///
/// Used inside `krabik-ui-dsl`; the generated code refers to the framework's
/// own modules through `crate::`. See the guide chapter on adding a widget.
///
/// On the struct: `#[widget(name = Slider)]` names the builder (and the
/// `Widget` variant, unless `variant = ...` says otherwise), `about = "..."`
/// documents it, `new = "..."` documents its constructor, and `text` adds
/// the shared `size`, `color` and `bold` setters.
///
/// On a field:
///
/// * `#[widget(arg)]` makes it a constructor argument.
/// * `#[widget(default = expr)]` sets its initial value.
/// * `#[widget(set)]` generates a setter named after the field. Options:
///   `name = other`, `into`, `some` (wrap in `Some`), `with = function` and
///   `from = Type` (transform the argument), `flag = value` (no argument).
/// * A `Handler<()>` field becomes `on_x(message)`; a `Handler<T>` field
///   becomes `on_x(|value| message)`.
#[proc_macro_derive(Widget, attributes(widget))]
pub fn derive_widget(input: TokenStream) -> TokenStream {
    widget::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
