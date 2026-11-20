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

    /// Rust paints with fixed colours, so it has to be told when the
    /// appearance changes. The screen and its state must stay as they were.
    func testSwitchingToDarkKeepsTheScreenAndItsState() throws {
        XCUIDevice.shared.appearance = .light
        defer { XCUIDevice.shared.appearance = .light }

        open("Radio group")
        tapButton("Large")
        XCTAssertTrue(app.staticTexts["Selected: Large"].waitForExistence(timeout: 5))
        let light = brightness(of: app.screenshot())

        XCUIDevice.shared.appearance = .dark
        XCTAssertTrue(app.staticTexts["Selected: Large"].waitForExistence(timeout: 5))
        sleep(1)
        let dark = brightness(of: app.screenshot())
        XCTAssertGreaterThan(light, 0.7, "the light screen is mostly light")
        XCTAssertLessThan(dark, 0.3, "the dark screen is mostly dark")

        backToCatalog()
    }

    /// The average brightness of a screenshot, from 0 (black) to 1 (white).
    private func brightness(of screenshot: XCUIScreenshot) -> Double {
        guard let image = screenshot.image.cgImage else { return 0 }
        var pixel = [UInt8](repeating: 0, count: 4)
        let context = CGContext(
            data: &pixel,
            width: 1,
            height: 1,
            bitsPerComponent: 8,
            bytesPerRow: 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )
        // Drawing the whole image into one pixel averages it.
        context?.interpolationQuality = .medium
        context?.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        return (Double(pixel[0]) + Double(pixel[1]) + Double(pixel[2])) / (3 * 255)
    }

    /// The swipe-back gesture is UIKit's own: it removes the screen before
    /// Rust hears about it. The application's stack has to follow, and a
    /// gesture the user abandons has to change nothing.
    func testSwipeBackKeepsTheStackInStep() throws {
        open("Radio group")
        tapButton("Large")
        XCTAssertTrue(app.staticTexts["Selected: Large"].waitForExistence(timeout: 5))
        capture("pushed-screen-back-button")

        // Abandoned: a short drag from the edge and back.
        let edge = app.coordinate(withNormalizedOffset: CGVector(dx: 0.0, dy: 0.5))
        edge.press(
            forDuration: 0.1,
            thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.12, dy: 0.5)),
            withVelocity: .slow,
            thenHoldForDuration: 0.3
        )
        XCTAssertTrue(app.staticTexts["Selected: Large"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Back"].isHittable)

        // Completed: the catalog is back.
        edge.press(
            forDuration: 0.1,
            thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5))
        )
        scrollToTop()
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Selected: Large"].exists)

        // Rust agrees about where it is: the next screen opens on top of the
        // catalog and its back button returns there.
        open("Switch")
        XCTAssertTrue(app.navigationBars["Switch"].waitForExistence(timeout: 5))
        backToCatalog()

        // The catalog itself is never swiped away.
        edge.press(
            forDuration: 0.1,
            thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5))
        )
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 5))
    }

    /// Slow work runs off the main thread and reports back as a message; a
    /// timer runs while the application asks for it.
    func testBackgroundWorkAndTheClock() throws {
        open("Async")
        XCTAssertTrue(app.staticTexts["Nothing loaded yet"].waitForExistence(timeout: 5))

        tapButton("Load")
        XCTAssertTrue(app.staticTexts["Loading…"].waitForExistence(timeout: 2))
        // The screen stays usable while the request runs.
        tapButton("Tap")
        XCTAssertTrue(app.staticTexts["Taps: 1"].waitForExistence(timeout: 2))
        XCTAssertTrue(app.staticTexts["Loaded 3 items"].waitForExistence(timeout: 8))

        let clock = app.switches["Clock"]
        XCTAssertTrue(clock.waitForExistence(timeout: 5))
        clock.tap()
        XCTAssertTrue(app.staticTexts["Ticks: 2"].waitForExistence(timeout: 8))

        // In the background the clock stands still.
        XCUIDevice.shared.press(.home)
        sleep(4)
        app.activate()
        XCTAssertTrue(clock.waitForExistence(timeout: 5))
        XCTAssertLessThanOrEqual(ticks(), 4, "the clock ran in the background")

        // Switched off, it stops.
        clock.tap()
        usleep(500_000)
        let stopped = ticks()
        sleep(3)
        XCTAssertEqual(ticks(), stopped)

        backToCatalog()
    }

    /// The number the Async page shows after "Ticks: ".
    private func ticks() -> Int {
        let label = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Ticks: ")
        ).firstMatch.label
        return Int(label.dropFirst("Ticks: ".count)) ?? -1
    }

    /// The native chrome a Scaffold adds around a screen's content.
    func testScaffoldFloatingButtonSnackbarAndDrawer() throws {
        open("Scaffold")
        XCTAssertTrue(app.staticTexts["Items: 0"].waitForExistence(timeout: 5))

        // The floating action button adds an item and raises a snackbar,
        // whose action takes the item back.
        app.buttons["Add"].tap()
        XCTAssertTrue(app.staticTexts["Items: 1"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Added item 1"].waitForExistence(timeout: 5))
        app.buttons["Undo"].tap()
        XCTAssertTrue(app.staticTexts["Items: 0"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Added item 1"].exists)

        // The drawer opens from state and a destination closes it again.
        tapButton("Open drawer")
        let archive = app.staticTexts["Archive"]
        XCTAssertTrue(archive.waitForExistence(timeout: 5))
        archive.tap()
        XCTAssertTrue(app.staticTexts["Section: Archive"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["Close menu"].waitForExistence(timeout: 1))

        // Tapping outside the panel closes it, and Rust hears about it: the
        // drawer can be opened again.
        tapButton("Open drawer")
        XCTAssertTrue(app.buttons["Close menu"].waitForExistence(timeout: 5))
        app.buttons["Close menu"].tap()
        XCTAssertFalse(app.staticTexts["Settings"].waitForExistence(timeout: 1))
        tapButton("Open drawer")
        XCTAssertTrue(app.staticTexts["Settings"].waitForExistence(timeout: 5))
        app.staticTexts["Inbox"].tap()
        XCTAssertTrue(app.staticTexts["Section: Inbox"].waitForExistence(timeout: 5))

        // Leaving the screen takes its chrome with it.
        backToCatalog()
        XCTAssertFalse(app.buttons["Add"].exists)
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
            app.staticTexts["Settings: tapping a tab sends the selected index."]
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
        XCTAssertTrue(app.images["Intrinsic asset icon"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Recycled list")
        XCTAssertTrue(app.staticTexts["Row taps: 0"].waitForExistence(timeout: 5))
        tapButton("Tap Recycled row 1")
        XCTAssertTrue(app.staticTexts["Row taps: 1"].waitForExistence(timeout: 5))
        tapButton("Shuffle rows")
        XCTAssertTrue(
            app.staticTexts["Recycled row 1 · updated"].waitForExistence(timeout: 5)
        )
        backToCatalog()

        open("Styling")
        let disabled = app.buttons["Disabled action"]
        XCTAssertTrue(disabled.waitForExistence(timeout: 5))
        XCTAssertFalse(disabled.isEnabled)
        backToCatalog()
    }

    func testNativeScreensAndPresentations() throws {
        open("Text")
        XCTAssertTrue(app.staticTexts["Display 34 / bold"].waitForExistence(timeout: 5))
        backToCatalog()

        // The dialog is the system alert, so it is found as an alert.
        open("Dialog")
        let alert = app.alerts["Delete task?"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        XCTAssertTrue(alert.staticTexts["This cannot be undone."].exists)
        alert.buttons["Cancel"].tap()
        XCTAssertTrue(alert.waitForNonExistence(timeout: 5))

        open("Dialog")
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        alert.buttons["Delete"].tap()
        XCTAssertTrue(alert.waitForNonExistence(timeout: 5))

        open("Bottom sheet")
        XCTAssertTrue(app.buttons["Open bottom sheet"].waitForExistence(timeout: 5))
        tapButton("Open bottom sheet")
        let filters = app.staticTexts["Filters"]
        XCTAssertTrue(filters.waitForExistence(timeout: 5))
        app.buttons["Close"].tap()
        XCTAssertTrue(filters.waitForNonExistence(timeout: 5))

        // Swiping the sheet away reaches Rust, so it can be opened again.
        tapButton("Open bottom sheet")
        XCTAssertTrue(filters.waitForExistence(timeout: 5))
        filters.swipeDown(velocity: .fast)
        XCTAssertTrue(filters.waitForNonExistence(timeout: 5))
        tapButton("Open bottom sheet")
        XCTAssertTrue(filters.waitForExistence(timeout: 5))
        app.buttons["Close"].tap()
        XCTAssertTrue(filters.waitForNonExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Open bottom sheet"].waitForExistence(timeout: 5))
        backToCatalog()
    }

    /// More choices than an alert usually has are all offered, and a sheet
    /// whose content grows is resized to show the new rows.
    func testManyChoicesAndAGrowingSheet() throws {
        open("Bottom sheet")
        XCTAssertTrue(app.staticTexts["Label: none"].waitForExistence(timeout: 5))

        tapButton("Choose a label")
        let alert = app.alerts["Choose a label"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        for choice in ["Red", "Green", "Blue", "Yellow", "Cancel"] {
            XCTAssertTrue(alert.buttons[choice].exists, "missing choice \(choice)")
        }
        alert.buttons["Blue"].tap()
        XCTAssertTrue(alert.waitForNonExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Label: Blue"].waitForExistence(timeout: 5))

        tapButton("Open bottom sheet")
        let more = app.buttons["More filters"]
        XCTAssertTrue(more.waitForExistence(timeout: 5))
        let closeBefore = app.buttons["Close"].frame.minY
        more.tap()
        let last = app.staticTexts["From this week"]
        XCTAssertTrue(last.waitForExistence(timeout: 5))
        sleep(1)
        // The sheet grew upwards to make room: its rows and the Close button
        // are on screen, not cut off below the edge.
        XCTAssertTrue(last.isHittable)
        let close = app.buttons["Close"]
        XCTAssertTrue(close.isHittable)
        XCTAssertLessThanOrEqual(close.frame.maxY, app.frame.maxY)
        XCTAssertLessThan(app.staticTexts["Filters"].frame.minY, closeBefore)
        close.tap()
        XCTAssertTrue(more.waitForNonExistence(timeout: 5))
        backToCatalog()
    }

    func testContainerWidgets() throws {
        open("View pager")
        XCTAssertTrue(app.staticTexts["Page 1 of 3"].waitForExistence(timeout: 5))
        app.scrollViews["View pager"].swipeLeft()
        XCTAssertTrue(app.staticTexts["Page 2 of 3"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Tabs view")
        XCTAssertTrue(app.staticTexts["Selected: Overview"].waitForExistence(timeout: 5))
        app.segmentedControls.firstMatch.buttons["Details"].tap()
        XCTAssertTrue(app.staticTexts["Selected: Details"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Details page content."].waitForExistence(timeout: 5))
        capture("tabs-view")
        backToCatalog()

        open("Bottom navigation")
        XCTAssertTrue(app.staticTexts["Home destination"].waitForExistence(timeout: 5))
        // The system tab bar reaches the bottom edge of the screen.
        let tabBar = app.tabBars.firstMatch
        XCTAssertEqual(tabBar.frame.maxY, app.frame.maxY, accuracy: 1)
        XCTAssertGreaterThanOrEqual(tabBar.frame.height, 49)
        tabBar.buttons["Search"].tap()
        XCTAssertTrue(app.staticTexts["Search destination"].waitForExistence(timeout: 5))
        backToCatalog()

        open("Toolbar")
        XCTAssertTrue(app.staticTexts["Toolbar taps: 0"].waitForExistence(timeout: 5))
        app.buttons["Refresh"].tap()
        XCTAssertTrue(app.staticTexts["Toolbar taps: 1"].waitForExistence(timeout: 5))
        app.buttons["Save"].tap()
        XCTAssertTrue(app.staticTexts["Toolbar taps: 2"].waitForExistence(timeout: 5))
        backToCatalog()
    }

    /// The device's language and appearance reach the application.
    func testTheApplicationHearsAboutTheEnvironment() throws {
        XCUIDevice.shared.appearance = .light
        defer { XCUIDevice.shared.appearance = .light }

        open("Environment")
        let language = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH 'Language: '")
        ).firstMatch
        XCTAssertTrue(language.waitForExistence(timeout: 5))
        XCTAssertGreaterThan(language.label.count, "Language: ".count, "a language was reported")
        XCTAssertTrue(app.staticTexts["Appearance: light"].exists)

        // The simulator takes a few seconds to switch.
        XCUIDevice.shared.appearance = .dark
        XCTAssertTrue(app.staticTexts["Appearance: dark"].waitForExistence(timeout: 20))
        capture("environment-dark")

        backToCatalog()
    }

    /// The application shows dark while the device is light, and the
    /// native bars follow.
    func testTheApplicationChoosesItsAppearance() throws {
        XCUIDevice.shared.appearance = .light
        open("Environment")
        XCTAssertTrue(app.staticTexts["Appearance: light"].waitForExistence(timeout: 20))
        let light = brightness(of: app.screenshot())

        tapButton("Always dark")
        XCTAssertTrue(app.staticTexts["Appearance: dark"].waitForExistence(timeout: 10))
        sleep(1)
        let dark = brightness(of: app.screenshot())
        XCTAssertGreaterThan(light, 0.7, "the light screen is mostly light")
        XCTAssertLessThan(dark, 0.3, "the dark screen is mostly dark, bars included")
        capture("appearance-always-dark")

        tapButton("Follow device")
        XCTAssertTrue(app.staticTexts["Appearance: light"].waitForExistence(timeout: 10))

        backToCatalog()
    }

    func testAContainerReactsToTaps() throws {
        open("Container")
        XCTAssertTrue(app.staticTexts["Nothing tapped yet."].waitForExistence(timeout: 5))

        // A tap on the card's text lands on the card.
        app.staticTexts["Inbox"].tap()
        XCTAssertTrue(app.staticTexts["Last tapped: Inbox (1 taps)"].waitForExistence(timeout: 5))

        // The button inside the card keeps its own tap.
        tapButton("Mute")
        XCTAssertTrue(app.buttons["Unmute"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Last tapped: Inbox (1 taps)"].exists)

        // A container that paints nothing is tappable too, and is announced
        // as a button.
        app.buttons["Archive row"].tap()
        XCTAssertTrue(
            app.staticTexts["Last tapped: Archive (2 taps)"].waitForExistence(timeout: 5))

        // Disabled and plain containers ignore taps.
        app.staticTexts["Disabled card"].tap()
        app.staticTexts["A plain container ignores taps."].tap()
        sleep(1)
        XCTAssertTrue(app.staticTexts["Last tapped: Archive (2 taps)"].exists)
        capture("container")

        backToCatalog()
    }

    func testPageTransitions() throws {
        open("Page transitions")

        tapButton("Push with slide")
        XCTAssertTrue(
            app.staticTexts["Pushed with the native slide transition."]
                .waitForExistence(timeout: 5)
        )
        app.buttons["Back"].tap()

        tapButton("Push with fade")
        XCTAssertTrue(
            app.staticTexts["Pushed with a cross-fade transition."]
                .waitForExistence(timeout: 5)
        )
        app.buttons["Back"].tap()

        tapButton("Push with no animation")
        XCTAssertTrue(
            app.staticTexts["Pushed with no animation."]
                .waitForExistence(timeout: 5)
        )
        app.buttons["Back"].tap()
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
        scrollToTop()
        XCTAssertTrue(app.staticTexts["Krabik UI DSL"].waitForExistence(timeout: 5))
    }

    /// Keeps a screenshot with the test result, for looking at by eye.
    private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func scrollToTop() {
        for _ in 0..<8 {
            app.swipeDown()
        }
    }
}
