import AVFoundation
import UIKit

/// The views this application makes itself, for the `NativeView` nodes of
/// the showcase: a star rating, which the DSL has no widget for, and a video.
enum NativeViews {
    static func register() {
        KrabikNativeViews.register("rating") { emit in RatingView(emit: emit) }
        KrabikNativeViews.register("video") { emit in VideoView(emit: emit) }
    }
}

/// Five stars; a tap on one sets the rating and reports it.
private final class RatingView: NSObject, KrabikNativeView {
    private let stack = UIStackView()
    private let emit: (String) -> Void

    var view: UIView { stack }

    init(emit: @escaping (String) -> Void) {
        self.emit = emit
        super.init()
        stack.axis = .horizontal
        stack.distribution = .fillEqually
        for index in 1...5 {
            let star = UIButton(type: .system)
            star.tag = index
            star.accessibilityLabel = "\(index) stars"
            star.addTarget(self, action: #selector(tapped(_:)), for: .touchUpInside)
            stack.addArrangedSubview(star)
        }
    }

    func update(_ props: [String: String]) {
        let stars = Int(props["stars"] ?? "") ?? 0
        for case let star as UIButton in stack.arrangedSubviews {
            let name = star.tag <= stars ? "star.fill" : "star"
            star.setImage(UIImage(systemName: name), for: .normal)
        }
    }

    @objc private func tapped(_ sender: UIButton) {
        emit(String(sender.tag))
    }
}

/// A looping video from an address.
private final class VideoView: NSObject, KrabikNativeView {
    private final class LayerView: UIView {
        override class var layerClass: AnyClass { AVPlayerLayer.self }
    }

    private let surface = LayerView()
    private let player = AVQueuePlayer()
    private var looper: AVPlayerLooper?
    private var shown: String?
    private let emit: (String) -> Void

    var view: UIView { surface }

    init(emit: @escaping (String) -> Void) {
        self.emit = emit
        super.init()
        (surface.layer as? AVPlayerLayer)?.player = player
        surface.backgroundColor = .black
        player.isMuted = true
    }

    func update(_ props: [String: String]) {
        let address = props["url"] ?? ""
        guard address != shown, let url = URL(string: address) else { return }
        shown = address
        looper = AVPlayerLooper(player: player, templateItem: AVPlayerItem(url: url))
        player.play()
        emit("playing")
    }
}
