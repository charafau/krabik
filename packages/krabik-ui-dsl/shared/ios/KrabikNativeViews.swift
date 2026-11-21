import UIKit
import WebKit

/// A view of the application's own inside a Rust-described screen.
///
/// Rust lays `view` out and calls `update` with the properties the
/// description gives, at first and whenever they change.
protocol KrabikNativeView: AnyObject {
    var view: UIView { get }
    func update(_ props: [String: String])
}

extension KrabikNativeView {
    func update(_ props: [String: String]) {}
}

/// The views an application supplies for `NativeView` nodes, by name.
///
/// Register a factory before the first screen is shown, for example in the
/// `init` of the `App`:
///
/// ```swift
/// KrabikNativeViews.register("rating") { emit in RatingView(emit: emit) }
/// ```
///
/// `emit` sends a string to the application's `on_event`.
@objc(KrabikNativeViews)
final class KrabikNativeViews: NSObject {
    typealias Factory = (_ emit: @escaping (String) -> Void) -> KrabikNativeView

    /// The name the framework's own web view is registered under.
    static let webView = "krabik.webview"

    private static var factories: [String: Factory] = [
        webView: { emit in KrabikWebView(emit: emit) },
    ]
    private static var missing: Set<String> = []

    /// Makes `factory` the source of the views named `name`.
    static func register(_ name: String, _ factory: @escaping Factory) {
        factories[name] = factory
    }

    /// Creates the host view of one `NativeView` node. Called by Rust.
    @objc static func makeHost(_ emit: @escaping (String) -> Void) -> UIView {
        Host(emit: emit)
    }

    /// Gives a host its name and properties. Called by Rust.
    @objc static func update(_ host: UIView, name: String, keys: [String], values: [String]) {
        var props: [String: String] = [:]
        for (key, value) in zip(keys, values) {
            props[key] = value
        }
        (host as? Host)?.update(name: name, props: props)
    }

    /// The frame Rust positions; the application's view fills it.
    private final class Host: UIView {
        private let emit: (String) -> Void
        private var name: String?
        private var content: KrabikNativeView?

        init(emit: @escaping (String) -> Void) {
            self.emit = emit
            super.init(frame: .zero)
            clipsToBounds = true
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) {
            fatalError("init(coder:) is not used")
        }

        func update(name: String, props: [String: String]) {
            if name != self.name {
                self.name = name
                content?.view.removeFromSuperview()
                content = KrabikNativeViews.factories[name]?(emit)
                if let view = content?.view {
                    view.frame = bounds
                    view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
                    addSubview(view)
                } else if KrabikNativeViews.missing.insert(name).inserted {
                    NSLog("krabik: no native view is registered as \"%@\"", name)
                }
            }
            content?.update(props)
        }
    }
}

/// The framework's own native view: the platform's web view.
private final class KrabikWebView: NSObject, KrabikNativeView, WKNavigationDelegate {
    private let web = WKWebView(frame: .zero)
    private let emit: (String) -> Void
    private var shown: String?

    var view: UIView { web }

    init(emit: @escaping (String) -> Void) {
        self.emit = emit
        super.init()
        web.navigationDelegate = self
    }

    func update(_ props: [String: String]) {
        web.configuration.defaultWebpagePreferences.allowsContentJavaScript =
            props["javascript"] != "false"
        let address = props["url"] ?? ""
        // Only an address the application newly asks for is loaded, so the
        // pages the user reached by following links are left alone.
        guard address != shown else { return }
        shown = address
        if let url = URL(string: address), !address.isEmpty {
            web.load(URLRequest(url: url))
        } else {
            web.loadHTMLString("", baseURL: nil)
        }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        if let address = webView.url?.absoluteString {
            emit(address)
        }
    }
}
