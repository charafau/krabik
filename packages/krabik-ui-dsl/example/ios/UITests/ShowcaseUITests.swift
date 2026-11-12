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
        XCTAssertTrue(app.staticTexts["Time: 10:30"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Bounded: 2026-06-15"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.datePickers.count, 5)
        backToCatalog()

        open("Tabs")
        XCTAssertTrue(
            app.staticTexts["Overview: the first tab is selected by default."]
                .waitForExistence(timeout: 5)
        )
        app.segmentedControls.firstMatch.buttons["Settings"].tap()
        XCTAssertTrue(
            app.staticTexts["Settings: tapping a tab reports Event::Selected."]
                .waitForExistence(timeout: 5)
        )
        backToCatalog()

        open("Frames & opacity")
        XCTAssertTrue(app.staticTexts["Panel is open."].waitForExistence(timeout: 5))
        tapButton("Hide panel")
        XCTAssertTrue(app.staticTexts["Panel is closed."].waitForExistence(timeout: 5))
        tapButton("Show panel")
        XCTAssertTrue(app.staticTexts["Panel is open."].waitForExistence(timeout: 5))
        backToCatalog()

        open("Decoration")
        XCTAssertTrue(app.staticTexts["Warm card"].waitForExistence(timeout: 5))
        tapButton("Cool palette")
        XCTAssertTrue(app.staticTexts["Cool card"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Enter & exit")
        XCTAssertTrue(app.staticTexts["2 cards"].waitForExistence(timeout: 5))
        tapButton("Add card")
        XCTAssertTrue(app.staticTexts["3 cards"].waitForExistence(timeout: 5))
        tapButton("Remove card")
        XCTAssertTrue(app.staticTexts["2 cards"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Curves & springs")
        XCTAssertTrue(app.staticTexts["Target: narrow"].waitForExistence(timeout: 5))
        tapButton("Run curves")
        XCTAssertTrue(app.staticTexts["Target: wide"].waitForExistence(timeout: 5))
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

    func testNativeScreensAndPresentations() throws {
        open("Text")
        XCTAssertTrue(app.staticTexts["Display 34 / bold"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Dialog")
        XCTAssertTrue(app.staticTexts["Delete task?"].waitForExistence(timeout: 5))
        app.buttons["Cancel"].tap()
        XCTAssertFalse(app.staticTexts["Delete task?"].exists)

        open("Bottom sheet")
        XCTAssertTrue(app.staticTexts["Filters"].waitForExistence(timeout: 5))
        app.buttons["Close"].tap()
        XCTAssertFalse(app.staticTexts["Filters"].exists)
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
        scrollToTop()
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 5))
    }

    private func scrollToTop() {
        for _ in 0..<8 {
            app.swipeDown()
        }
    }
}
