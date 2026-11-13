import SwiftUI

@main
struct KrabikUiDslExampleApp: App {
    init() {
        // Registers this example's showcase app with the framework before any view
        // is created.
        krabik_ui_dsl_example_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
