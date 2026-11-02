package com.example.krabik_ui_example;

import android.app.Activity;
import android.view.View;

/**
 * Single JNI boundary between Android and the Rust UI implementation.
 *
 * <p>This class deliberately contains no UI logic. It only loads the Rust library, gives Rust the
 * {@link Activity} and a click-listener adapter, and forwards lifecycle events into native code.
 * Keeping the bridge self-contained makes it straightforward to move into an Android library
 * module later.
 */
public final class AndroidBridge implements View.OnClickListener {

    static {
        System.loadLibrary("krabik_ui_example");
    }

    private final Activity activity;

    public AndroidBridge(Activity activity) {
        this.activity = activity;
    }

    /** Hands the activity and listener to Rust, which creates and shows the view hierarchy. */
    public void createUi() {
        nativeCreateUi(activity, this);
    }

    /** Releases native state that would otherwise outlive this activity. */
    public void destroyUi() {
        nativeDestroyUi();
    }

    /**
     * Lets Rust handle the system back gesture.
     *
     * @return {@code true} when Rust returned to the main list, {@code false} when the activity
     *     should handle the back press normally.
     */
    public boolean back() {
        return nativeBack();
    }

    @Override
    public void onClick(View view) {
        nativeOnClick(view);
    }

    private native void nativeCreateUi(Activity activity, View.OnClickListener listener);

    private native void nativeOnClick(View view);

    private native boolean nativeBack();

    private native void nativeDestroyUi();
}
