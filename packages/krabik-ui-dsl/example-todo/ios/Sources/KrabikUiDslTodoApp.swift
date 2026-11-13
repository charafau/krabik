import SwiftUI

@main
struct KrabikUiDslTodoApp: App {
    init() {
        // Registers this example's todo app with the framework before any view
        // is created.
        krabik_ui_dsl_todo_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
