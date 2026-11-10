import SwiftUI

@main
struct KrabikUiDslExampleApp: App {
    init() {
        // Registers this example's checklist app with the DSL runtime, so the
        // root view renders it instead of the built-in gallery.
        krabik_ui_dsl_example_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
