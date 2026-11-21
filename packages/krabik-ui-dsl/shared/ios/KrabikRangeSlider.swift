import UIKit

/// The native control used for `RangeSlider` nodes: a track with two thumbs.
///
/// UIKit has no slider with two thumbs. This one works between 0 and 1;
/// Rust maps the two places to the values of the DSL slider and snaps them.
/// Rust creates it by its Objective-C name and listens for `.valueChanged`.
@objc(KrabikRangeSlider)
final class KrabikRangeSlider: UIControl {
    /// The place of the lower thumb along the track, from 0 to 1.
    @objc private(set) var low: Float = 0
    /// The place of the upper thumb along the track, from 0 to 1.
    @objc private(set) var high: Float = 1

    private let track = UIView()
    private let fill = UIView()
    private let lowThumb = UIView()
    private let highThumb = UIView()
    private var dragged: UIView?

    private static let thumbSize: CGFloat = 28
    private static let trackHeight: CGFloat = 4

    override init(frame: CGRect) {
        super.init(frame: frame)
        track.backgroundColor = .systemFill
        track.layer.cornerRadius = Self.trackHeight / 2
        fill.backgroundColor = tintColor
        fill.layer.cornerRadius = Self.trackHeight / 2
        for view in [track, fill, lowThumb, highThumb] {
            view.isUserInteractionEnabled = false
            addSubview(view)
        }
        for thumb in [lowThumb, highThumb] {
            thumb.backgroundColor = .white
            thumb.layer.cornerRadius = Self.thumbSize / 2
            thumb.layer.shadowColor = UIColor.black.cgColor
            thumb.layer.shadowOpacity = 0.25
            thumb.layer.shadowRadius = 3
            thumb.layer.shadowOffset = CGSize(width: 0, height: 1)
        }
        isAccessibilityElement = true
        accessibilityTraits = .adjustable
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not used")
    }

    /// Applies the accent color from Rust.
    @objc func configure(color: UIColor) {
        fill.backgroundColor = color
    }

    /// Applies the two places from Rust without reporting them back.
    @objc func setLow(_ low: Float, high: Float) {
        let first = min(max(low, 0), 1)
        let second = min(max(high, first), 1)
        guard first != self.low || second != self.high else { return }
        self.low = first
        self.high = second
        setNeedsLayout()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let usable = max(bounds.width - Self.thumbSize, 0)
        let middle = bounds.midY
        track.frame = CGRect(
            x: Self.thumbSize / 2, y: middle - Self.trackHeight / 2,
            width: usable, height: Self.trackHeight
        )
        let lowX = CGFloat(low) * usable
        let highX = CGFloat(high) * usable
        fill.frame = CGRect(
            x: Self.thumbSize / 2 + lowX, y: middle - Self.trackHeight / 2,
            width: highX - lowX, height: Self.trackHeight
        )
        let size = CGSize(width: Self.thumbSize, height: Self.thumbSize)
        lowThumb.frame = CGRect(origin: CGPoint(x: lowX, y: middle - Self.thumbSize / 2), size: size)
        highThumb.frame = CGRect(origin: CGPoint(x: highX, y: middle - Self.thumbSize / 2), size: size)
        accessibilityValue = "\(Int((low * 100).rounded())) to \(Int((high * 100).rounded())) percent"
    }

    private func place(at point: CGPoint) -> Float {
        let usable = max(bounds.width - Self.thumbSize, 1)
        return Float(min(max((point.x - Self.thumbSize / 2) / usable, 0), 1))
    }

    override func beginTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        let at = place(at: touch.location(in: self))
        // The nearer thumb moves. Where they lie on each other, the side of
        // the touch decides, so neither can be trapped under the other.
        let toLow = abs(at - low)
        let toHigh = abs(at - high)
        if toLow == toHigh {
            dragged = at < low ? lowThumb : highThumb
        } else {
            dragged = toLow < toHigh ? lowThumb : highThumb
        }
        return move(to: at)
    }

    override func continueTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        move(to: place(at: touch.location(in: self)))
    }

    override func endTracking(_ touch: UITouch?, with event: UIEvent?) {
        dragged = nil
    }

    override func cancelTracking(with event: UIEvent?) {
        dragged = nil
    }

    private func move(to place: Float) -> Bool {
        if dragged === lowThumb {
            low = min(place, high)
        } else {
            high = max(place, low)
        }
        setNeedsLayout()
        sendActions(for: .valueChanged)
        return true
    }
}
