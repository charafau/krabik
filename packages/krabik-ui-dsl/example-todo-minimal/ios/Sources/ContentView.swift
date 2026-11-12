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
        HostView()
    }
}
