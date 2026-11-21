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
        // The lines of the "more" menu are behind one button on the bar.
        app.buttons["More"].tap()
        XCTAssertTrue(app.buttons["Settings"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["Help"].exists)
        app.buttons["Settings"].tap()
        XCTAssertTrue(app.staticTexts["Toolbar taps: 3"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Last action: Settings"].exists)
        backToCatalog()
    }

    /// Swiping a row of a recycled list away, dragging one elsewhere, and
    /// the header that stays at the top.
    func testListGestures() throws {
        func shown(_ text: String) {
            XCTAssertTrue(app.staticTexts[text].waitForExistence(timeout: 5), "\(text) is shown")
        }

        open("List gestures")
        shown("32 rows · Fruit 1, Fruit 2, Fruit 3")
        let listTop = app.staticTexts["Fruit"].frame.minY

        app.staticTexts["Fruit 2"].swipeLeft()
        shown("31 rows · Fruit 1, Fruit 3, Fruit 4")
        XCTAssertFalse(app.staticTexts["Fruit 2"].exists)
        // The application keeps a header: the row comes back.
        app.staticTexts["Fruit"].swipeLeft()
        sleep(1)
        shown("31 rows · Fruit 1, Fruit 3, Fruit 4")
        XCTAssertTrue(app.staticTexts["Fruit"].exists)

        // A long press picks a row up; it is dropped two rows further down.
        app.staticTexts["Fruit 1"].press(forDuration: 1.0, thenDragTo: app.staticTexts["Fruit 4"])
        shown("31 rows · Fruit 3, Fruit 4, Fruit 1")
        XCTAssertGreaterThan(
            app.staticTexts["Fruit 1"].frame.minY, app.staticTexts["Fruit 4"].frame.minY
        )
        capture("list-reordered")

        // Scrolled, the section's header stays at the top of the list.
        app.swipeUp()
        sleep(1)
        capture("list-sticky")
        let pinned = ["Fruit", "Vegetables", "Grains", "Spices"].contains { name in
            app.staticTexts.matching(identifier: name).allElementsBoundByIndex.contains {
                abs($0.frame.minY - listTop) < 20
            }
        }
        XCTAssertTrue(pinned, "a header is pinned at the top of the list")
        backToCatalog()
    }

    /// A view the application registers, and the framework's web view.
    func testNativeViewsAreLaidOutAndReportBack() throws {
        func shown(_ text: String, timeout: TimeInterval = 5) {
            XCTAssertTrue(app.staticTexts[text].waitForExistence(timeout: timeout), "\(text) is shown")
        }

        open("Native views")
        shown("Stars: 2")
        // Five stars from the application's Swift code, in the frame Rust gave.
        let four = app.buttons["4 stars"]
        XCTAssertTrue(four.exists)
        XCTAssertEqual(app.buttons["5 stars"].frame.maxX - app.buttons["1 stars"].frame.minX, 240, accuracy: 1)
        XCTAssertEqual(four.frame.height, 56, accuracy: 1)
        // What the view reports reaches the application.
        four.tap()
        shown("Stars: 4")
        // A new property reaches the same view.
        tapButton("One star")
        shown("Stars: 1")

        // The web view loaded its page and said so.
        shown("Pages loaded: 1", timeout: 15)
        XCTAssertTrue(app.webViews.staticTexts["Hello from a web view"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.otherElements["Missing view"].exists)
        capture("native-views")
        backToCatalog()
    }

    /// The platform's menu on buttons and containers, and the choices built
    /// on it.
    func testMenusAndChoices() throws {
        func shown(_ text: String) {
            XCTAssertTrue(app.staticTexts[text].waitForExistence(timeout: 5), "\(text) is shown")
        }
        func pick(_ line: String) {
            let item = app.buttons[line]
            XCTAssertTrue(item.waitForExistence(timeout: 5), "the menu has \(line)")
            item.tap()
        }

        open("Menus & choices")
        shown("Sorted by Name")
        app.buttons["Sort menu"].tap()
        capture("menu-open")
        pick("Date")
        shown("Sorted by Date")

        app.buttons["More actions"].tap()
        pick("Delete")
        shown("Last action: Delete")

        // A long press shows a label that explains an icon.
        app.buttons["Find notes"].press(forDuration: 1.0)
        // The label is added to the window, where a UI test does not find
        // it; the screenshot is the evidence. It goes away on its own.
        capture("tooltip")
        sleep(3)
        shown("Last action: Delete")

        // A context menu opens on a long press and leaves the tap alone.
        let target = app.buttons["Context target"]
        target.tap()
        shown("Last action: Opened")
        target.press(forDuration: 1.0)
        capture("context-menu")
        pick("Rename")
        shown("Last action: Rename")

        shown("Country: none")
        app.buttons["Country dropdown"].tap()
        pick("Japan")
        shown("Country: Japan")
        capture("dropdown-picked")

        // A touch on the track moves the nearer thumb, to a multiple of ten.
        let price = app.descendants(matching: .any)["Price range"]
        XCTAssertTrue(price.exists)
        XCTAssertEqual(price.frame.height, 48, accuracy: 0.5)
        shown("Price: 20 to 80")
        price.coordinate(withNormalizedOffset: CGVector(dx: 0.33, dy: 0.5)).tap()
        shown("Price: 30 to 80")
        price.coordinate(withNormalizedOffset: CGVector(dx: 0.97, dy: 0.5)).tap()
        shown("Price: 30 to 100")
        capture("range-slider")

        // A text that can be copied offers "Copy" when it is held.
        app.descendants(matching: .any)["Copyable text"].press(forDuration: 1.0)
        XCTAssertTrue(app.buttons["Copy"].waitForExistence(timeout: 5))
        app.buttons["Copy"].tap()

        let week = app.buttons["Week"]
        XCTAssertTrue(week.exists)
        // The three segments share the width.
        XCTAssertEqual(week.frame.width, app.buttons["Month"].frame.width, accuracy: 0.5)
        week.tap()
        shown("Range: Week")
        capture("segmented")
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

    /// The platform's pull-to-refresh on a scroll view and on a recycled list.
    func testPullingDownRefreshes() throws {
        open("Pull to refresh")
        XCTAssertTrue(app.staticTexts["Scroll refreshes: 0"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["List refreshes: 0"].exists)

        pullDown(app.staticTexts["Pull this box down"])
        XCTAssertTrue(app.staticTexts["Scroll refreshes: 1"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["List refreshes: 0"].exists)

        // The list grows by a page of rows with every refresh.
        XCTAssertFalse(app.staticTexts["Message 6"].exists)
        pullDown(app.staticTexts["Message 2"])
        XCTAssertTrue(app.staticTexts["List refreshes: 1"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Message 6"].waitForExistence(timeout: 5))
        capture("pull-to-refresh")

        backToCatalog()
    }

    /// Drags an element far enough down for the refresh control to fire.
    private func pullDown(_ element: XCUIElement) {
        let start = element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
        let end = start.withOffset(CGVector(dx: 0, dy: 320))
        start.press(forDuration: 0.05, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0.2)
    }

    /// An SVG asset is drawn by Rust, and a stack puts its children on top
    /// of each other.
    func testAStackLayersItsChildrenOverVectorArt() throws {
        XCUIDevice.shared.appearance = .light
        open("Stack")
        let backdrop = app.images["Sunrise backdrop"]
        let caption = app.staticTexts["Caption over the art"]
        XCTAssertTrue(backdrop.waitForExistence(timeout: 10))
        XCTAssertTrue(caption.exists)
        XCTAssertTrue(app.staticTexts["Centered"].exists)

        // The caption is inside the image's frame: it lies on top of it.
        XCTAssertTrue(backdrop.frame.contains(caption.frame), "the caption lies over the image")
        // An image view with nothing in it would show the light page behind.
        XCTAssertLessThan(brightness(of: backdrop.screenshot()), 0.6, "the sunrise was drawn")
        capture("stack-over-svg")
        backToCatalog()

        // On the Image page the view takes the drawing's 16:9 shape.
        open("Image")
        let sunrise = app.images["Vector sunrise"]
        XCTAssertTrue(sunrise.waitForExistence(timeout: 10))
        XCTAssertEqual(sunrise.frame.width / sunrise.frame.height, 16.0 / 9.0, accuracy: 0.05)
        backToCatalog()
    }

    /// A text's frame is the size its style needs: a line height applies to a
    /// single line and to every line of a paragraph.
    func testTextStylesAreMeasuredForWhatIsDrawn() throws {
        open("Text")
        XCTAssertTrue(app.staticTexts["Thin 100"].waitForExistence(timeout: 10))
        for label in ["Medium 500", "Black 900", "Italic", "Bold italic", "Underlined",
                      "Struck through", "LETTER SPACING 2"] {
            XCTAssertTrue(app.staticTexts[label].exists, "\(label) is shown")
        }

        let single = app.staticTexts["One line, 40 high"]
        XCTAssertTrue(single.exists)
        XCTAssertEqual(single.frame.height, 40, accuracy: 1)

        // The plain 16 point texts show what one line needs without a line height.
        let plain = app.staticTexts["Italic"].frame.height
        XCTAssertLessThan(plain, 28)

        let paragraph = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Line height 28:")
        ).firstMatch
        XCTAssertTrue(paragraph.exists)
        let lines = (paragraph.frame.height / 28).rounded()
        XCTAssertGreaterThanOrEqual(lines, 2, "the paragraph wraps")
        XCTAssertEqual(paragraph.frame.height, lines * 28, accuracy: 1)

        // A rich text is one label, taller than a plain line of its base
        // size because one of its parts is larger.
        let rich = app.staticTexts["Rich text: bold, italic, teal, underlined and large."]
        XCTAssertTrue(rich.exists)
        XCTAssertGreaterThan(rich.frame.height, plain)
        XCTAssertTrue(app.staticTexts["Was 19.99 now 12.50"].exists)

        // A text in a font file, and one whose file does not exist.
        XCTAssertTrue(app.staticTexts["A font from a file"].exists)
        XCTAssertTrue(app.staticTexts["A missing font file falls back"].exists)
        XCTAssertEqual(app.staticTexts["A missing font file falls back"].frame.height, plain, accuracy: 1)

        app.swipeUp()
        capture("text-styles")
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

    /// The layout additions (wrapping, positioned children, long presses,
    /// icons) and the widgets that are assembled from containers.
    func testLayoutAdditionsAndComposedWidgets() throws {
        func shown(_ text: String) {
            XCTAssertTrue(app.staticTexts[text].waitForExistence(timeout: 5), "\(text) is shown")
        }
        func reveal(_ element: XCUIElement) {
            var attempts = 0
            while !(element.exists && element.isHittable) && attempts < 8 {
                app.swipeUp()
                attempts += 1
            }
            XCTAssertTrue(element.exists, "\(element) can be reached")
        }

        open("Wrap & position")
        // The six chips do not fit on one line, so the row wraps.
        let rust = app.buttons["Language Rust"]
        XCTAssertTrue(rust.waitForExistence(timeout: 10))
        let kotlin = app.buttons["Language Kotlin"]
        let typescript = app.buttons["Language TypeScript"]
        XCTAssertGreaterThanOrEqual(typescript.frame.minY, rust.frame.maxY)
        XCTAssertLessThan(typescript.frame.minX, kotlin.frame.minX)
        XCTAssertEqual(rust.frame.height, 32, accuracy: 0.5)
        kotlin.tap()
        shown("Picked: Rust, Kotlin")
        XCTAssertEqual(app.otherElements["Positioned stack"].frame.size.width, 220, accuracy: 0.5)
        // A maximum width wraps the text early.
        let narrow = app.otherElements["Narrow text"]
        XCTAssertEqual(narrow.frame.width, 160, accuracy: 1)
        XCTAssertGreaterThan(narrow.frame.height, 30)
        capture("layout-top")

        let target = app.buttons["Press target"]
        reveal(target)
        XCTAssertTrue(app.images["Add icon"].exists)
        // The space is shared out around the three boxes, or evenly.
        XCTAssertTrue(app.otherElements["Space around"].exists)
        target.tap()
        shown("Taps: 1 · Holds: 0")
        target.press(forDuration: 1.0)
        shown("Taps: 1 · Holds: 1")
        capture("layout-bottom")
        backToCatalog()

        open("Display pieces")
        shown("A card")
        shown("Unread: 3")
        // The badge shows the count, and lies partly outside its icon.
        let icon = app.images["Unread icon"]
        let badge = app.staticTexts["3"]
        XCTAssertTrue(badge.exists)
        XCTAssertGreaterThan(badge.frame.maxX, icon.frame.maxX)
        XCTAssertLessThan(badge.frame.minY, icon.frame.minY + 4)
        // An avatar is a circle of twice its radius.
        tapButton("Add unread")
        shown("Unread: 4")
        shown("4")
        app.staticTexts["Wi-Fi"].tap()
        shown("Tile taps: 1")
        // The switch inside the tile keeps its own taps.
        app.switches["Wi-Fi switch"].tap()
        shown("Off")
        shown("Tile taps: 1")
        capture("pieces-top")
        let remove = app.buttons["Remove urgent"]
        reveal(remove)
        remove.tap()
        XCTAssertTrue(app.staticTexts["urgent"].waitForNonExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["draft"].exists)
        let share = app.buttons["Share item"]
        reveal(share)
        XCTAssertGreaterThanOrEqual(share.frame.width, 44)
        XCTAssertGreaterThanOrEqual(share.frame.height, 44)
        share.tap()
        shown("Button taps: 1")
        XCTAssertTrue(app.buttons["Filled"].exists)
        capture("pieces-bottom")
        backToCatalog()

        open("Composed inputs")
        let advanced = app.buttons["Advanced"]
        XCTAssertTrue(advanced.waitForExistence(timeout: 10))
        advanced.tap()
        shown("Hidden until expanded")
        capture("inputs-top")
        advanced.tap()
        XCTAssertTrue(app.staticTexts["Hidden until expanded"].waitForNonExistence(timeout: 5))
        app.buttons["Increase"].tap()
        shown("Quantity: 3")
        app.buttons["Decrease"].tap()
        app.buttons["Decrease"].tap()
        shown("Quantity: 1")
        // At the end of the range the minus is dimmed and no longer a button.
        XCTAssertFalse(app.buttons["Decrease"].exists)
        XCTAssertTrue(app.otherElements["Decrease"].exists)
        XCTAssertTrue(app.buttons["Increase"].exists)

        let search = app.textFields["Search notes"]
        XCTAssertTrue(search.exists)
        search.tap()
        search.typeText("tea")
        shown("Query: \"tea\" · searched: \"\"")
        app.buttons["Clear"].tap()
        shown("Query: \"\" · searched: \"\"")

        let city = app.textFields["City"]
        reveal(city)
        city.tap()
        city.typeText("to")
        shown("Kyoto")
        capture("inputs-suggestions")
        app.staticTexts["Toronto"].tap()
        shown("City: Toronto")
        XCTAssertFalse(app.staticTexts["Kyoto"].exists)

        let next = app.buttons["To payment"]
        reveal(next)
        next.tap()
        shown("How do you pay?")
        XCTAssertFalse(app.staticTexts["Where should it go?"].exists)
        capture("inputs-stepper")
        backToCatalog()

        open("Grid")
        shown("A list with three equally wide items on a line.")
        let one = app.otherElements["Tile 1"]
        XCTAssertTrue(one.waitForExistence(timeout: 5))
        let two = app.otherElements["Tile 2"].frame
        let three = app.otherElements["Tile 3"].frame
        let four = app.otherElements["Tile 4"].frame
        XCTAssertEqual(one.frame.minY, two.minY, accuracy: 0.5)
        XCTAssertEqual(one.frame.width, two.width, accuracy: 0.5)
        XCTAssertEqual(two.width, three.width, accuracy: 0.5)
        XCTAssertEqual(two.minX - one.frame.maxX, 8, accuracy: 0.5)
        XCTAssertEqual(four.minX, one.frame.minX, accuracy: 0.5)
        XCTAssertEqual(four.minY - one.frame.maxY, 8, accuracy: 0.5)
        app.swipeUp()
        capture("grid")
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
