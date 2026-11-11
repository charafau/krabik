import SwiftUI

@main
struct KrabikUiDslTodoMinimalApp: App {
    init() {
        // Registers this example's minimal todo app with the DSL runtime, so
        // the root view renders it instead of the built-in gallery.
        krabik_ui_dsl_todo_minimal_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
