//! The Android pieces of a session: the host and the Kotlin `KrabikHostController`.

use crate::android::host::AndroidState;
use crate::host::{Actions, Controller, Host, HostKind, Platform};
use crate::navigation::{Screen, ScreenTransition};
use crate::presentation::Dialog;
use crate::runtime::Runtime;
use crate::tree::NodeId;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::refs::Global;
use jni::sys::{jint, jlong};
use jni::{Env, JValue, jni_sig, jni_str};

/// The Android pieces of a [`Session`].
pub(crate) struct Android;

impl Platform for Android {
    type Cx<'a> = Env<'a>;
    type Error = jni::errors::Error;
    type Host = AndroidState;
    type Controller = AndroidController;
}

impl Host<Android> for AndroidState {
    fn kind(&self) -> &HostKind {
        &self.kind
    }

    fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    fn row_actions(&self, id: NodeId) -> Option<&Actions> {
        self.row_actions.get(&id)
    }

    fn row_of(&self, id: NodeId) -> Option<(NodeId, usize)> {
        self.row_nodes.get(&id).copied()
    }

    fn render(&mut self, env: &mut Env<'_>, element: &crate::ui::RawElement) -> JniResult<()> {
        AndroidState::render(self, env, element)
    }
}

/// The Kotlin `KrabikHostController`, which owns the screen stack, toolbar,
/// bottom navigation, alert and sheet.
pub(crate) struct AndroidController(pub(crate) Global<JObject<'static>>);

impl Controller<Android> for AndroidController {
    fn push(&self, env: &mut Env<'_>, host: u64, screen: &Screen) -> JniResult<()> {
        let title = env.new_string(&screen.title)?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("push"),
            jni_sig!("(JLjava/lang/String;I)V"),
            &[
                JValue::Long(host as jlong),
                JValue::Object(title.as_ref()),
                JValue::Int(screen.transition.code()),
            ],
        )?;
        Ok(())
    }

    fn pop(&self, env: &mut Env<'_>, count: usize, transition: ScreenTransition) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("pop"),
            jni_sig!("(II)V"),
            &[JValue::Int(count as jint), JValue::Int(transition.code())],
        )?;
        Ok(())
    }

    fn set_root_visible(&self, env: &mut Env<'_>, visible: bool) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("setRootVisible"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(visible)],
        )?;
        Ok(())
    }

    fn set_chrome(&self, env: &mut Env<'_>, screen: Option<&Screen>) -> JniResult<()> {
        let toolbar = screen.and_then(|screen| screen.toolbar.as_ref());
        let actions = toolbar.map_or(&[][..], |toolbar| toolbar.actions.as_slice());
        // A screen shows the toolbar unless it explicitly hides it.
        let hidden = screen.is_none() || toolbar.is_some_and(|toolbar| toolbar.hidden);

        let action_labels = string_array!(env, actions.iter().map(|a| a.label.as_str()));
        let action_icons = string_array!(
            env,
            actions.iter().map(|a| a.icon.android_name().unwrap_or(""))
        );
        let title = jcall!(
            env,
            env.new_string(screen.map_or("", |screen| screen.title.as_str()))
        )?;
        let subtitle = jcall!(
            env,
            env.new_string(toolbar.and_then(|t| t.subtitle.as_deref()).unwrap_or(""))
        )?;
        let back_title = jcall!(
            env,
            env.new_string(
                toolbar
                    .and_then(|t| t.back_title.as_deref())
                    .unwrap_or("Back")
            )
        )?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("setToolbar"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z[Ljava/lang/String;[Ljava/lang/String;)V"),
            &[
                JValue::Object(title.as_ref()),
                JValue::Object(subtitle.as_ref()),
                JValue::Object(back_title.as_ref()),
                JValue::Bool(hidden),
                JValue::Object(&action_labels),
                JValue::Object(&action_icons),
            ],
        )?;

        // No items hides the bar.
        let bar = screen.and_then(|screen| screen.bottom_navigation.as_ref());
        let items = bar.map_or(&[][..], |bar| bar.items.as_slice());
        let labels = string_array!(env, items.iter().map(|item| item.label.as_str()));
        let icons = string_array!(
            env,
            items
                .iter()
                .map(|item| item.icon.android_name().unwrap_or(""))
        );
        env.call_method(
            self.0.as_obj(),
            jni_str!("setBottomNavigation"),
            jni_sig!("([Ljava/lang/String;[Ljava/lang/String;I)V"),
            &[
                JValue::Object(&labels),
                JValue::Object(&icons),
                JValue::Int(bar.map_or(0, |bar| bar.selected) as jint),
            ],
        )?;
        Ok(())
    }

    fn set_back_enabled(&self, env: &mut Env<'_>, enabled: bool) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("setBackEnabled"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(enabled)],
        )?;
        Ok(())
    }

    fn show_dialog(&self, env: &mut Env<'_>, dialog: &Dialog) -> JniResult<()> {
        let title = jcall!(env, env.new_string(&dialog.title))?;
        let message = jcall!(env, env.new_string(dialog.message.as_deref().unwrap_or("")))?;
        let labels = string_array!(env, dialog.buttons.iter().map(|b| b.label.as_str()));
        let role_codes: Vec<jint> = dialog.buttons.iter().map(|b| b.role.code()).collect();
        let roles = env.new_int_array(role_codes.len())?;
        roles.set_region(env, 0, &role_codes)?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("showDialog"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;[IZ)V"),
            &[
                JValue::Object(title.as_ref()),
                JValue::Object(message.as_ref()),
                JValue::Object(&labels),
                JValue::Object(&roles),
                JValue::Bool(dialog.is_cancelable()),
            ],
        )?;
        Ok(())
    }

    fn dismiss_dialog(&self, env: &mut Env<'_>) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("dismissDialog"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    }

    fn present_sheet(&self, env: &mut Env<'_>, host: u64) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("presentSheet"),
            jni_sig!("(J)V"),
            &[JValue::Long(host as jlong)],
        )?;
        Ok(())
    }

    fn dismiss_sheet(&self, env: &mut Env<'_>) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("dismissSheet"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    }
}
