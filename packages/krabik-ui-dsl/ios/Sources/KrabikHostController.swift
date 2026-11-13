import QuartzCore
import UIKit

/// Owns the native navigation controller and modal presentations for a session.
///
/// Rust is the source of truth: it calls `krabikPush`, `krabikPop`,
/// `krabikPresent` and `krabikDismiss` after an application state change. The
/// navigation bar and transitions are native; the back button asks Rust first.
final class KrabikHostController: UIViewController {
    private var session: UInt64 = 0
    private var mainHost: RustHostView!
    private var navigation: UINavigationController!
    private weak var presented: UIViewController?
    private var toolbarActions: [String] = []

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

        let root = UIViewController()
        root.view = mainHost
        navigation = UINavigationController(rootViewController: root)
        navigation.setNavigationBarHidden(true, animated: false)
        navigation.interactivePopGestureRecognizer?.isEnabled = false

        addChild(navigation)
        navigation.view.frame = view.bounds
        navigation.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(navigation.view)
        navigation.didMove(toParent: self)

        krabik_ios_set_controller(session, Unmanaged.passUnretained(self).toOpaque())
    }

    @objc func krabikPush(_ host: Int64, title: String, transition: Int) {
        let vc = UIViewController()
        let hostView = RustHostView(frame: .zero)
        hostView.bind(session: session, host: UInt64(host))
        vc.view = hostView
        vc.title = title
        if navigation.viewControllers.count > 1 {
            vc.navigationItem.leftBarButtonItem = UIBarButtonItem(
                title: "Back",
                style: .plain,
                target: self,
                action: #selector(backTapped)
            )
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

    @objc func krabikPop(_ transition: Int) {
        let toRoot = navigation.viewControllers.count <= 2
        switch transition {
        case 1:
            addFadeTransition()
            navigation.popViewController(animated: false)
        case 2:
            navigation.popViewController(animated: false)
        default:
            navigation.popViewController(animated: true)
        }
        if toRoot {
            navigation.setNavigationBarHidden(true, animated: transition == 0)
        }
    }

    @objc func krabikSetToolbar(
        _ title: String,
        subtitle: String,
        backTitle: String,
        hidden: Bool,
        actionIds: [String],
        actionLabels: [String],
        actionIcons: [String]
    ) {
        toolbarActions = actionIds
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
        let index: Int
        if let item = sender as? UIBarButtonItem {
            index = item.tag
        } else if let button = sender as? UIButton {
            index = button.tag
        } else {
            return
        }
        guard toolbarActions.indices.contains(index) else { return }
        toolbarActions[index].withCString { action in
            krabik_ios_on_toolbar_action(session, action)
        }
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

    @objc func krabikSetRootVisible(_ visible: Bool) {
        mainHost.isHidden = !visible
        navigation.setNavigationBarHidden(visible, animated: true)
    }

    @objc func krabikPresent(_ host: Int64, kind: Int) {
        let hostView = RustHostView(frame: .zero)
        hostView.bind(session: session, host: UInt64(host))

        if kind == 1 {
            let sheet = UIViewController()
            sheet.view = hostView
            sheet.modalPresentationStyle = .pageSheet
            if #available(iOS 15.0, *),
               let presentation = sheet.sheetPresentationController {
                presentation.detents = [.medium(), .large()]
                presentation.prefersGrabberVisible = true
            }
            presented = sheet
            present(sheet, animated: true)
            return
        }

        let wrapper = UIViewController()
        wrapper.modalPresentationStyle = .overFullScreen
        wrapper.view.backgroundColor = UIColor.black.withAlphaComponent(0.4)
        hostView.translatesAutoresizingMaskIntoConstraints = false
        wrapper.view.addSubview(hostView)
        NSLayoutConstraint.activate([
            hostView.centerXAnchor.constraint(equalTo: wrapper.view.centerXAnchor),
            hostView.centerYAnchor.constraint(equalTo: wrapper.view.centerYAnchor),
            hostView.widthAnchor.constraint(equalTo: wrapper.view.widthAnchor, multiplier: 0.9),
            hostView.heightAnchor.constraint(equalTo: wrapper.view.heightAnchor, multiplier: 0.35),
        ])
        let tap = UITapGestureRecognizer(target: self, action: #selector(backdropTapped))
        wrapper.view.addGestureRecognizer(tap)
        presented = wrapper
        present(wrapper, animated: true)
    }

    @objc func krabikDismiss() {
        presented?.dismiss(animated: true)
        presented = nil
    }

    @objc private func backTapped() {
        _ = krabik_ios_on_back(session)
    }

    @objc private func backdropTapped() {
        krabik_ios_on_modal_dismiss(session)
    }
}
