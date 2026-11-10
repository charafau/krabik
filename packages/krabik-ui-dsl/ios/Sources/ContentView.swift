import SwiftUI

/// Hosts the UIKit view whose widgets are created by the Rust renderer.
private struct RustView: UIViewRepresentable {
    func makeUIView(context: Context) -> RustRootView {
        RustRootView()
    }

    func updateUIView(_ uiView: RustRootView, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        RustView()
    }
}
