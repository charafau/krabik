import XCTest

/// Drives every gallery demo through the real UIKit widgets created by Rust.
///
/// The tests double as the iOS parity check: they exercise clicks, text
/// editing, submit/focus events, switch toggles, and the scroll containers.
final class GalleryUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.staticTexts["Native UI DSL"].waitForExistence(timeout: 10))
    }

    override func tearDownWithError() throws {
        app = nil
    }

    func testControls() throws {
        open("Controls")
        XCTAssertTrue(app.staticTexts["Notifications are off."].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Wi-Fi is on."].waitForExistence(timeout: 5))

        let wifi = app.buttons["Wi-Fi"]
        XCTAssertTrue(wifi.waitForExistence(timeout: 5))
        wifi.tap()
        XCTAssertTrue(app.staticTexts["Wi-Fi is off."].waitForExistence(timeout: 5))

        app.buttons["Large"].tap()
        XCTAssertTrue(app.staticTexts["Size: Large"].waitForExistence(timeout: 5))

        let slider = app.sliders["Volume"]
        XCTAssertTrue(slider.waitForExistence(timeout: 5))
        if !slider.isHittable {
            app.swipeUp()
        }
        slider.adjust(toNormalizedSliderPosition: 0.75)
        let volume = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Volume: 7"),
        ).firstMatch
        XCTAssertTrue(volume.waitForExistence(timeout: 5))
        XCTAssertTrue(app.progressIndicators.firstMatch.exists)

        let stepped = app.sliders["Stepped"]
        XCTAssertTrue(stepped.waitForExistence(timeout: 5))
        stepped.adjust(toNormalizedSliderPosition: 1.0)
        XCTAssertTrue(app.staticTexts["Stepped: 100"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.activityIndicators.firstMatch.exists)
        capture("16-controls")
    }

    func testGalleryDemos() throws {
        capture("01-gallery")

        open("Counter")
        XCTAssertTrue(app.staticTexts["Not tapped yet"].exists)
        app.buttons["Add one"].tap()
        XCTAssertTrue(app.staticTexts["Tapped once"].waitForExistence(timeout: 5))
        app.buttons["Add one"].tap()
        XCTAssertTrue(app.staticTexts["Tapped 2 times"].waitForExistence(timeout: 5))
        app.buttons["Reset"].tap()
        XCTAssertTrue(app.staticTexts["Not tapped yet"].waitForExistence(timeout: 5))
        capture("02-counter")
        back()

        open("Layouts")
        XCTAssertTrue(app.staticTexts["A spacer pushes its siblings to the edges"].exists)
        XCTAssertTrue(app.staticTexts["Start"].exists)
        XCTAssertTrue(app.staticTexts["End"].exists)
        capture("03-layouts")
        back()

        open("Typography")
        XCTAssertTrue(app.staticTexts["Display 34 / bold"].exists)
        XCTAssertTrue(app.staticTexts["Heading 24 / bold"].exists)
        XCTAssertTrue(app.staticTexts["Body 16 - The quick brown fox jumps over the lazy dog."].exists)
        capture("04-typography")
        back()

        open("Alignment")
        XCTAssertTrue(app.staticTexts["SpaceBetween"].exists)
        XCTAssertTrue(app.staticTexts["small"].exists)
        capture("05-alignment")
        back()

        open("Scroll")
        XCTAssertTrue(app.staticTexts["Row 1"].exists)
        XCTAssertTrue(app.staticTexts["Row 20"].exists)
        app.swipeUp()
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["Eight"].waitForExistence(timeout: 5))
        capture("06-scroll")
        back()

        open("Form")
        XCTAssertTrue(app.staticTexts["Type your name below."].exists)
        let name = app.textFields["Your name"]
        XCTAssertTrue(name.waitForExistence(timeout: 5))
        name.tap()
        name.typeText("Krabik")
        XCTAssertTrue(app.staticTexts["Hello, Krabik!"].waitForExistence(timeout: 5))
        let bio = app.textFields["Tell us about yourself"]
        XCTAssertTrue(bio.exists)
        bio.tap()
        XCTAssertTrue(app.staticTexts["The bio field is focused."].waitForExistence(timeout: 5))
        let email = app.textFields["you@example.com"]
        XCTAssertTrue(email.exists)
        email.tap()
        email.typeText("krabik@example.com")
        capture("07-form")
        app.buttons["Clear"].tap()
        XCTAssertTrue(app.staticTexts["Type your name below."].waitForExistence(timeout: 5))
        back()

        open("Media")
        XCTAssertTrue(app.images["Krabik app icon"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.images["Remote landscape photo"].exists)
        capture("08-media")
        back()

        open("Styling")
        XCTAssertTrue(app.staticTexts["Gradient card"].exists)
        XCTAssertTrue(app.buttons["Ripple"].exists)
        XCTAssertTrue(app.buttons["Disabled"].exists)
        capture("09-styling")
        back()

        open("Controls")
        XCTAssertTrue(app.staticTexts["Notifications are off."].exists)
        let toggle = app.switches["Notifications"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 5))
        toggle.tap()
        XCTAssertTrue(app.staticTexts["Notifications are on."].waitForExistence(timeout: 5))
        capture("10-controls")
        back()

        // The list materializes only the rows around the viewport; scrolling
        // must release the earlier ones and pull in later ones.
        if !app.buttons["List"].isHittable {
            app.swipeUp()
        }
        open("List")
        XCTAssertTrue(app.staticTexts["Row 1"].waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(rowNumbers().count, 40)
        for _ in 0..<8 {
            app.swipeUp()
        }
        waitForAbsence(app.staticTexts["Row 1"])
        let numbers = rowNumbers()
        XCTAssertFalse(numbers.contains(1), "row 1 stayed in the window")
        XCTAssertGreaterThan(numbers.max() ?? 0, 5, "scrolling did not advance the window")
        XCTAssertLessThanOrEqual(numbers.count, 40, "window grew to \(numbers.count)")
        capture("11-list")
        back()

        // Variable-height rows are measured on first sight, then reused.
        if !app.buttons["List (variable)"].isHittable {
            app.swipeUp()
        }
        open("List (variable)")
        let variableRow = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Row 1 -"),
        ).firstMatch
        XCTAssertTrue(variableRow.waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(rowNumbers().count, 40)
        for _ in 0..<6 {
            app.swipeUp()
        }
        let variableNumbers = rowNumbers()
        XCTAssertLessThanOrEqual(variableNumbers.count, 40)
        XCTAssertGreaterThan(variableNumbers.max() ?? 0, 5)
        capture("12-list-variable")
        back()

        // Recycled rows are built by a UICollectionView on demand and diffed
        // when the application state changes.
        if !app.buttons["List (recycled)"].isHittable {
            app.swipeUp()
        }
        open("List (recycled)")
        XCTAssertTrue(app.staticTexts["Row 1 · v0"].waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(rowNumbers().count, 40)
        app.buttons["Tap"].firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Taps: 1"].waitForExistence(timeout: 5))
        app.buttons["Bump rows"].tap()
        XCTAssertTrue(app.staticTexts["Row 1 · v1"].waitForExistence(timeout: 5))
        for _ in 0..<8 {
            app.swipeUp()
        }
        waitForAbsence(app.staticTexts["Row 1 · v1"])
        let recycledNumbers = rowNumbers()
        XCTAssertFalse(recycledNumbers.contains(1), "row 1 stayed in the recycled list")
        XCTAssertGreaterThan(recycledNumbers.max() ?? 0, 5, "recycled list did not scroll")
        XCTAssertLessThanOrEqual(recycledNumbers.count, 40, "recycled window grew")
        capture("13-list-recycled")
        back()

        // The same collection view sizes every holder from Rust's measurement.
        if !app.buttons["List (recycled, variable)"].isHittable {
            app.swipeUp()
        }
        open("List (recycled, variable)")
        let recycledVariableRow = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Row 1 -"),
        ).firstMatch
        XCTAssertTrue(recycledVariableRow.waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(rowNumbers().count, 40)
        for _ in 0..<5 {
            app.swipeUp()
        }
        let recycledVariableNumbers = rowNumbers()
        XCTAssertGreaterThan(recycledVariableNumbers.max() ?? 0, 5)
        XCTAssertLessThanOrEqual(recycledVariableNumbers.count, 40)
        capture("14-list-recycled-variable")
        back()

        // Focus survives recycling: the field returns focused with its text.
        if !app.buttons["List (recycled, form)"].isHittable {
            app.swipeUp()
        }
        open("List (recycled, form)")
        let note = app.textFields.firstMatch
        XCTAssertTrue(note.waitForExistence(timeout: 5))
        note.tap()
        note.typeText("hello")
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))

        // A revision-only rebind keeps the field focused.
        app.buttons["Refresh"].tap()
        XCTAssertTrue(app.textFields["hello"].waitForExistence(timeout: 5))

        // Scrolling the row out and back restores the focus and the text.
        for _ in 0..<6 {
            app.swipeUp()
        }
        for _ in 0..<6 {
            app.swipeDown()
        }
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))
        XCTAssertTrue(app.textFields["hello"].exists)
        capture("15-list-recycled-form")
        back()

        XCTAssertTrue(app.staticTexts["Native UI DSL"].waitForExistence(timeout: 5))
    }

    private func open(_ title: String) {
        let button = app.buttons[title]
        XCTAssertTrue(button.waitForExistence(timeout: 5), "missing gallery entry \(title)")
        button.tap()
        XCTAssertTrue(app.buttons["Back"].waitForExistence(timeout: 5), "\(title) did not open")
    }

    private func back() {
        app.buttons["Back"].tap()
        XCTAssertTrue(app.staticTexts["Native UI DSL"].waitForExistence(timeout: 5))
    }

    private func rowNumbers() -> [Int] {
        app.staticTexts
            .matching(NSPredicate(format: "label BEGINSWITH %@", "Row "))
            .allElementsBoundByIndex
            .compactMap { element -> Int? in
                let digits = element.label.dropFirst("Row ".count).prefix { $0.isNumber }
                return Int(digits)
            }
    }

    private func waitForAbsence(_ element: XCUIElement) {
        let expectation = XCTNSPredicateExpectation(
            predicate: NSPredicate(format: "exists == false"),
            object: element,
        )
        XCTAssertEqual(XCTWaiter().wait(for: [expectation], timeout: 5), .completed)
    }

    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
