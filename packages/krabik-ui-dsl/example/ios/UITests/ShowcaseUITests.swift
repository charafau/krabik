import XCTest

/// Drives the one-page-per-widget catalog through the native widgets.
final class ShowcaseUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 10))
    }

    override func tearDownWithError() throws {
        app = nil
    }

    func testNewWidgets() throws {
        open("Radio group")
        XCTAssertTrue(app.staticTexts["Selected: Medium"].waitForExistence(timeout: 5))
        tapButton("Large")
        XCTAssertTrue(app.staticTexts["Selected: Large"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Slider")
        XCTAssertTrue(app.staticTexts["Stepped: 30"].waitForExistence(timeout: 5))
        let stepped = app.sliders["Stepped volume"]
        XCTAssertTrue(stepped.waitForExistence(timeout: 5))
        stepped.adjust(toNormalizedSliderPosition: 1.0)
        XCTAssertTrue(app.staticTexts["Stepped: 100"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Date picker")
        XCTAssertTrue(app.staticTexts["Compact: 2026-11-12"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.datePickers.count, 2)
        backToCatalog()

        open("Progress")
        tapButton("Set 100%")
        XCTAssertTrue(app.staticTexts["100%"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Spinner")
        XCTAssertTrue(app.activityIndicators["Loading spinner"].exists)
        backToCatalog()

        open("Image")
        XCTAssertTrue(app.images["Krabik app icon"].waitForExistence(timeout: 10))
        backToCatalog()

        open("Recycled list")
        XCTAssertTrue(app.staticTexts["Row taps: 0"].waitForExistence(timeout: 5))
        tapButton("Tap Recycled row 1")
        XCTAssertTrue(app.staticTexts["Row taps: 1"].waitForExistence(timeout: 5))
        backToCatalog()
    }

    func testWindowedListExists() throws {
        open("Windowed list")
        XCTAssertTrue(app.staticTexts["Windowed row 1"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Windowed row 200"].exists)
        backToCatalog()
    }

    /// Scrolls the catalog until the page button is hittable, then opens it.
    private func open(_ label: String) {
        scrollToTop()
        let button = app.buttons[label]
        var attempts = 0
        while !button.isHittable && attempts < 8 {
            app.swipeUp()
            attempts += 1
        }
        XCTAssertTrue(button.waitForExistence(timeout: 5), "missing page button: \(label)")
        button.tap()
    }

    private func tapButton(_ label: String) {
        let button = app.buttons[label]
        if !button.isHittable {
            app.swipeUp()
        }
        XCTAssertTrue(button.waitForExistence(timeout: 5), "missing button: \(label)")
        button.tap()
    }

    private func backToCatalog() {
        app.buttons["Back"].tap()
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 5))
    }

    private func scrollToTop() {
        for _ in 0..<8 {
            app.swipeDown()
        }
    }
}
