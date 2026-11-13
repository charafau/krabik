import SwiftUI

@main
struct KrabikUiDslApp: App {
    init() {
        // Registers the gallery with the framework before any view is created.
        krabik_ui_dsl_harness_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
