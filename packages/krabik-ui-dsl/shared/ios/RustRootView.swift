import UIKit

/// One Rust-rendered host view.
///
/// The main host is created by `krabik_ios_create`; child hosts attach with a
/// session/host pair and are used by navigation screens and the sheet.
final class RustHostView: UIView {
    private var session: UInt64 = 0
    private var host: UInt64 = 0
    private var renderedSize: CGSize = .zero
    private var pendingRender = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .white
        clipsToBounds = true
    }

    /// Binds the main host created by the session.
    func bindMain(session: UInt64) {
        self.session = session
        self.host = 0
        renderedSize = .zero
        setNeedsLayout()
    }

    /// Binds a child host and attaches it to the session.
    ///
    /// The native controller creates hosts while Rust holds the session lock,
    /// so the attach itself is deferred to the next main-queue turn.
    func bind(session: UInt64, host: UInt64) {
        self.session = session
        self.host = host
        renderedSize = .zero
        DispatchQueue.main.async { [weak self] in
            guard let self, self.session == session, self.host == host else {
                return
            }
            krabik_ios_attach_host(session, host, Unmanaged.passUnretained(self).toOpaque())
            self.setNeedsLayout()
        }
    }

    /// Binds a child host, attaches it and renders it at its current size
    /// right away. Only valid when Rust is not in the middle of a render.
    func bindNow(session: UInt64, host: UInt64) {
        self.session = session
        self.host = host
        krabik_ios_attach_host(session, host, Unmanaged.passUnretained(self).toOpaque())
        renderedSize = bounds.size
        krabik_ios_render(session, host, Double(bounds.width), Double(bounds.height))
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layoutSubviews() {
        super.layoutSubviews()

        guard session != 0, bounds.width > 0, bounds.height > 0 else {
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
            guard let self, self.session != 0 else {
                return
            }
            self.pendingRender = false
            let size = self.bounds.size
            guard size.width > 0, size.height > 0, size != self.renderedSize else {
                return
            }
            self.renderedSize = size
            krabik_ios_render(
                self.session,
                self.host,
                Double(size.width),
                Double(size.height)
            )
        }
    }

    deinit {
        if session != 0 && host != 0 {
            krabik_ios_destroy_host(session, host)
        }
    }
}
