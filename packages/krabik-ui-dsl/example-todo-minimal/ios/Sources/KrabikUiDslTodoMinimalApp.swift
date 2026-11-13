import SwiftUI

@main
struct KrabikUiDslTodoMinimalApp: App {
    init() {
        // Registers this example's minimal todo app with the framework before any view
        // is created.
        krabik_ui_dsl_todo_minimal_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
