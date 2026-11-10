import XCTest

/// Drives the checklist example through the real UIKit widgets built by Rust.
///
/// The test exercises the consumption pattern this example documents: a host
/// shell that installs a Rust app factory and renders whatever the app
/// describes — switches, dynamic rows, and in-place text updates.
final class ChecklistUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.staticTexts["Ship the app"].waitForExistence(timeout: 10))
    }

    override func tearDownWithError() throws {
        app = nil
    }

    func testChecklist() throws {
        // The summary reflects the initial state.
        XCTAssertTrue(app.staticTexts["1 of 3 tasks done · 0 notes"].exists)

        // Completing a task updates the summary in place.
        app.switches["Register the app factory"].tap()
        XCTAssertTrue(app.staticTexts["2 of 3 tasks done · 0 notes"].waitForExistence(timeout: 5))

        // Hiding completed tasks removes their native views.
        app.switches["Hide completed"].tap()
        waitForAbsence(app.staticTexts["✓ Review the renderer"])
        app.switches["Hide completed"].tap()
        XCTAssertTrue(app.staticTexts["✓ Review the renderer"].waitForExistence(timeout: 5))

        // Adding a note appends a native row and updates the summary.
        let note = app.textFields["Add a note"]
        XCTAssertTrue(note.waitForExistence(timeout: 5))
        note.tap()
        note.typeText("First note")
        app.buttons["Add"].tap()
        XCTAssertTrue(app.staticTexts["• First note"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["2 of 3 tasks done · 1 note"].waitForExistence(timeout: 5))

        // Clearing removes the rows again.
        app.buttons["Clear notes"].tap()
        waitForAbsence(app.staticTexts["• First note"])
        XCTAssertTrue(app.staticTexts["2 of 3 tasks done · 0 notes"].waitForExistence(timeout: 5))
    }

    private func waitForAbsence(_ element: XCUIElement) {
        let gone = expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: element)
        wait(for: [gone], timeout: 5)
    }
}
