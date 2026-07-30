import AppKit

final class AppOverlayScroller: NSScroller {
    override class var isCompatibleWithOverlayScrollers: Bool {
        true
    }

    override func draw(_ dirtyRect: NSRect) {
        drawKnob()
    }

    override func drawKnob() {
        guard knobProportion < 1 else {
            return
        }
        let systemKnobRect = rect(for: .knob)
        let knobWidth: CGFloat = 3
        let knobRect = NSRect(
            x: bounds.maxX - knobWidth - 2,
            y: systemKnobRect.minY + 2,
            width: knobWidth,
            height: max(systemKnobRect.height - 4, knobWidth)
        )
        NSColor.secondaryLabelColor.withAlphaComponent(0.55).setFill()
        NSBezierPath(
            roundedRect: knobRect,
            xRadius: knobWidth / 2,
            yRadius: knobWidth / 2
        ).fill()
    }
}
