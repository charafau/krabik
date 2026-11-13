//! The public UI vocabulary: widgets, layout values and decoration.
//!
//! Everything an application needs to describe its interface is re-exported
//! here; the definitions live in [`crate::widgets`], [`crate::style`] and
//! [`crate::element`].

pub(crate) use crate::element::ElementKind;
pub use crate::element::{Element, RawElement};
pub use crate::message::Handler;
pub use crate::style::*;
pub use crate::widgets::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_convert_into_elements() {
        let element: Element = Column::new()
            .spacing(8)
            .padding(16)
            .child(Text::new("Hello").bold())
            .child(Button::new("Click").on_click(()))
            .into();

        assert_eq!(element.children().len(), 2);
        assert_eq!(element.style.gap, 8.0);
        assert_eq!(element.style.padding, EdgeInsets::all(16.0));

        let ElementKind::Widget(Widget::Text(text)) = &element.children()[0].kind else {
            panic!("expected text");
        };
        assert_eq!(text.content, "Hello");
        assert_eq!(text.weight, FontWeight::Bold);

        let ElementKind::Widget(Widget::Button(button)) = &element.children()[1].kind else {
            panic!("expected button");
        };
        assert_eq!(button.label, "Click");
        assert!(button.on_click.is_set());
    }

    #[test]
    fn spacer_grows_by_default() {
        let element: Element = Spacer::new().into();
        assert_eq!(element.style.grow, 1.0);
        assert!(element.children().is_empty());
    }

    #[test]
    fn keys_are_kept_on_the_element() {
        let element: Element = Text::new("a").key("first").into();
        assert_eq!(element.key(), Some("first"));
    }

    #[test]
    fn switch_keeps_its_state_and_action() {
        let element: Element = Switch::new(true).on_toggle(|_| ()).into();

        let ElementKind::Widget(Widget::Switch(props)) = &element.kind else {
            panic!("expected switch");
        };
        assert!(props.checked);
        assert!(props.on_toggle.is_set());
    }

    #[test]
    fn radio_group_keeps_options_and_selection() {
        let element: Element = RadioGroup::new(["Small", "Medium", "Large"])
            .selected(1)
            .on_select(|_| ())
            .spacing(12)
            .into();

        let ElementKind::Widget(Widget::RadioGroup(props)) = &element.kind else {
            panic!("expected radio group");
        };
        assert_eq!(props.options, ["Small", "Medium", "Large"]);
        assert_eq!(props.selected, Some(1));
        assert!(props.on_select.is_set());
        assert_eq!(props.spacing, 12.0);
    }

    #[test]
    fn slider_step_is_optional() {
        let element: Element = Slider::new(0.5).step(0.25).into();
        let ElementKind::Widget(Widget::Slider(props)) = &element.kind else {
            panic!("expected slider");
        };
        assert_eq!(props.step, Some(0.25));

        let element: Element = Slider::new(0.5).step(0.0).into();
        let ElementKind::Widget(Widget::Slider(props)) = &element.kind else {
            panic!("expected slider");
        };
        assert_eq!(props.step, None);
    }

    #[test]
    fn slider_values_snap_to_the_step() {
        assert_eq!(snap_slider_value(37.0, 0.0, 100.0, Some(10.0)), 40.0);
        assert_eq!(snap_slider_value(34.0, 0.0, 100.0, Some(10.0)), 30.0);
        assert_eq!(snap_slider_value(9.0, 1.0, 5.0, Some(1.0)), 5.0);
        assert_eq!(snap_slider_value(0.5, 1.0, 5.0, Some(1.0)), 1.0);
        assert_eq!(snap_slider_value(37.0, 0.0, 100.0, None), 37.0);
        assert_eq!(snap_slider_value(-4.0, 0.0, 100.0, None), 0.0);
        assert_eq!(snap_slider_value(140.0, 0.0, 100.0, None), 100.0);
    }

    #[test]
    fn indeterminate_progress_is_a_spinner() {
        let element: Element = ProgressIndicator::indeterminate().color(0xFF00_897B).into();
        let ElementKind::Widget(Widget::Spinner(props)) = &element.kind else {
            panic!("expected spinner");
        };
        assert_eq!(props.color, 0xFF00_897B);
    }

    #[test]
    fn dates_format_and_clamp() {
        assert_eq!(DateValue::new(2026, 11, 12).to_string(), "2026-11-12");
        assert_eq!(DateValue::new(2026, 0, 0), DateValue::new(2026, 1, 1));
        assert_eq!(DateValue::new(2026, 13, 40), DateValue::new(2026, 12, 31));
    }

    #[test]
    fn times_format_and_clamp() {
        assert_eq!(TimeValue::new(9, 5).to_string(), "09:05");
        assert_eq!(TimeValue::new(30, 90), TimeValue::new(23, 59));
    }

    #[test]
    fn picker_values_clamp_to_their_ranges() {
        let date_range = Some((DateValue::new(2026, 1, 10), DateValue::new(2026, 1, 20)));
        let time_range = Some((TimeValue::new(9, 0), TimeValue::new(17, 0)));

        assert_eq!(
            DatePickerValue::Date(DateValue::new(2025, 12, 31)).clamped(date_range, time_range),
            DatePickerValue::Date(DateValue::new(2026, 1, 10))
        );
        assert_eq!(
            DatePickerValue::Time(TimeValue::new(18, 30)).clamped(date_range, time_range),
            DatePickerValue::Time(TimeValue::new(17, 0))
        );
        assert_eq!(
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 1, 20),
                time: TimeValue::new(18, 30),
            }
            .clamped(date_range, time_range),
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 1, 20),
                time: TimeValue::new(17, 0),
            }
        );
        // A range whose bounds are reversed is ignored.
        assert_eq!(
            DatePickerValue::Time(TimeValue::new(12, 0))
                .clamped(None, Some((TimeValue::new(17, 0), TimeValue::new(9, 0)))),
            DatePickerValue::Time(TimeValue::new(12, 0))
        );
    }

    #[test]
    fn time_and_datetime_pickers_keep_their_ranges() {
        let element: Element = DatePicker::time(TimeValue::new(10, 30))
            .time_range(TimeValue::new(9, 0), TimeValue::new(17, 0))
            .on_change(|_| ())
            .into();
        let ElementKind::Widget(Widget::DatePicker(props)) = &element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(props.value, DatePickerValue::Time(TimeValue::new(10, 30)));
        assert_eq!(
            props.time_range,
            Some((TimeValue::new(9, 0), TimeValue::new(17, 0)))
        );

        let element: Element =
            DatePicker::date_time(DateValue::new(2026, 11, 12), TimeValue::new(10, 30))
                .range(DateValue::new(2026, 11, 1), DateValue::new(2026, 11, 30))
                .into();
        let ElementKind::Widget(Widget::DatePicker(props)) = &element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(
            props.value,
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 11, 12),
                time: TimeValue::new(10, 30),
            }
        );
        assert_eq!(
            props.date_range,
            Some((DateValue::new(2026, 11, 1), DateValue::new(2026, 11, 30)))
        );
    }

    #[test]
    fn tab_bar_keeps_tabs_selection_and_action() {
        let element: Element = TabBar::new(["Day", "Week", "Month"])
            .selected(2)
            .on_select(|_| ())
            .into();
        let ElementKind::Widget(Widget::TabBar(props)) = &element.kind else {
            panic!("expected tab bar");
        };
        assert_eq!(props.tabs, ["Day", "Week", "Month"]);
        assert_eq!(props.selected, 2);
        assert!(props.on_select.is_set());
    }

    #[test]
    fn tabs_compose_a_bar_above_their_content() {
        let element: Element = Tabs::new(["One", "Two"])
            .selected(1)
            .on_select(|_| ())
            .child(Text::new("content"))
            .into();

        assert_eq!(element.children.len(), 2);
        let ElementKind::Widget(Widget::TabBar(props)) = &element.children[0].kind else {
            panic!("expected tab bar");
        };
        assert_eq!(props.tabs, ["One", "Two"]);
        assert_eq!(props.selected, 1);
        assert!(props.on_select.is_set());
        assert!(matches!(
            element.children[1].kind,
            ElementKind::Widget(Widget::Text(_))
        ));
    }

    #[test]
    fn lists_keep_their_row_transition() {
        let list: Element = List::new(vec!["a", "b"])
            .key(|value| value.to_string())
            .row(|value| Text::new(*value).into())
            .recycled()
            .row_transition(180)
            .into();
        let ElementKind::List(props) = &list.kind else {
            panic!("expected list");
        };
        assert_eq!(props.row_transition_ms, 180);

        let plain: Element = List::new(vec!["a"])
            .key(|value| value.to_string())
            .row(|value| Text::new(*value).into())
            .into();
        let ElementKind::List(props) = &plain.kind else {
            panic!("expected list");
        };
        assert_eq!(props.row_transition_ms, 0);
    }

    #[test]
    fn disabled_decoration_is_dimmed_without_changing_rgb() {
        assert_eq!(dim_color(0xFF21_96F3, true), 0xFF21_96F3);
        assert_eq!(dim_color(0xFF21_96F3, false), 0x6621_96F3);
        // A translucent color keeps its own alpha as the base.
        assert_eq!(dim_color(0x8021_96F3, false), 0x3321_96F3);
    }

    #[test]
    fn style_builders_store_animations() {
        let element: Element = Button::new("Animate")
            .animate(320)
            .animation_curve(AnimationCurve::EaseOut)
            .into();
        assert_eq!(
            element.style.animation,
            Some(Animation::new(320).curve(AnimationCurve::EaseOut))
        );

        let element: Element = Text::new("Fade")
            .animation_curve(AnimationCurve::Linear)
            .into();
        assert_eq!(
            element.style.animation,
            Some(Animation::new(Animation::DEFAULT_DURATION_MS).curve(AnimationCurve::Linear))
        );
    }

    #[test]
    fn custom_curves_and_enter_exit_transitions_are_stored() {
        let element: Element = Button::new("Motion")
            .animation(Animation::new(400).curve(AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0)))
            .animate_enter(200)
            .enter_curve(AnimationCurve::spring(0.7, 0.35))
            .animate_exit(160)
            .exit_curve(AnimationCurve::EaseIn)
            .into();

        let animation = element.style.animation.as_ref().unwrap();
        assert_eq!(
            animation.curve,
            AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0)
        );
        let enter = element.style.enter.as_ref().unwrap();
        assert_eq!(enter.duration_ms, 200);
        assert_eq!(enter.curve, AnimationCurve::spring(0.7, 0.35));
        let exit = element.style.exit.as_ref().unwrap();
        assert_eq!(exit.duration_ms, 160);
        assert_eq!(exit.curve, AnimationCurve::EaseIn);
    }

    #[test]
    fn pager_and_tabs_view_keep_pages_and_selection() {
        let pager: Element = Pager::new()
            .selected(1)
            .on_select(|_| ())
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        let ElementKind::Pager(props) = &pager.kind else {
            panic!("expected pager");
        };
        assert_eq!(props.selected, 1);
        assert!(props.on_select.is_set());
        assert_eq!(pager.children.len(), 2);

        let tabs: Element = TabsView::new(["One", "Two"])
            .selected(1)
            .on_select(|_| ())
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        assert_eq!(tabs.children.len(), 2);
        let ElementKind::Widget(Widget::TabBar(bar)) = &tabs.children[0].kind else {
            panic!("expected tab bar");
        };
        assert_eq!(bar.selected, 1);
        let ElementKind::Pager(pager) = &tabs.children[1].kind else {
            panic!("expected pager");
        };
        assert_eq!(pager.selected, 1);
        assert_eq!(tabs.children[1].children.len(), 2);
    }

    #[test]
    fn icons_prefer_platform_names_over_the_shared_name() {
        let icon = Icon::new().shared("home").drawable("ic_home");
        assert_eq!(icon.android_name(), Some("ic_home"));
        assert_eq!(icon.ios_name(), Some("home"));

        let icon = Icon::new().shared("home").sf("house.fill");
        assert_eq!(icon.android_name(), Some("home"));
        assert_eq!(icon.ios_name(), Some("house.fill"));
    }

    #[test]
    fn date_picker_keeps_its_mode_and_action() {
        let element: Element = DatePicker::new(DateValue::new(2026, 11, 12))
            .mode(DatePickerMode::Inline)
            .on_change(|_| ())
            .into();

        let ElementKind::Widget(Widget::DatePicker(props)) = &element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(
            props.value,
            DatePickerValue::Date(DateValue::new(2026, 11, 12))
        );
        assert_eq!(props.mode, DatePickerMode::Inline);
        assert!(props.on_change.is_set());
    }
}
