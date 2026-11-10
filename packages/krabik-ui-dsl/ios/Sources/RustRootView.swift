import UIKit

/// The only UIView the Rust UI owns.
///
/// Rust creates every native widget as a subview, gives it an exact frame, and
/// receives its events. No layout happens in Swift beyond forwarding the
/// viewport size.
final class RustRootView: UIView {
    private var handle: UInt64 = 0
    private var renderedSize: CGSize = .zero
    private var pendingRender = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .white
        clipsToBounds = true
        handle = krabik_ios_create(
            Unmanaged.passUnretained(self).toOpaque(),
            Double(frame.width),
            Double(frame.height)
        )
        renderedSize = frame.size
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layoutSubviews() {
        super.layoutSubviews()

        guard handle != 0, bounds.width > 0, bounds.height > 0 else {
            return
        }
        guard bounds.size != renderedSize, !pendingRender else {
            return
        }

        // UIKit can run layout synchronously from inside a Rust event handler
        // (for example when a focused field is removed from its superview and
        // resigns first responder). Rendering there would re-enter the state
        // mutex, so coalesce and defer to the next main-queue turn.
        pendingRender = true
        DispatchQueue.main.async { [weak self] in
            guard let self, self.handle != 0 else {
                return
            }
            self.pendingRender = false
            let size = self.bounds.size
            guard size.width > 0, size.height > 0, size != self.renderedSize else {
                return
            }
            self.renderedSize = size
            krabik_ios_render(self.handle, Double(size.width), Double(size.height))
        }
    }

    deinit {
        if handle != 0 {
            krabik_ios_destroy(handle)
        }
    }
}
