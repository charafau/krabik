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

    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
