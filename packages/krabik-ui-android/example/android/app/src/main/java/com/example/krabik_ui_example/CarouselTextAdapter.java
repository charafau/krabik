package com.example.krabik_ui_example;

import android.content.Context;
import android.graphics.Color;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.TextView;

import androidx.recyclerview.widget.RecyclerView;

import com.google.android.material.carousel.MaskableFrameLayout;

/**
 * Minimal adapter used by the Rust Material carousel demo.
 *
 * <p>CarouselLayoutManager requires every item's root view to be a {@link MaskableFrameLayout},
 * so this adapter wraps each text item in one. Java owns the class because RecyclerView.Adapter is
 * an abstract class that cannot be implemented through JNI.
 */
public final class CarouselTextAdapter
        extends RecyclerView.Adapter<CarouselTextAdapter.Holder> {

    private final String[] items;

    public CarouselTextAdapter(Context context, String[] items) {
        this.items = items;
    }

    static final class Holder extends RecyclerView.ViewHolder {
        final TextView text;

        Holder(MaskableFrameLayout root, TextView text) {
            super(root);
            this.text = text;
        }
    }

    @Override
    public Holder onCreateViewHolder(ViewGroup parent, int viewType) {
        Context context = parent.getContext();

        MaskableFrameLayout root = new MaskableFrameLayout(context);
        root.setLayoutParams(
                new RecyclerView.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT));

        TextView text = new TextView(context);
        text.setLayoutParams(
                new FrameLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT));
        text.setGravity(Gravity.CENTER);
        text.setPadding(dp(context, 16), dp(context, 8), dp(context, 16), dp(context, 8));
        text.setTextSize(TypedValue.COMPLEX_UNIT_SP, 16);
        text.setTextColor(Color.rgb(17, 24, 39));

        root.addView(text);
        return new Holder(root, text);
    }

    @Override
    public void onBindViewHolder(Holder holder, int position) {
        holder.text.setText(items[position]);
    }

    @Override
    public int getItemCount() {
        return items.length;
    }

    private static int dp(Context context, float value) {
        return Math.round(value * context.getResources().getDisplayMetrics().density);
    }
}
