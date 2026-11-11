import XCTest

/// Drives the todo showcase through the real UIKit widgets built by Rust.
final class TodoUITests: XCTestCase {
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

    func testTodoFlow() throws {
        XCTAssertTrue(app.staticTexts["1 of 4 done"].exists)
        XCTAssertTrue(app.progressIndicators.firstMatch.exists)

        // Completing a row updates the summary and the progress indicator.
        app.buttons["Wire the native renderer"].tap()
        XCTAssertTrue(app.staticTexts["2 of 4 done"].waitForExistence(timeout: 5))

        // Submitting the text field adds a task and dismisses the keyboard.
        let field = app.textFields["What needs doing?"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.tap()
        field.typeText("Write docs\n")
        XCTAssertTrue(app.buttons["Write docs"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["2 of 5 done"].waitForExistence(timeout: 5))

        // Filters show only the matching rows.
        app.buttons["Active"].tap()
        XCTAssertTrue(app.buttons["Write docs"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Review the UI DSL"].exists)

        app.buttons["Done"].tap()
        XCTAssertTrue(app.buttons["Review the UI DSL"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Write docs"].exists)

        app.buttons["All"].tap()

        // The slider updates the priority used for new tasks.
        let slider = app.sliders.firstMatch
        XCTAssertTrue(slider.waitForExistence(timeout: 5))
        slider.adjust(toNormalizedSliderPosition: 1.0)
        XCTAssertTrue(app.staticTexts["Priority: 5"].waitForExistence(timeout: 5))

        field.tap()
        field.typeText("High priority\n")
        XCTAssertTrue(app.buttons["High priority"].waitForExistence(timeout: 5))

        // Clearing completed tasks removes their rows and updates the summary.
        app.buttons["Clear completed"].tap()
        XCTAssertTrue(app.staticTexts["0 of 4 done"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Review the UI DSL"].exists)
        XCTAssertFalse(app.buttons["Wire the native renderer"].exists)

        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "todo-showcase"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
