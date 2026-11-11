import SwiftUI

@main
struct KrabikUiDslTodoApp: App {
    init() {
        // Registers this example's todo app with the DSL runtime, so the root
        // view renders it instead of the built-in gallery.
        krabik_ui_dsl_todo_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
