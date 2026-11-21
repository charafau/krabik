import SwiftUI

@main
struct KrabikUiDslExampleApp: App {
    init() {
        // Registers this example's showcase app with the framework before any view
        // is created.
        krabik_ui_dsl_example_install()
        // The views of this application's own, before any screen asks for one.
        NativeViews.register()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
