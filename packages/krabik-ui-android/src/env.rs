//! Fallible JNI helpers.
//!
//! Every JNI call can fail in two ways: the `jni` crate returns an error, or the Java call left a
//! pending exception. [`EnvExt`] folds both checks into one result so call sites stay readable.

use jni::objects::{JClass, JObject, JString, JThrowable, JValue};
use jni::refs::Global;
use jni::signature::{FieldSignature, MethodSignature};
use jni::strings::JNIStr;
use jni::sys::{jint, jlong};
use jni::{Env, jni_sig, jni_str};

use super::constants::LOG_TAG;
use super::error::{Error, Result};

/// Convenience wrappers around [`Env`] that report failures as [`Error`].
pub trait EnvExt<'local> {
    /// Looks up a class by its JNI name.
    fn class(&mut self, name: &JNIStr) -> Result<JClass<'local>>;

    /// Looks up `class_name` and invokes the constructor matching `sig`.
    fn construct<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a method for its side effect.
    fn call_void<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<()>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a method that returns an object.
    fn call_object<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a method that returns an `int`.
    fn call_int<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jint>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Reads a static `int` field, such as a framework resource id.
    fn static_int(&mut self, class_name: &JNIStr, field: &JNIStr) -> Result<jint>;

    /// Calls a `static long` method.
    fn call_static_long<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jlong>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a `static int` method.
    fn call_static_int<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jint>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a `static` method for its side effect.
    fn call_static_void<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<()>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Calls a method that returns an object.
    fn call_static_object<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>;

    /// Reads a `float` field from `object`.
    fn field_float<'sig, S>(&mut self, object: &JObject<'_>, field: &JNIStr, sig: S) -> Result<f32>
    where
        S: AsRef<FieldSignature<'sig>>;

    /// Creates a Java string.
    fn java_string(&mut self, text: &str) -> Result<JString<'local>>;

    /// Promotes a local reference into a global one.
    fn global_ref(&mut self, object: &JObject<'_>) -> Result<Global<JObject<'static>>>;

    /// Checks whether `object` is an instance of `class_name`.
    fn instance_of(&mut self, object: &JObject<'_>, class_name: &JNIStr) -> Result<bool>;

    /// Fails with the pending Java exception, if any.
    fn check_exception(&mut self, context: &str) -> Result<()>;
}

impl<'local> EnvExt<'local> for Env<'local> {
    fn class(&mut self, name: &JNIStr) -> Result<JClass<'local>> {
        let context = format!("finding class {name}");
        let class = self
            .find_class(name)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        Ok(class)
    }

    fn construct<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = format!("constructing {class_name}");
        let class = self.class(class_name)?;
        let object = self
            .new_object(&class, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        Ok(object)
    }

    fn call_void<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<()>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = method.to_str();
        self.call_method(object, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)
    }

    fn call_object<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = method.to_str();
        let value = self
            .call_method(object, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        let result = value.l().map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        Ok(result)
    }

    fn call_int<'sig, 'args, S>(
        &mut self,
        object: &JObject<'_>,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jint>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = method.to_str();
        let value = self
            .call_method(object, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        let result = value.i().map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        Ok(result)
    }

    fn static_int(&mut self, class_name: &JNIStr, field: &JNIStr) -> Result<jint> {
        let context = format!("reading {class_name}.{field}");
        let class = self.class(class_name)?;
        let value = self
            .get_static_field(&class, field, jni_sig!("I"))
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        value.i().map_err(|error| Error::context(&context, error))
    }

    fn call_static_long<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jlong>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = format!("{class_name}.{}", method.to_str());
        let class = self.class(class_name)?;
        let value = self
            .call_static_method(&class, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        value.j().map_err(|error| Error::context(&context, error))
    }

    fn call_static_int<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<jint>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = format!("{class_name}.{}", method.to_str());
        let class = self.class(class_name)?;
        let value = self
            .call_static_method(&class, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        value.i().map_err(|error| Error::context(&context, error))
    }

    fn call_static_object<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = format!("{class_name}.{}", method.to_str());
        let class = self.class(class_name)?;
        let value = self
            .call_static_method(&class, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        let result = value.l().map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        Ok(result)
    }

    fn call_static_void<'sig, 'args, S>(
        &mut self,
        class_name: &JNIStr,
        method: &JNIStr,
        sig: S,
        args: &[JValue],
    ) -> Result<()>
    where
        S: AsRef<MethodSignature<'sig, 'args>>,
    {
        let context = format!("{class_name}.{}", method.to_str());
        let class = self.class(class_name)?;
        self.call_static_method(&class, method, sig, args)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)
    }

    fn field_float<'sig, S>(&mut self, object: &JObject<'_>, field: &JNIStr, sig: S) -> Result<f32>
    where
        S: AsRef<FieldSignature<'sig>>,
    {
        let context = format!("reading field {}", field.to_str());
        let value = self
            .get_field(object, field, sig)
            .map_err(|error| Error::context(&context, error))?;
        self.check_exception(&context)?;
        value.f().map_err(|error| Error::context(&context, error))
    }

    fn java_string(&mut self, text: &str) -> Result<JString<'local>> {
        let string = self
            .new_string(text)
            .map_err(|error| Error::context("creating Java string", error))?;
        self.check_exception("creating Java string")?;
        Ok(string)
    }

    fn global_ref(&mut self, object: &JObject<'_>) -> Result<Global<JObject<'static>>> {
        self.new_global_ref(object)
            .map_err(|error| Error::context("creating global reference", error))
    }

    fn instance_of(&mut self, object: &JObject<'_>, class_name: &JNIStr) -> Result<bool> {
        let class = self.class(class_name)?;
        self.is_instance_of(object, &class)
            .map_err(|error| Error::context(format!("checking {class_name}"), error))
    }

    fn check_exception(&mut self, context: &str) -> Result<()> {
        match take_pending_exception(self) {
            Some(description) => Err(Error::context(context, description)),
            None => Ok(()),
        }
    }
}

/// Returns and clears a pending Java exception, formatting it for Logcat when possible.
pub fn take_pending_exception(env: &mut Env<'_>) -> Option<String> {
    if !env.exception_check() {
        return None;
    }

    let throwable = env.exception_occurred();
    // Clear first: calling `toString()` while an exception is pending is not allowed.
    env.exception_clear();

    let description = throwable
        .as_ref()
        .and_then(|throwable| throwable_to_string(env, throwable))
        .unwrap_or_else(|| "Java exception (no details)".to_string());

    // Describing the exception must not leak a new pending exception.
    env.exception_clear();

    Some(description)
}

/// Drops a pending Java exception without reporting it.
pub fn discard_pending_exception(env: &mut Env<'_>) {
    env.exception_clear();
}

fn throwable_to_string(env: &mut Env<'_>, throwable: &JThrowable<'_>) -> Option<String> {
    let value = env
        .call_method(
            throwable,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .ok()?;
    let object = value.l().ok()?;
    // `toString` is known to return a `java.lang.String`, so this cast is sound.
    Some(unsafe { JString::from_raw(env, object.into_raw()) }.to_string())
}

/// Writes an error message to Logcat through `android.util.Log`.
///
/// This intentionally uses JNI instead of adding a logging dependency, keeping the app's
/// dependency set to the `jni` crate only.
pub fn log_error(env: &mut Env<'_>, message: &str) {
    let result = (|| -> jni::errors::Result<()> {
        let log_class = env.find_class(jni_str!("android/util/Log"))?;
        let tag = env.new_string(LOG_TAG)?;
        let message = env.new_string(message)?;

        env.call_static_method(
            log_class,
            jni_str!("println"),
            jni_sig!("(ILjava/lang/String;Ljava/lang/String;)I"),
            &[
                // android.util.Log.ERROR
                JValue::Int(6),
                JValue::Object(tag.as_ref()),
                JValue::Object(message.as_ref()),
            ],
        )?;
        Ok(())
    })();

    // Logcat is best-effort: never turn a diagnostic failure into a second failure.
    if result.is_err() {
        env.exception_clear();
    }
}
