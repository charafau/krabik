import UIKit

// The pieces of a screen's chrome that UIKit has no control for.
//
// Android has a Material snackbar and navigation drawer; iOS has neither, so
// these are small views with the same behaviour. They are plain UIKit and
// know nothing about Rust: `KrabikHostController` owns them and reports what
// the user does.

/// A brief message at the bottom of the screen, with an optional action.
final class KrabikSnackbarView: UIView {
    /// The action button was pressed.
    var onAction: (() -> Void)?

    init(message: String, action: String) {
        super.init(frame: .zero)
        backgroundColor = UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(white: 0.92, alpha: 1)
                : UIColor(white: 0.16, alpha: 1)
        }
        let foreground = UIColor { traits in
            traits.userInterfaceStyle == .dark ? .black : .white
        }
        layer.cornerRadius = 12
        layer.shadowColor = UIColor.black.cgColor
        layer.shadowOpacity = 0.2
        layer.shadowRadius = 8
        layer.shadowOffset = CGSize(width: 0, height: 2)

        let label = UILabel()
        label.text = message
        label.textColor = foreground
        label.font = .preferredFont(forTextStyle: .subheadline)
        label.adjustsFontForContentSizeCategory = true
        label.numberOfLines = 2
        label.setContentHuggingPriority(.defaultLow, for: .horizontal)

        let stack = UIStackView(arrangedSubviews: [label])
        stack.axis = .horizontal
        stack.alignment = .center
        stack.spacing = 12
        if !action.isEmpty {
            let button = UIButton(type: .system)
            button.setTitle(action, for: .normal)
            button.titleLabel?.font = .preferredFont(forTextStyle: .subheadline).bold()
            button.tintColor = UIColor { traits in
                traits.userInterfaceStyle == .dark ? .systemBlue : .systemTeal
            }
            button.setContentHuggingPriority(.required, for: .horizontal)
            button.addTarget(self, action: #selector(actionTapped), for: .touchUpInside)
            stack.addArrangedSubview(button)
        }
        stack.translatesAutoresizingMaskIntoConstraints = false
        addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 16),
            stack.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -16),
            stack.topAnchor.constraint(equalTo: topAnchor, constant: 12),
            stack.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12),
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    @objc private func actionTapped() {
        onAction?()
    }
}

private extension UIFont {
    func bold() -> UIFont {
        guard let descriptor = fontDescriptor.withSymbolicTraits(.traitBold) else { return self }
        return UIFont(descriptor: descriptor, size: 0)
    }
}

/// A panel of destinations that slides in from the leading edge, over a
/// dimmed screen.
final class KrabikDrawerView: UIView, UITableViewDataSource, UITableViewDelegate {
    /// A destination was picked.
    var onSelect: ((Int) -> Void)?
    /// The user dismissed the drawer by tapping outside it or swiping it shut.
    var onClose: (() -> Void)?

    private let dim = UIControl()
    private let panel = UIView()
    private let table = UITableView(frame: .zero, style: .insetGrouped)
    private var labels: [String] = []
    private var icons: [String] = []
    private var selected = -1
    private var panelLeading: NSLayoutConstraint!
    private(set) var isOpen = false

    private static let cell = "krabik-drawer-item"

    override init(frame: CGRect) {
        super.init(frame: frame)
        isHidden = true

        dim.backgroundColor = UIColor.black.withAlphaComponent(0.35)
        dim.alpha = 0
        dim.isAccessibilityElement = true
        dim.accessibilityTraits = .button
        dim.accessibilityLabel = "Close menu"
        dim.addTarget(self, action: #selector(dismissedByUser), for: .touchUpInside)
        dim.frame = bounds
        dim.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        addSubview(dim)

        panel.backgroundColor = .systemGroupedBackground
        panel.translatesAutoresizingMaskIntoConstraints = false
        addSubview(panel)
        // Off screen until opened.
        panelLeading = panel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: -Self.width)
        NSLayoutConstraint.activate([
            panelLeading,
            panel.topAnchor.constraint(equalTo: topAnchor),
            panel.bottomAnchor.constraint(equalTo: bottomAnchor),
            panel.widthAnchor.constraint(equalToConstant: Self.width),
        ])

        table.dataSource = self
        table.delegate = self
        table.backgroundColor = .clear
        table.register(UITableViewCell.self, forCellReuseIdentifier: Self.cell)
        table.translatesAutoresizingMaskIntoConstraints = false
        panel.addSubview(table)
        NSLayoutConstraint.activate([
            table.leadingAnchor.constraint(equalTo: panel.leadingAnchor),
            table.trailingAnchor.constraint(equalTo: panel.trailingAnchor),
            table.topAnchor.constraint(equalTo: panel.safeAreaLayoutGuide.topAnchor),
            table.bottomAnchor.constraint(equalTo: panel.bottomAnchor),
        ])

        let swipe = UISwipeGestureRecognizer(target: self, action: #selector(dismissedByUser))
        swipe.direction = .left
        addGestureRecognizer(swipe)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    private static let width: CGFloat = 300

    func configure(labels: [String], icons: [String], selected: Int) {
        self.labels = labels
        self.icons = icons
        self.selected = selected
        table.reloadData()
    }

    /// Slides the panel in or out. Does not report anything: the caller
    /// knows why it asked.
    func setOpen(_ open: Bool, animated: Bool) {
        guard open != isOpen else { return }
        isOpen = open
        if open {
            isHidden = false
            layoutIfNeeded()
        }
        panelLeading.constant = open ? 0 : -Self.width
        let changes = {
            self.dim.alpha = open ? 1 : 0
            self.layoutIfNeeded()
        }
        let finished: (Bool) -> Void = { _ in
            // A newer request may have reopened it meanwhile.
            if !self.isOpen {
                self.isHidden = true
            }
        }
        if animated {
            UIView.animate(
                withDuration: 0.25,
                delay: 0,
                options: [.curveEaseOut, .beginFromCurrentState],
                animations: changes,
                completion: finished
            )
        } else {
            changes()
            finished(true)
        }
        // Keeps VoiceOver inside the open drawer.
        accessibilityViewIsModal = open
    }

    @objc private func dismissedByUser() {
        guard isOpen else { return }
        setOpen(false, animated: true)
        onClose?()
    }

    // An accessibility escape (two-finger scrub) closes it too.
    override func accessibilityPerformEscape() -> Bool {
        dismissedByUser()
        return true
    }

    func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
        labels.count
    }

    func tableView(_ tableView: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        let cell = tableView.dequeueReusableCell(withIdentifier: Self.cell, for: indexPath)
        var content = cell.defaultContentConfiguration()
        content.text = labels[indexPath.row]
        let icon = icons.indices.contains(indexPath.row) ? icons[indexPath.row] : ""
        if !icon.isEmpty {
            content.image = UIImage(systemName: icon) ?? UIImage(named: icon)
        }
        cell.contentConfiguration = content
        let isSelected = indexPath.row == selected
        cell.accessoryType = isSelected ? .checkmark : .none
        cell.accessibilityTraits = isSelected ? [.button, .selected] : .button
        return cell
    }

    func tableView(_ tableView: UITableView, didSelectRowAt indexPath: IndexPath) {
        tableView.deselectRow(at: indexPath, animated: true)
        onSelect?(indexPath.row)
    }
}
