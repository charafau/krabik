//! Hot reload for development: new application code in a running app.
//!
//! An application is a library, and the runtime reaches it through one
//! trait object. So new code can be had without restarting:
//!
//! 1. the app's library is built again and loaded next to the old one;
//! 2. the new library *adopts* the application value: it takes the same
//!    memory as a value of its own type and returns a trait object whose
//!    methods are the new code;
//! 3. the session continues with that object and renders.
//!
//! The state survives because it is never copied or rebuilt. That is also
//! the limit: it only works while the layout of the application's state is
//! what the running code believes it is. A library whose state type has
//! another size, alignment or name is refused, and the app keeps running
//! the code it has.
//!
//! The new library carries its own copy of this framework, which is used
//! only to build descriptions and handlers; its sessions and platform
//! entry points are never touched. The old library stays loaded, because
//! handlers and commands made by old code may still run.
//!
//! An application opts in with [`hot_reload!`](crate::hot_reload).

use std::ffi::{CStr, CString, c_char, c_int, c_void};

use crate::app::{AnyApp, App};

/// What must be equal between the running application and a library that
/// wants to take it over.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    size: usize,
    align: usize,
    state: u64,
    message: u64,
}

impl Fingerprint {
    /// The fingerprint of an application type.
    pub fn of<A: App>() -> Self {
        Self {
            size: size_of::<A>(),
            align: align_of::<A>(),
            state: name_hash(std::any::type_name::<A>()),
            message: name_hash(std::any::type_name::<A::Msg>()),
        }
    }
}

/// FNV-1a: the same in every build, which a randomly seeded hasher is not.
fn name_hash(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Takes over the application value at `state` as an `A` of this library.
///
/// Returns a thin pointer to a boxed trait object, because a trait object
/// itself cannot cross a C function. Called by the function
/// [`hot_reload!`](crate::hot_reload) exports.
///
/// # Safety
///
/// `state` must have come from `Box::into_raw` of a value whose type has
/// the fingerprint of `A`, and must not be used again by the caller.
pub unsafe fn adopt<A: App>(state: *mut ()) -> *mut () {
    // SAFETY: the caller hands over a boxed value with the layout of `A`.
    let app: Box<dyn AnyApp> = unsafe { Box::from_raw(state.cast::<A>()) };
    Box::into_raw(Box::new(app)).cast()
}

/// Lets a library take over the running application; see [`crate::hot`].
///
/// Invoke it once, at the top level of the app's crate, with the type that
/// implements [`App`]:
///
/// ```ignore
/// krabik_ui_dsl::hot_reload!(Notes);
/// ```
#[macro_export]
macro_rules! hot_reload {
    ($app:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn krabik_hot_fingerprint() -> $crate::hot::Fingerprint {
            $crate::hot::Fingerprint::of::<$app>()
        }

        /// # Safety
        ///
        /// See `krabik_ui_dsl::hot::adopt`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn krabik_hot_adopt(state: *mut ()) -> *mut () {
            // SAFETY: the caller checked this library's fingerprint.
            unsafe { $crate::hot::adopt::<$app>(state) }
        }
    };
}

/// A loaded library that can take over an application.
pub(crate) struct Library {
    #[cfg_attr(not(test), allow(dead_code))]
    handle: *mut c_void,
    fingerprint: Fingerprint,
    adopt: unsafe extern "C" fn(*mut ()) -> *mut (),
}

impl Library {
    /// Loads the library at `path`. It is never unloaded.
    pub(crate) fn open(path: &str) -> Result<Self, String> {
        let name = CString::new(path).map_err(|_| "the path contains a NUL byte".to_owned())?;
        // SAFETY: `name` is a C string; the library's initializers are the
        // Rust runtime's own.
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        if handle.is_null() {
            return Err(format!("could not load {path}: {}", last_error()));
        }
        // SAFETY: the two symbols are the functions `hot_reload!` exports,
        // with exactly these signatures.
        unsafe {
            let fingerprint = symbol(handle, c"krabik_hot_fingerprint", path)?;
            let adopt = symbol(handle, c"krabik_hot_adopt", path)?;
            let fingerprint: extern "C" fn() -> Fingerprint = std::mem::transmute(fingerprint);
            Ok(Self {
                handle,
                fingerprint: fingerprint(),
                adopt: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(*mut ()) -> *mut ()>(
                    adopt,
                ),
            })
        }
    }

    /// Hands `app` to the library and returns it running the library's
    /// code, or returns it unchanged with the reason the library cannot
    /// take it.
    pub(crate) fn adopt(
        &self,
        app: Box<dyn AnyApp>,
    ) -> Result<Box<dyn AnyApp>, (Box<dyn AnyApp>, String)> {
        let running = app.fingerprint();
        if running != self.fingerprint {
            let reason = format!(
                "the application's state or message type changed \
                 (running {running:?}, new {:?}); restart the app",
                self.fingerprint
            );
            return Err((app, reason));
        }
        // SAFETY: the fingerprints are equal, so the library reads the
        // value as the type it was written as. `adopt` returns a boxed
        // `Box<dyn AnyApp>`, as `hot::adopt` documents.
        unsafe {
            let adopted = (self.adopt)(app.into_raw());
            Ok(*Box::from_raw(adopted.cast::<Box<dyn AnyApp>>()))
        }
    }
}

#[cfg(test)]
impl Library {
    /// An application made by the library's own `symbol` function, which
    /// returns the raw pointer of a boxed application value.
    pub(crate) fn create(&self, name: &CStr) -> Box<dyn AnyApp> {
        // SAFETY: test fixtures export `extern "C" fn() -> *mut ()` under
        // this name, returning a value of the library's application type.
        unsafe {
            let create = symbol(self.handle, name, "the fixture").expect("the constructor");
            let create: extern "C" fn() -> *mut () = std::mem::transmute(create);
            let adopted = (self.adopt)(create());
            *Box::from_raw(adopted.cast::<Box<dyn AnyApp>>())
        }
    }
}

/// Looks a function up in one library.
unsafe fn symbol(handle: *mut c_void, name: &CStr, path: &str) -> Result<*mut c_void, String> {
    // SAFETY: `handle` is a live library and `name` a C string.
    let found = unsafe { dlsym(handle, name.as_ptr()) };
    if found.is_null() {
        Err(format!(
            "{path} does not export {}; does the app invoke `hot_reload!`?",
            name.to_string_lossy()
        ))
    } else {
        Ok(found)
    }
}

fn last_error() -> String {
    // SAFETY: `dlerror` returns null or a C string owned by the loader.
    unsafe {
        let message = dlerror();
        if message.is_null() {
            "unknown error".to_owned()
        } else {
            CStr::from_ptr(message).to_string_lossy().into_owned()
        }
    }
}

const RTLD_NOW: c_int = 2;
/// The library's symbols are not offered to libraries loaded later.
#[cfg(any(target_os = "macos", target_os = "ios"))]
const RTLD_LOCAL: c_int = 4;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
const RTLD_LOCAL: c_int = 0;

unsafe extern "C" {
    fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}
