package com.example.krabik_ui_example;

import android.view.View;
import android.view.ViewTreeObserver;

/**
 * Restores a {@link View}'s scroll offset after it has been attached and laid out.
 *
 * <p>Calling {@link View#scrollTo(int, int)} immediately after {@code setContentView} is not
 * enough: a newly created {@code ScrollView} has not measured its child yet, so the requested
 * offset is clamped to zero. This helper waits for the first layout pass and then applies the
 * offset.
 */
public final class ScrollRestorer {

    private ScrollRestorer() {}

    /** Scrolls {@code view} to {@code y} once it has a measured size. */
    public static void restore(View view, int y) {
        view.post(() -> {
            if (view.isLaidOut()) {
                view.scrollTo(0, y);
                return;
            }

            view.getViewTreeObserver()
                    .addOnGlobalLayoutListener(
                            new ViewTreeObserver.OnGlobalLayoutListener() {
                                @Override
                                public void onGlobalLayout() {
                                    view.getViewTreeObserver()
                                            .removeOnGlobalLayoutListener(this);
                                    view.scrollTo(0, y);
                                }
                            });
        });
    }
}
