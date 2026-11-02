package com.example.krabik_ui_example;

import android.content.Context;
import android.graphics.Color;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.recyclerview.widget.RecyclerView;

/**
 * Minimal adapter used by the Rust RecyclerView and ViewPager2 demos.
 *
 * <p>Both AndroidX widgets draw through a {@link RecyclerView.Adapter}. Java cannot implement an
 * abstract class through JNI, so the application owns this small adapter and Rust passes it the
 * item strings through the {@code (Context, String[], boolean)} constructor.
 */
public final class TextRecyclerAdapter
        extends RecyclerView.Adapter<TextRecyclerAdapter.Holder> {

    private final String[] items;
    private final boolean fillParent;

    public TextRecyclerAdapter(Context context, String[] items, boolean fillParent) {
        this.items = items;
        this.fillParent = fillParent;
    }

    static final class Holder extends RecyclerView.ViewHolder {
        final TextView text;

        Holder(TextView text) {
            super(text);
            this.text = text;
        }
    }

    @Override
    public Holder onCreateViewHolder(ViewGroup parent, int viewType) {
        TextView text = new TextView(parent.getContext());
        text.setLayoutParams(
                new RecyclerView.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        fillParent
                                ? ViewGroup.LayoutParams.MATCH_PARENT
                                : ViewGroup.LayoutParams.WRAP_CONTENT));
        text.setMinHeight(dp(parent.getContext(), 56));
        text.setGravity(Gravity.CENTER_VERTICAL);
        text.setPadding(dp(parent.getContext(), 16), dp(parent.getContext(), 8),
                dp(parent.getContext(), 16), dp(parent.getContext(), 8));

        text.setTextSize(TypedValue.COMPLEX_UNIT_SP, 16);
        text.setTextColor(Color.rgb(17, 24, 39));
        return new Holder(text);
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
