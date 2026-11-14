//! `#[derive(Widget)]`: the builder for a widget's properties struct.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, Field, Fields, GenericArgument, Ident, LitStr, Path,
    PathArguments, Type, parse2,
};

/// What `#[widget(...)]` on the struct says.
struct Config {
    name: Ident,
    variant: Ident,
    about: Option<LitStr>,
    new_doc: Option<LitStr>,
    text: bool,
}

/// What `#[widget(...)]` on a field says.
#[derive(Default)]
struct FieldConfig {
    arg: bool,
    default: Option<Expr>,
    setter: Option<Setter>,
    /// The field holds a colour that is not resolved for dark mode.
    fixed_color: bool,
}

#[derive(Default)]
struct Setter {
    name: Option<Ident>,
    into: bool,
    some: bool,
    with: Option<Path>,
    from: Option<Type>,
    flag: Option<Expr>,
}

pub(crate) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;
    let props = &input.ident;
    let config = struct_config(&input)?;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    props,
                    "#[derive(Widget)] needs a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                props,
                "#[derive(Widget)] can only be used on a struct",
            ));
        }
    };

    let name = &config.name;
    let variant = &config.variant;
    // The pattern every setter uses to reach its properties.
    let access = quote! {
        crate::element::ElementKind::Widget(crate::widgets::Widget::#variant(props))
    };

    let mut new_params = Vec::new();
    let mut new_args = Vec::new();
    let mut initializers = Vec::new();
    let mut methods = Vec::new();
    let mut handlers = Vec::new();
    let mut colors = Vec::new();

    for field in fields {
        let ident = field.ident.as_ref().expect("named field");
        let field_config = field_config(field)?;
        let docs = doc_attributes(&field.attrs);
        if !field_config.fixed_color {
            // Colours are resolved for the appearance wherever they sit.
            if is_color(&field.ty) {
                colors.push(quote! { self.#ident = self.#ident.resolve(scheme); });
            } else if option_payload(&field.ty).is_some_and(is_color) {
                colors.push(quote! {
                    if let Some(color) = &mut self.#ident {
                        *color = color.resolve(scheme);
                    }
                });
            } else if vec_payload(&field.ty).is_some_and(is_color) {
                colors.push(quote! {
                    for color in &mut self.#ident {
                        *color = color.resolve(scheme);
                    }
                });
            }
        }

        if field_config.arg {
            let (param, convert) = constructor_argument(&field.ty);
            new_params.push(quote! { #ident: #param });
            new_args.push(quote! { #ident });
            initializers.push(if convert.is_empty() {
                quote! { #ident }
            } else {
                quote! { #ident: #ident #convert }
            });
        } else if let Some(default) = &field_config.default {
            initializers.push(quote! { #ident: #default });
        } else if handler_payload(&field.ty).is_some() {
            initializers.push(quote! { #ident: crate::message::Handler::none() });
        } else {
            initializers.push(quote! { #ident: ::core::default::Default::default() });
        }

        if let Some(payload) = handler_payload(&field.ty) {
            if field_config.setter.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "a `Handler` field already gets an event method; remove `set`",
                ));
            }
            handlers.push(ident.clone());
            methods.push(if is_unit(payload) {
                quote! {
                    #(#docs)*
                    pub fn #ident(mut self, message: M) -> Self
                    where
                        M: Clone + Send + Sync + 'static,
                    {
                        if let #access = &mut self.0.kind {
                            props.#ident = crate::message::Handler::new(move |()| message.clone());
                        }
                        self
                    }
                }
            } else {
                quote! {
                    #(#docs)*
                    pub fn #ident(
                        mut self,
                        message: impl Fn(#payload) -> M + Send + Sync + 'static,
                    ) -> Self
                    where
                        M: 'static,
                    {
                        if let #access = &mut self.0.kind {
                            props.#ident = crate::message::Handler::new(message);
                        }
                        self
                    }
                }
            });
        } else if let Some(setter) = &field_config.setter {
            methods.push(setter_method(ident, &field.ty, setter, &docs, &access)?);
        }
    }

    let about = config.about.iter();
    let new_doc = match &config.new_doc {
        Some(doc) => doc.value(),
        None => "Creates the widget.".to_owned(),
    };
    let text_style = config.text.then(|| {
        quote! {
            fn text_style(&mut self) -> Option<crate::widgets::TextStyleMut<'_>> {
                Some(crate::widgets::TextStyleMut {
                    size: &mut self.size,
                    color: &mut self.color,
                    weight: &mut self.weight,
                })
            }
        }
    });
    let text_builder = config
        .text
        .then(|| quote! { crate::element::text_builder!(#name); });

    Ok(quote! {
        impl #props {
            #[doc = #new_doc]
            pub fn new(#(#new_params),*) -> Self {
                Self { #(#initializers),* }
            }
        }

        #(#[doc = #about])*
        pub struct #name<M = ()>(
            pub(crate) crate::element::RawElement,
            pub(crate) ::core::marker::PhantomData<fn() -> M>,
        );

        impl<M> #name<M> {
            #[doc = #new_doc]
            pub fn new(#(#new_params),*) -> Self {
                Self(
                    crate::element::RawElement::leaf(crate::element::ElementKind::Widget(
                        crate::widgets::Widget::#variant(#props::new(#(#new_args),*)),
                    )),
                    ::core::marker::PhantomData,
                )
            }

            #(#methods)*
        }

        impl<M> From<#name<M>> for crate::element::Element<M> {
            fn from(component: #name<M>) -> Self {
                crate::element::Element::from_raw(component.0)
            }
        }

        crate::element::style_builder!(#name);
        #text_builder

        impl crate::widgets::WidgetProps for #props {
            fn map_messages(&mut self, wrap: &crate::message::Wrap) {
                let _ = wrap;
                #(self.#handlers.map(wrap);)*
            }

            #text_style

            fn resolve_colors(&mut self, scheme: crate::style::ColorScheme) {
                let _ = scheme;
                #(#colors)*
            }
        }
    })
}

fn struct_config(input: &DeriveInput) -> syn::Result<Config> {
    let mut name = None;
    let mut variant = None;
    let mut about = None;
    let mut new_doc = None;
    let mut text = false;
    for attr in widget_attributes(&input.attrs) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                name = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("variant") {
                variant = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("about") {
                about = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("new") {
                new_doc = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("text") {
                text = true;
            } else {
                return Err(meta.error(
                    "unknown option; expected `name`, `variant`, `about`, `new` or `text`",
                ));
            }
            Ok(())
        })?;
    }
    let name: Ident = name.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            "missing `#[widget(name = ...)]`: the name of the builder to generate",
        )
    })?;
    Ok(Config {
        variant: variant.unwrap_or_else(|| name.clone()),
        name,
        about,
        new_doc,
        text,
    })
}

fn field_config(field: &Field) -> syn::Result<FieldConfig> {
    let mut config = FieldConfig::default();
    for attr in widget_attributes(&field.attrs) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("arg") {
                config.arg = true;
            } else if meta.path.is_ident("default") {
                config.default = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("set") {
                let mut setter = Setter::default();
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|option| {
                        if option.path.is_ident("name") {
                            setter.name = Some(option.value()?.parse()?);
                        } else if option.path.is_ident("into") {
                            setter.into = true;
                        } else if option.path.is_ident("some") {
                            setter.some = true;
                        } else if option.path.is_ident("with") {
                            setter.with = Some(option.value()?.parse()?);
                        } else if option.path.is_ident("from") {
                            setter.from = Some(option.value()?.parse()?);
                        } else if option.path.is_ident("flag") {
                            setter.flag = Some(option.value()?.parse()?);
                        } else {
                            return Err(option.error(
                                "unknown option; expected `name`, `into`, `some`, `with`, \
                                 `from` or `flag`",
                            ));
                        }
                        Ok(())
                    })?;
                }
                config.setter = Some(setter);
            } else if meta.path.is_ident("fixed_color") {
                config.fixed_color = true;
            } else {
                return Err(
                    meta.error("unknown option; expected `arg`, `default`, `set` or `fixed_color`")
                );
            }
            Ok(())
        })?;
    }
    if config.arg && config.default.is_some() {
        return Err(syn::Error::new_spanned(
            field,
            "a constructor argument cannot also have a default",
        ));
    }
    Ok(config)
}

fn setter_method(
    field: &Ident,
    ty: &Type,
    setter: &Setter,
    docs: &[&Attribute],
    access: &TokenStream,
) -> syn::Result<TokenStream> {
    let name = setter.name.clone().unwrap_or_else(|| field.clone());

    if let Some(flag) = &setter.flag {
        return Ok(quote! {
            #(#docs)*
            pub fn #name(mut self) -> Self {
                if let #access = &mut self.0.kind {
                    props.#field = #flag;
                }
                self
            }
        });
    }

    // The type the stored value is built from, before `some` wraps it.
    let inner = if setter.some {
        option_payload(ty).ok_or_else(|| {
            syn::Error::new_spanned(ty, "`set(some)` needs a field of type `Option<...>`")
        })?
    } else {
        ty
    };
    let into = setter.into || is_string(inner);
    let parameter = match &setter.from {
        Some(from) => quote! { #from },
        None if into => quote! { impl Into<#inner> },
        None => quote! { #inner },
    };
    let mut value = quote! { #field };
    if let Some(with) = &setter.with {
        value = quote! { #with(#value) };
    } else if into && setter.from.is_none() {
        value = quote! { #value.into() };
    }
    if setter.some {
        value = quote! { Some(#value) };
    }

    Ok(quote! {
        #(#docs)*
        pub fn #name(mut self, #field: #parameter) -> Self {
            if let #access = &mut self.0.kind {
                props.#field = #value;
            }
            self
        }
    })
}

/// The parameter type and conversion for a constructor argument.
fn constructor_argument(ty: &Type) -> (TokenStream, TokenStream) {
    if is_string(ty) {
        (quote! { impl Into<String> }, quote! { .into() })
    } else if vec_payload(ty).is_some_and(is_string) {
        (
            quote! { impl IntoIterator<Item = impl Into<String>> },
            quote! { .into_iter().map(Into::into).collect() },
        )
    } else {
        (quote! { #ty }, TokenStream::new())
    }
}

fn widget_attributes(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|attr| attr.path().is_ident("widget"))
}

fn doc_attributes(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect()
}

/// The `A` of a `Handler<A>` field.
fn handler_payload(ty: &Type) -> Option<&Type> {
    generic_payload(ty, "Handler")
}

fn option_payload(ty: &Type) -> Option<&Type> {
    generic_payload(ty, "Option")
}

fn vec_payload(ty: &Type) -> Option<&Type> {
    generic_payload(ty, "Vec")
}

/// The single type argument of `Wrapper<T>`, whatever path `Wrapper` has.
fn generic_payload<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.first()? {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    }
}

fn is_string(ty: &Type) -> bool {
    is_named(ty, "String")
}

/// Whether the field is a `Color`, however the path to it is written.
fn is_color(ty: &Type) -> bool {
    is_named(ty, "Color")
}

/// Whether the type is `name` without type arguments, written bare or with
/// a path in front (`Color`, `style::Color`, `crate::style::Color`).
///
/// A macro sees names, not types: an alias such as `type Tint = Color` is
/// not recognised, and an unrelated type that happens to be called `Color`
/// is. Use the real name, or `#[widget(fixed_color)]` to opt a field out.
fn is_named(ty: &Type, name: &str) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name && segment.arguments.is_none())
}

fn is_unit(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expanded(input: TokenStream) -> String {
        expand(input).expect("expands").to_string()
    }

    fn error(input: TokenStream) -> String {
        expand(input).expect_err("fails").to_string()
    }

    #[test]
    fn generates_the_builder_constructor_and_conversions() {
        let code = expanded(quote! {
            #[widget(name = Badge, about = "A badge.")]
            pub struct BadgeProps {
                #[widget(arg)]
                pub label: String,
                #[widget(default = 3, set)]
                pub count: u32,
            }
        });
        assert!(code.contains("pub struct Badge < M = () >"), "{code}");
        assert!(code.contains("pub fn new (label : impl Into < String >) -> Self"));
        assert!(code.contains("label : label . into ()"));
        assert!(code.contains("count : 3"));
        assert!(code.contains("pub fn count (mut self , count : u32) -> Self"));
        assert!(code.contains("crate :: widgets :: Widget :: Badge (BadgeProps :: new (label))"));
        assert!(
            code.contains("impl < M > From < Badge < M >> for crate :: element :: Element < M >")
        );
        assert!(code.contains("crate :: element :: style_builder ! (Badge) ;"));
        assert!(!code.contains("text_builder"));
    }

    #[test]
    fn a_unit_handler_takes_a_message_and_a_payload_handler_takes_a_function() {
        let code = expanded(quote! {
            #[widget(name = Thing)]
            pub struct ThingProps {
                pub on_click: Handler<()>,
                pub on_change: Handler<f32>,
            }
        });
        assert!(
            code.contains("pub fn on_click (mut self , message : M) -> Self"),
            "{code}"
        );
        assert!(code.contains("message : impl Fn (f32) -> M + Send + Sync + 'static"));
        assert!(code.contains("on_click : crate :: message :: Handler :: none ()"));
        assert!(code.contains("self . on_click . map (wrap) ;"));
        assert!(code.contains("self . on_change . map (wrap) ;"));
    }

    #[test]
    fn setter_options_shape_the_argument_and_the_stored_value() {
        let code = expanded(quote! {
            #[widget(name = Thing, variant = Other, text)]
            pub struct ThingProps {
                #[widget(set(name = text_align))]
                pub align: TextAlign,
                #[widget(set(some))]
                pub selected: Option<usize>,
                #[widget(set(with = positive, from = f32))]
                pub step: Option<f32>,
                #[widget(set(flag = true))]
                pub ellipsis: bool,
                #[widget(set)]
                pub placeholder: String,
                pub size: f32,
                pub color: Color,
                pub weight: FontWeight,
            }
        });
        assert!(
            code.contains("pub fn text_align (mut self , align : TextAlign) -> Self"),
            "{code}"
        );
        assert!(code.contains("pub fn selected (mut self , selected : usize) -> Self"));
        assert!(code.contains("props . selected = Some (selected)"));
        assert!(code.contains("pub fn step (mut self , step : f32) -> Self"));
        assert!(code.contains("props . step = positive (step)"));
        assert!(code.contains("pub fn ellipsis (mut self) -> Self"));
        assert!(code.contains("props . ellipsis = true"));
        assert!(code.contains("placeholder : impl Into < String >"));
        assert!(code.contains("crate :: widgets :: Widget :: Other (props)"));
        assert!(code.contains("crate :: element :: text_builder ! (Thing) ;"));
        assert!(code.contains("fn text_style"));
        assert!(code.contains("self . color = self . color . resolve (scheme) ;"));
    }

    #[test]
    fn colours_are_resolved_wherever_they_sit() {
        let code = expanded(quote! {
            #[widget(name = Thing)]
            pub struct ThingProps {
                pub tint: Color,
                pub qualified: crate::style::Color,
                pub maybe: Option<Color>,
                pub stops: Vec<style::Color>,
                #[widget(fixed_color)]
                pub brand: Color,
                pub count: u32,
            }
        });
        assert!(
            code.contains("self . tint = self . tint . resolve (scheme) ;"),
            "{code}"
        );
        assert!(code.contains("self . qualified = self . qualified . resolve (scheme) ;"));
        assert!(code.contains("if let Some (color) = & mut self . maybe"));
        assert!(code.contains("for color in & mut self . stops"));
        assert!(!code.contains("self . brand . resolve"));
        assert!(!code.contains("self . count . resolve"));
    }

    #[test]
    fn mistakes_are_reported_in_plain_words() {
        assert!(
            error(quote! { pub struct Props { pub a: u32 } }).contains("missing `#[widget(name")
        );
        assert!(
            error(quote! { #[widget(name = A)] pub enum Props { B } })
                .contains("can only be used on a struct")
        );
        assert!(
            error(quote! {
                #[widget(name = A)]
                pub struct Props { #[widget(colour)] pub a: u32 }
            })
            .contains("unknown option")
        );
        assert!(
            error(quote! {
                #[widget(name = A)]
                pub struct Props { #[widget(set)] pub on_tap: Handler<()> }
            })
            .contains("already gets an event method")
        );
        assert!(
            error(quote! {
                #[widget(name = A)]
                pub struct Props { #[widget(set(some))] pub a: u32 }
            })
            .contains("needs a field of type `Option<...>`")
        );
    }
}
