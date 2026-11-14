//! The Android pieces of a session: the host and the Kotlin `KrabikHostController`.

use crate::android::HOST_CONTROLLER_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method};
use crate::android::host::AndroidState;
use crate::host::{Actions, Controller, Host, HostKind, Platform};
use crate::navigation::{Screen, ScreenTransition};
use crate::presentation::Dialog;
use crate::runtime::Runtime;
use crate::tree::NodeId;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
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
        jni_handles(env)?.controller.push.call_void(
            env,
            self.0.as_obj(),
            &[
                JValue::Long(host as jlong),
                JValue::Object(title.as_ref()),
                JValue::Int(screen.transition.code()),
            ],
        )?;
        Ok(())
    }

    fn pop(&self, env: &mut Env<'_>, count: usize, transition: ScreenTransition) -> JniResult<()> {
        jni_handles(env)?.controller.pop.call_void(
            env,
            self.0.as_obj(),
            &[JValue::Int(count as jint), JValue::Int(transition.code())],
        )?;
        Ok(())
    }

    fn set_root_visible(&self, env: &mut Env<'_>, visible: bool) -> JniResult<()> {
        jni_handles(env)?.controller.set_root_visible.call_void(
            env,
            self.0.as_obj(),
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
        jni_handles(env)?.controller.set_toolbar.call_void(
            env,
            self.0.as_obj(),
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
        jni_handles(env)?
            .controller
            .set_bottom_navigation
            .call_void(
                env,
                self.0.as_obj(),
                &[
                    JValue::Object(&labels),
                    JValue::Object(&icons),
                    JValue::Int(bar.map_or(0, |bar| bar.selected) as jint),
                ],
            )?;
        Ok(())
    }

    fn set_back_enabled(&self, env: &mut Env<'_>, enabled: bool) -> JniResult<()> {
        jni_handles(env)?.controller.set_back_enabled.call_void(
            env,
            self.0.as_obj(),
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
        jni_handles(env)?.controller.show_dialog.call_void(
            env,
            self.0.as_obj(),
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
        jni_handles(env)?
            .controller
            .dismiss_dialog
            .call_void(env, self.0.as_obj(), &[])?;
        Ok(())
    }

    fn present_sheet(&self, env: &mut Env<'_>, host: u64) -> JniResult<()> {
        jni_handles(env)?.controller.present_sheet.call_void(
            env,
            self.0.as_obj(),
            &[JValue::Long(host as jlong)],
        )?;
        Ok(())
    }

    fn dismiss_sheet(&self, env: &mut Env<'_>) -> JniResult<()> {
        jni_handles(env)?
            .controller
            .dismiss_sheet
            .call_void(env, self.0.as_obj(), &[])?;
        Ok(())
    }
}

/// The methods of the Kotlin `KrabikHostController`, resolved once; see
/// [`crate::android::handles`].
pub(crate) struct Handles {
    /// Keeps the class loaded so the method ids below stay valid.
    _class: Global<JClass<'static>>,
    pub(crate) push: Method,
    pub(crate) pop: Method,
    pub(crate) set_root_visible: Method,
    pub(crate) set_toolbar: Method,
    pub(crate) set_bottom_navigation: Method,
    pub(crate) set_back_enabled: Method,
    pub(crate) show_dialog: Method,
    pub(crate) dismiss_dialog: Method,
    pub(crate) present_sheet: Method,
    pub(crate) dismiss_sheet: Method,
}

impl Handles {
    pub(crate) fn load(env: &mut Env<'_>) -> JniResult<Self> {
        let class = class(env, HOST_CONTROLLER_CLASS)?;
        Ok(Self {
            push: method!(env, &class, "push", "(JLjava/lang/String;I)V", VOID),
            pop: method!(env, &class, "pop", "(II)V", VOID),
            set_root_visible: method!(env, &class, "setRootVisible", "(Z)V", VOID),
            set_toolbar: method!(
                env,
                &class,
                "setToolbar",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z[Ljava/lang/String;[Ljava/lang/String;)V",
                VOID
            ),
            set_bottom_navigation: method!(
                env,
                &class,
                "setBottomNavigation",
                "([Ljava/lang/String;[Ljava/lang/String;I)V",
                VOID
            ),
            set_back_enabled: method!(env, &class, "setBackEnabled", "(Z)V", VOID),
            show_dialog: method!(
                env,
                &class,
                "showDialog",
                "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;[IZ)V",
                VOID
            ),
            dismiss_dialog: method!(env, &class, "dismissDialog", "()V", VOID),
            present_sheet: method!(env, &class, "presentSheet", "(J)V", VOID),
            dismiss_sheet: method!(env, &class, "dismissSheet", "()V", VOID),
            _class: class,
        })
    }
}
