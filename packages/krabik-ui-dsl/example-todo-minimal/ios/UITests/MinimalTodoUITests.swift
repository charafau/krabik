import XCTest

/// Drives the minimal todo app through the real UIKit widgets built by Rust.
final class MinimalTodoUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.staticTexts["Todo"].waitForExistence(timeout: 10))
    }

    override func tearDownWithError() throws {
        app = nil
    }

    func testMinimalTodoFlow() throws {
        XCTAssertTrue(app.staticTexts["1 of 4 done"].exists)

        app.buttons["Wire the native renderer"].tap()
        XCTAssertTrue(app.staticTexts["2 of 4 done"].waitForExistence(timeout: 5))

        let field = app.textFields["What needs doing?"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.tap()
        field.typeText("Write docs\n")
        XCTAssertTrue(app.buttons["Write docs"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["2 of 5 done"].waitForExistence(timeout: 5))

        app.buttons["Clear completed"].tap()
        XCTAssertTrue(app.staticTexts["0 of 3 done"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Review the UI DSL"].exists)
        XCTAssertFalse(app.buttons["Wire the native renderer"].exists)

        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "todo-minimal"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
