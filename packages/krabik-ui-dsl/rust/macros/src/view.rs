//! `view!`: a declarative way to write a widget tree.
//!
//! The macro is syntax only. Every node becomes the builder calls an
//! application could write by hand, so any builder works without being
//! registered here, including ones an application defines itself.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, Pat, Path, Token, braced, parenthesized, token};

/// One thing that can appear where a child is expected.
enum Child {
    /// `Name(args) { children }`
    Node(Node),
    /// An ordinary expression that produces an element, such as `row(item)`.
    Embedded(Expr),
    If(Conditional),
    For {
        pattern: Pat,
        iterator: Expr,
        body: Vec<Child>,
    },
    Match {
        subject: Expr,
        arms: Vec<Arm>,
    },
}

struct Node {
    path: Path,
    /// Arguments for the constructor.
    positional: Vec<Expr>,
    /// `name: value` and bare `name` after the positional arguments.
    settings: Vec<Setting>,
    /// `.method(args)` calls written after the node.
    chain: Vec<(Ident, Punctuated<Expr, Token![,]>)>,
    children: Option<Vec<Child>>,
}

enum Setting {
    Value(Ident, Expr),
    Flag(Ident),
}

struct Conditional {
    condition: Expr,
    then: Vec<Child>,
    otherwise: Option<Otherwise>,
}

enum Otherwise {
    If(Box<Conditional>),
    Block(Vec<Child>),
}

struct Arm {
    pattern: Pat,
    guard: Option<Expr>,
    body: Vec<Child>,
}

pub(crate) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let children = parse_children.parse2(input)?;
    let mut children = children.into_iter();
    let root = match (children.next(), children.next()) {
        (Some(root), None) => root,
        (None, _) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "view! needs one root node, for example `view! { Column { ... } }`",
            ));
        }
        (Some(_), Some(_)) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "view! takes one root node; wrap several in a `Column { ... }` or `Row { ... }`",
            ));
        }
    };
    let root = match root {
        Child::Node(node) => node_expression(&node),
        Child::Embedded(expression) => quote! { #expression },
        Child::If(_) | Child::For { .. } | Child::Match { .. } => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "the root of view! must be a node; put `if`, `for` and `match` inside a \
                 container such as `Column { ... }`",
            ));
        }
    };
    Ok(quote! { ::krabik_ui_dsl::ui::Element::from(#root) })
}

fn parse_children(input: ParseStream) -> syn::Result<Vec<Child>> {
    let mut children = Vec::new();
    loop {
        // Separators are optional, so a list reads like a block of statements.
        while input.peek(Token![,]) || input.peek(Token![;]) {
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                input.parse::<Token![;]>()?;
            }
        }
        if input.is_empty() {
            return Ok(children);
        }
        children.push(input.parse()?);
    }
}

fn parse_block(input: ParseStream) -> syn::Result<Vec<Child>> {
    let content;
    braced!(content in input);
    parse_children(&content)
}

impl Parse for Child {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![if]) {
            Ok(Self::If(input.parse()?))
        } else if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let pattern = Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<Token![in]>()?;
            let iterator = Expr::parse_without_eager_brace(input)?;
            let body = parse_block(input)?;
            Ok(Self::For {
                pattern,
                iterator,
                body,
            })
        } else if input.peek(Token![match]) {
            input.parse::<Token![match]>()?;
            let subject = Expr::parse_without_eager_brace(input)?;
            let content;
            braced!(content in input);
            let mut arms = Vec::new();
            while !content.is_empty() {
                arms.push(content.parse()?);
            }
            Ok(Self::Match { subject, arms })
        } else if input.peek(token::Brace) {
            Err(input.error(
                "a `{ ... }` block here has no node to belong to; write an embedded \
                 expression without braces, for example `self.footer()`",
            ))
        } else if starts_with_node(input) {
            Ok(Self::Node(input.parse()?))
        } else {
            Ok(Self::Embedded(Expr::parse_without_eager_brace(input)?))
        }
    }
}

/// Whether the next child is a node rather than an ordinary expression.
///
/// Builders are types, so their names are capitalized: `Text`, `ui::Column`,
/// `DatePicker::time`. Anything else, such as `self.footer()` or `row(item)`,
/// is ordinary Rust that produces an element.
fn starts_with_node(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.call(Path::parse_mod_style).is_ok_and(|path| {
        path.segments.iter().any(|segment| {
            segment
                .ident
                .to_string()
                .chars()
                .next()
                .is_some_and(char::is_uppercase)
        })
    })
}

impl Parse for Conditional {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![if]>()?;
        let condition = Expr::parse_without_eager_brace(input)?;
        let then = parse_block(input)?;
        let otherwise = if input.peek(Token![else]) {
            input.parse::<Token![else]>()?;
            Some(if input.peek(Token![if]) {
                Otherwise::If(Box::new(input.parse()?))
            } else {
                Otherwise::Block(parse_block(input)?)
            })
        } else {
            None
        };
        Ok(Self {
            condition,
            then,
            otherwise,
        })
    }
}

impl Parse for Arm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pattern = Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        input.parse::<Token![=>]>()?;
        // `=> { ... }` is a block of children; anything else is one child.
        let body = if input.peek(token::Brace) {
            parse_block(input)?
        } else {
            vec![input.parse()?]
        };
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            pattern,
            guard,
            body,
        })
    }
}

impl Parse for Node {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.call(Path::parse_mod_style)?;
        let mut positional = Vec::new();
        let mut settings = Vec::new();

        if input.peek(token::Paren) {
            let content;
            parenthesized!(content in input);
            while !content.is_empty() {
                let named =
                    content.peek(Ident) && content.peek2(Token![:]) && !content.peek2(Token![::]);
                if named {
                    let name = content.parse()?;
                    content.parse::<Token![:]>()?;
                    settings.push(Setting::Value(name, content.parse()?));
                } else if !settings.is_empty() {
                    // After the first `name: value`, a bare name is a setting
                    // without an argument, such as `bold`.
                    let flag = content.fork();
                    let is_flag =
                        flag.parse::<Ident>().is_ok() && (flag.is_empty() || flag.peek(Token![,]));
                    if !is_flag {
                        return Err(content.error(
                            "constructor arguments must come before `name: value` settings",
                        ));
                    }
                    settings.push(Setting::Flag(content.parse()?));
                } else {
                    positional.push(content.parse()?);
                }
                if content.is_empty() {
                    break;
                }
                content.parse::<Token![,]>()?;
            }
        }

        let mut chain = Vec::new();
        while input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            let method: Ident = input.parse()?;
            if !input.peek(token::Paren) {
                return Err(syn::Error::new(
                    method.span(),
                    "expected a method call such as `.bold()`",
                ));
            }
            let content;
            parenthesized!(content in input);
            chain.push((method, Punctuated::parse_terminated(&content)?));
        }

        let children = if input.peek(token::Brace) {
            Some(parse_block(input)?)
        } else {
            None
        };

        Ok(Self {
            path,
            positional,
            settings,
            chain,
            children,
        })
    }
}

/// The builder expression for a node.
fn node_expression(node: &Node) -> TokenStream {
    let path = &node.path;
    let positional = &node.positional;
    // `DatePicker::time(...)` names its constructor; `Text(...)` means `new`.
    let names_constructor = path
        .segments
        .last()
        .and_then(|segment| segment.ident.to_string().chars().next())
        .is_some_and(char::is_lowercase);
    let mut builder = if names_constructor {
        quote! { #path(#(#positional),*) }
    } else {
        quote! { #path::new(#(#positional),*) }
    };

    for setting in &node.settings {
        builder = match setting {
            Setting::Value(name, value) => quote! { #builder.#name(#value) },
            Setting::Flag(name) => quote! { #builder.#name() },
        };
    }
    for (method, arguments) in &node.chain {
        builder = quote! { #builder.#method(#arguments) };
    }

    match &node.children {
        None => builder,
        Some(children) => {
            let statements = child_statements(children);
            quote! {
                {
                    let mut __node = #builder;
                    #statements
                    __node
                }
            }
        }
    }
}

/// Statements that add `children` to the builder in `__node`.
///
/// Children are added one statement at a time rather than through one
/// chained expression, so `if`, `for` and `match` can simply be the Rust
/// control flow they look like.
fn child_statements(children: &[Child]) -> TokenStream {
    let statements = children.iter().map(|child| match child {
        Child::Node(node) => {
            let expression = node_expression(node);
            quote! { __node = __node.child(#expression); }
        }
        Child::Embedded(expression) => quote! { __node = __node.child(#expression); },
        Child::If(conditional) => conditional_statement(conditional),
        Child::For {
            pattern,
            iterator,
            body,
        } => {
            let body = child_statements(body);
            quote! { for #pattern in #iterator { #body } }
        }
        Child::Match { subject, arms } => {
            let arms = arms.iter().map(|arm| {
                let pattern = &arm.pattern;
                let guard = arm.guard.iter();
                let body = child_statements(&arm.body);
                quote! { #pattern #(if #guard)* => { #body } }
            });
            quote! { match #subject { #(#arms)* } }
        }
    });
    quote! { #(#statements)* }
}

fn conditional_statement(conditional: &Conditional) -> TokenStream {
    let condition = &conditional.condition;
    let then = child_statements(&conditional.then);
    let otherwise = match &conditional.otherwise {
        None => TokenStream::new(),
        Some(Otherwise::If(next)) => {
            let next = conditional_statement(next);
            quote! { else #next }
        }
        Some(Otherwise::Block(children)) => {
            let children = child_statements(children);
            quote! { else { #children } }
        }
    };
    quote! { if #condition { #then } #otherwise }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The expansion without the outer `Element::from(...)`, with whitespace
    /// normalized so it can be compared with hand-written code.
    fn expanded(input: TokenStream) -> String {
        let code = expand(input).expect("expands").to_string();
        code.strip_prefix(":: krabik_ui_dsl :: ui :: Element :: from (")
            .and_then(|code| code.strip_suffix(')'))
            .expect("wrapped in Element::from")
            .trim()
            .to_owned()
    }

    fn same(input: TokenStream, expected: TokenStream) {
        assert_eq!(expanded(input), expected.to_string());
    }

    fn error(input: TokenStream) -> String {
        expand(input).expect_err("fails").to_string()
    }

    #[test]
    fn a_node_calls_new_then_its_settings() {
        same(
            quote! { Text("Hello", size: 20, color: BLUE, bold) },
            quote! { Text::new("Hello").size(20).color(BLUE).bold() },
        );
        same(quote! { Spacer }, quote! { Spacer::new() });
        same(
            quote! { Column(padding: 16) },
            quote! { Column::new().padding(16) },
        );
    }

    #[test]
    fn a_bare_name_before_any_setting_is_a_constructor_argument() {
        same(quote! { Text(title) }, quote! { Text::new(title) });
        same(
            quote! { Slider(self.volume, on_change: Msg::Volume) },
            quote! { Slider::new(self.volume).on_change(Msg::Volume) },
        );
    }

    #[test]
    fn a_lowercase_last_segment_is_the_constructor() {
        same(
            quote! { DatePicker::time(self.time, on_change: Msg::Time) },
            quote! { DatePicker::time(self.time).on_change(Msg::Time) },
        );
        same(quote! { ui::Text("Hi") }, quote! { ui::Text::new("Hi") });
    }

    #[test]
    fn method_calls_can_follow_a_node() {
        same(
            quote! { Text("Hello").bold().size(12) },
            quote! { Text::new("Hello").bold().size(12) },
        );
    }

    #[test]
    fn children_are_added_one_by_one() {
        same(
            quote! {
                Column(spacing: 8) {
                    Text("A")
                    self.footer()
                    row(item)
                    Text("B")
                }
            },
            quote! {
                {
                    let mut __node = Column::new().spacing(8);
                    __node = __node.child(Text::new("A"));
                    __node = __node.child(self.footer());
                    __node = __node.child(row(item));
                    __node = __node.child(Text::new("B"));
                    __node
                }
            },
        );
    }

    #[test]
    fn control_flow_stays_rust_control_flow() {
        same(
            quote! {
                Column {
                    if self.loading { Spinner } else if let Some(error) = &self.error {
                        Text(error)
                    } else {
                        Text("Done")
                    }
                    for (index, item) in self.items.iter().enumerate() {
                        Text(item.title.clone(), key: index.to_string())
                    }
                    match self.tab {
                        Tab::Home => Text("Home"),
                        Tab::Other(name) if name.is_empty() => {}
                        _ => { Text("A") Text("B") }
                    }
                }
            },
            quote! {
                {
                    let mut __node = Column::new();
                    if self.loading {
                        __node = __node.child(Spinner::new());
                    } else if let Some(error) = &self.error {
                        __node = __node.child(Text::new(error));
                    } else {
                        __node = __node.child(Text::new("Done"));
                    }
                    for (index, item) in self.items.iter().enumerate() {
                        __node = __node.child(Text::new(item.title.clone()).key(index.to_string()));
                    }
                    match self.tab {
                        Tab::Home => { __node = __node.child(Text::new("Home")); }
                        Tab::Other(name) if name.is_empty() => {}
                        _ => {
                            __node = __node.child(Text::new("A"));
                            __node = __node.child(Text::new("B"));
                        }
                    }
                    __node
                }
            },
        );
    }

    #[test]
    fn the_root_can_be_an_embedded_expression() {
        same(quote! { self.page() }, quote! { self.page() });
    }

    #[test]
    fn mistakes_are_reported_in_plain_words() {
        assert!(error(quote! {}).contains("needs one root node"));
        assert!(error(quote! { Text("A") Text("B") }).contains("takes one root node"));
        assert!(error(quote! { if x { Text("A") } }).contains("root of view! must be a node"));
        assert!(
            error(quote! { Text(size: 20, "Hello") })
                .contains("constructor arguments must come before")
        );
        assert!(error(quote! { Column { Text("A"), { x } } }).contains("without braces"));
        assert!(error(quote! { Text("A").bold }).contains("expected a method call"));
    }
}
