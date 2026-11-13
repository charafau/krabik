import QuartzCore
import UIKit

/// Owns the native chrome of one session: the navigation stack and bar, the
/// tab bar, the system alert and the sheet.
///
/// Rust is the source of truth. After an application state change it calls
/// the `krabik…` methods below. Nothing here changes what is shown on its own;
/// user input is reported to Rust, which decides.
final class KrabikHostController: UIViewController {
    private var session: UInt64 = 0
    private var mainHost: RustHostView!
    private var navigation: UINavigationController!
    private let tabBar = UITabBar()
    private var tabLabels: [String] = []
    private var tabIcons: [String] = []
    private weak var alert: UIAlertController?
    private weak var sheet: UIViewController?
    /// The sheet host waiting for its deferred presentation, if any.
    private var pendingSheet: Int64?

    deinit {
        // Releases the Rust session and every native view it created.
        if session != 0 {
            krabik_ios_destroy(session)
        }
    }

    override func viewDidLoad() {
        super.viewDidLoad()

        mainHost = RustHostView(frame: view.bounds)
        session = krabik_ios_create(
            Unmanaged.passUnretained(mainHost).toOpaque(),
            Double(view.bounds.width),
            Double(view.bounds.height)
        )
        mainHost.bindMain(session: session)
        mainHost.frame = view.bounds
        mainHost.autoresizingMask = [.flexibleWidth, .flexibleHeight]

        navigation = UINavigationController(rootViewController: hostController(mainHost))
        navigation.setNavigationBarHidden(true, animated: false)
        navigation.interactivePopGestureRecognizer?.isEnabled = false

        addChild(navigation)
        navigation.view.frame = view.bounds
        navigation.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(navigation.view)
        navigation.didMove(toParent: self)

        // The system tab bar spans the bottom edge, including the area under
        // the home indicator, and keeps its standard 49pt content height.
        tabBar.delegate = self
        tabBar.isHidden = true
        tabBar.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(tabBar)
        NSLayoutConstraint.activate([
            tabBar.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            tabBar.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            tabBar.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            tabBar.topAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.bottomAnchor,
                constant: -Self.tabBarHeight
            ),
        ])

        krabik_ios_set_controller(session, Unmanaged.passUnretained(self).toOpaque())
    }

    @objc func krabikPush(_ host: Int64, title: String, transition: Int) {
        let hostView = RustHostView(frame: .zero)
        hostView.bind(session: session, host: UInt64(host))
        let vc = hostController(hostView)
        vc.title = title
        // Rust owns the stack, so the system back button (which pops on its
        // own) is replaced by one that asks Rust. The first screen has none.
        vc.navigationItem.hidesBackButton = true
        if navigation.viewControllers.count > 1 {
            let back = UIBarButtonItem(
                image: UIImage(systemName: "chevron.backward"),
                style: .plain,
                target: self,
                action: #selector(backTapped)
            )
            back.accessibilityLabel = "Back"
            vc.navigationItem.leftBarButtonItem = back
        }
        switch transition {
        case 1:
            addFadeTransition()
            navigation.setNavigationBarHidden(false, animated: false)
            navigation.pushViewController(vc, animated: false)
        case 2:
            navigation.setNavigationBarHidden(false, animated: false)
            navigation.pushViewController(vc, animated: false)
        default:
            navigation.setNavigationBarHidden(false, animated: true)
            navigation.pushViewController(vc, animated: true)
        }
    }

    /// Pops `count` screens in one step, animated with the top screen's transition.
    @objc func krabikPop(_ count: Int, transition: Int) {
        let controllers = navigation.viewControllers
        guard count > 0, controllers.count > 1 else { return }
        let target = controllers[max(0, controllers.count - 1 - count)]
        switch transition {
        case 1:
            addFadeTransition()
            navigation.popToViewController(target, animated: false)
        case 2:
            navigation.popToViewController(target, animated: false)
        default:
            navigation.popToViewController(target, animated: true)
        }
        if target === controllers.first {
            navigation.setNavigationBarHidden(true, animated: transition == 0)
        }
    }

    @objc func krabikSetToolbar(
        _ title: String,
        subtitle: String,
        backTitle: String,
        hidden: Bool,
        actionLabels: [String],
        actionIcons: [String]
    ) {
        guard let top = navigation.viewControllers.last else { return }

        if subtitle.isEmpty {
            top.navigationItem.title = title
            top.navigationItem.titleView = nil
        } else {
            let titleLabel = UILabel()
            titleLabel.text = title
            titleLabel.font = .systemFont(ofSize: 17, weight: .semibold)
            titleLabel.textAlignment = .center
            let subtitleLabel = UILabel()
            subtitleLabel.text = subtitle
            subtitleLabel.font = .systemFont(ofSize: 12)
            subtitleLabel.textColor = .secondaryLabel
            subtitleLabel.textAlignment = .center
            let stack = UIStackView(arrangedSubviews: [titleLabel, subtitleLabel])
            stack.axis = .vertical
            stack.alignment = .center
            stack.spacing = 0
            stack.sizeToFit()
            top.navigationItem.titleView = stack
        }

        if navigation.viewControllers.count > 1 {
            navigation.viewControllers[navigation.viewControllers.count - 2]
                .navigationItem.backButtonTitle = backTitle
        }

        var items: [UIBarButtonItem] = []
        for index in actionLabels.indices {
            let label = actionLabels[index]
            let iconName = actionIcons.indices.contains(index) ? actionIcons[index] : ""
            if iconName.isEmpty {
                let item = UIBarButtonItem(
                    title: label,
                    style: .plain,
                    target: self,
                    action: #selector(toolbarTapped(_:))
                )
                item.tag = index
                items.append(item)
            } else {
                let button = UIButton(type: .system)
                button.setTitle(label, for: .normal)
                if let image = toolbarImage(iconName) {
                    button.setImage(image, for: .normal)
                }
                button.sizeToFit()
                button.tag = index
                button.addTarget(self, action: #selector(toolbarTapped(_:)), for: .touchUpInside)
                items.append(UIBarButtonItem(customView: button))
            }
        }
        top.navigationItem.rightBarButtonItems = items
        navigation.setNavigationBarHidden(
            hidden || navigation.viewControllers.count <= 1,
            animated: false
        )
    }

    @objc private func toolbarTapped(_ sender: Any) {
        // Each item's tag is its index in the screen's toolbar actions.
        let index: Int
        if let item = sender as? UIBarButtonItem {
            index = item.tag
        } else if let button = sender as? UIButton {
            index = button.tag
        } else {
            return
        }
        krabik_ios_on_toolbar_action(session, UInt64(index))
    }

    private func toolbarImage(_ name: String) -> UIImage? {
        UIImage(systemName: name) ?? UIImage(named: name)
    }

    private func addFadeTransition() {
        let transition = CATransition()
        transition.duration = 0.28
        transition.type = .fade
        transition.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        navigation.view.layer.add(transition, forKey: kCATransition)
    }

    /// Switches between the main app root and the navigation screens.
    @objc func krabikSetRootVisible(_ visible: Bool) {
        mainHost.isHidden = !visible
        navigation.setNavigationBarHidden(visible, animated: true)
    }

    // MARK: - Bottom navigation

    /// Applies the top screen's tab bar; no items hides it.
    @objc func krabikSetBottomNavigation(_ labels: [String], icons: [String], selected: Int) {
        if labels != tabLabels || icons != tabIcons {
            tabLabels = labels
            tabIcons = icons
            tabBar.items = labels.indices.map { index in
                let name = icons.indices.contains(index) ? icons[index] : ""
                let image = name.isEmpty ? nil : toolbarImage(name)
                return UITabBarItem(title: labels[index], image: image, tag: index)
            }
        }
        if let items = tabBar.items, !items.isEmpty {
            tabBar.selectedItem = items[min(max(selected, 0), items.count - 1)]
        }
        tabBar.isHidden = labels.isEmpty
        // Keeps every screen's content above the bar.
        navigation.additionalSafeAreaInsets.bottom = labels.isEmpty ? 0 : Self.tabBarHeight
    }

    // MARK: - Dialog

    /// Shows the system alert. `roles` holds one code per button: 0 default,
    /// 1 cancel, 2 destructive. A pressed button reports its index.
    @objc func krabikShowDialog(
        _ title: String,
        message: String,
        labels: [String],
        roles: [NSNumber]
    ) {
        krabikDismissDialog()
        let alert = UIAlertController(
            title: title,
            message: message.isEmpty ? nil : message,
            preferredStyle: .alert
        )
        var hasCancel = false
        for index in labels.indices {
            let role = roles.indices.contains(index) ? roles[index].intValue : 0
            let style: UIAlertAction.Style
            switch role {
            // UIKit allows a single cancel action.
            case 1 where !hasCancel:
                hasCancel = true
                style = .cancel
            case 2:
                style = .destructive
            default:
                style = .default
            }
            alert.addAction(UIAlertAction(title: labels[index], style: style) { [weak self] _ in
                self?.dialogClosed(button: index)
            })
        }
        if labels.isEmpty {
            // An alert without buttons could never be closed.
            alert.addAction(UIAlertAction(title: "OK", style: .default) { [weak self] _ in
                self?.dialogClosed(button: -1)
            })
        }
        self.alert = alert
        topPresenter().present(alert, animated: true)
    }

    /// Closes the system alert without reporting anything back.
    @objc func krabikDismissDialog() {
        alert?.dismiss(animated: true)
        alert = nil
    }

    private func dialogClosed(button: Int) {
        alert = nil
        krabik_ios_on_dialog_action(session, Int64(button))
    }

    // MARK: - Sheet

    /// Presents the native sheet around a Rust host, sized to its content.
    ///
    /// Rust calls this while it holds the session lock, so the work is
    /// deferred one main-queue turn: the content is rendered first, and its
    /// height becomes the sheet's detent.
    @objc func krabikPresentSheet(_ host: Int64) {
        pendingSheet = host
        DispatchQueue.main.async { [weak self] in
            guard let self, self.pendingSheet == host else { return }
            self.pendingSheet = nil

            let hostView = RustHostView(frame: self.view.bounds)
            hostView.backgroundColor = .clear
            hostView.bindNow(session: self.session, host: UInt64(host))
            let contentHeight = CGFloat(krabik_ios_content_height(self.session, UInt64(host)))

            let controller = self.hostController(hostView, background: .systemBackground)
            controller.modalPresentationStyle = .pageSheet
            if let presentation = controller.sheetPresentationController {
                presentation.detents = [
                    .custom { context in min(contentHeight, context.maximumDetentValue) },
                ]
                presentation.prefersGrabberVisible = true
            }
            controller.presentationController?.delegate = self
            self.sheet = controller
            self.topPresenter().present(controller, animated: true)
        }
    }

    /// Dismisses the sheet without reporting anything back.
    @objc func krabikDismissSheet() {
        pendingSheet = nil
        sheet?.dismiss(animated: true)
        sheet = nil
    }

    // MARK: - Helpers

    /// Wraps a Rust host in a controller that keeps it inside the safe area,
    /// so content never sits under the status bar, home indicator or tab bar.
    private func hostController(
        _ host: RustHostView,
        background: UIColor = .white
    ) -> UIViewController {
        let controller = UIViewController()
        controller.view.backgroundColor = background
        host.translatesAutoresizingMaskIntoConstraints = false
        controller.view.addSubview(host)
        let guide = controller.view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            host.topAnchor.constraint(equalTo: guide.topAnchor),
            host.bottomAnchor.constraint(equalTo: guide.bottomAnchor),
            host.leadingAnchor.constraint(equalTo: guide.leadingAnchor),
            host.trailingAnchor.constraint(equalTo: guide.trailingAnchor),
        ])
        return controller
    }

    /// The controller that can present right now: the topmost presented one.
    private func topPresenter() -> UIViewController {
        var top: UIViewController = self
        while let next = top.presentedViewController, !next.isBeingDismissed {
            top = next
        }
        return top
    }

    private static let tabBarHeight: CGFloat = 49

    @objc private func backTapped() {
        _ = krabik_ios_on_back(session)
    }
}

extension KrabikHostController: UITabBarDelegate {
    func tabBar(_ tabBar: UITabBar, didSelect item: UITabBarItem) {
        krabik_ios_on_bottom_navigation_selected(session, UInt64(item.tag))
    }
}

extension KrabikHostController: UIAdaptivePresentationControllerDelegate {
    /// The user swiped the sheet away or tapped outside it.
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
        sheet = nil
        krabik_ios_on_modal_dismiss(session)
    }
}
