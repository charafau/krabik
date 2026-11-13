import SwiftUI

/// Hosts the native navigation controller whose screens are rendered by Rust.
private struct HostView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> KrabikHostController {
        KrabikHostController()
    }

    func updateUIViewController(_ controller: KrabikHostController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        // The host reaches the screen edges so the tab bar can cover the
        // home-indicator area; each screen keeps its content in the safe area.
        HostView()
            .ignoresSafeArea(.container)
    }
}
