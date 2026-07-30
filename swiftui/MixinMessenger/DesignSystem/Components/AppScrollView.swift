import SwiftUI

struct AppScrollView<Content: View>: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.mixinTheme) private var theme

    private let axes: Axis.Set
    private let content: Content
    private let externalScrollPosition: Binding<ScrollPosition>?
    private let onVerticalOffset: ((CGFloat) -> Void)?
    private let showsIndicator: Bool
    private let thumbVisibility: Bool

    @State private var verticalMetrics = AppScrollMetrics.zero
    @State private var horizontalMetrics = AppScrollMetrics.zero
    @State private var scrollPosition = ScrollPosition()
    @State private var indicatorHovered = false
    @State private var indicatorDragging = false
    @State private var indicatorActive = false
    @State private var hideTask: Task<Void, Never>?

    init(
        _ axes: Axis.Set = .vertical,
        scrollPosition: Binding<ScrollPosition>? = nil,
        showsIndicator: Bool = true,
        thumbVisibility: Bool = false,
        onVerticalOffset: ((CGFloat) -> Void)? = nil,
        @ViewBuilder content: () -> Content
    ) {
        self.axes = axes
        externalScrollPosition = scrollPosition
        self.showsIndicator = showsIndicator
        self.thumbVisibility = thumbVisibility
        self.onVerticalOffset = onVerticalOffset
        self.content = content()
    }

    var body: some View {
        ScrollView(axes, showsIndicators: false) {
            content
        }
        .scrollPosition(activeScrollPosition)
        .onScrollGeometryChange(for: AppScrollMetricsPair.self) { geometry in
            AppScrollMetricsPair(
                vertical: AppScrollMetrics(
                    offset: geometry.contentOffset.y,
                    contentLength: geometry.contentSize.height,
                    viewportLength: geometry.containerSize.height
                ),
                horizontal: AppScrollMetrics(
                    offset: geometry.contentOffset.x,
                    contentLength: geometry.contentSize.width,
                    viewportLength: geometry.containerSize.width
                )
            )
        } action: { oldMetrics, newMetrics in
            verticalMetrics = newMetrics.vertical
            horizontalMetrics = newMetrics.horizontal
            onVerticalOffset?(newMetrics.vertical.offset)
            if oldMetrics != newMetrics {
                noteScrollActivity()
            }
        }
        .onScrollPhaseChange { _, newPhase in
            if newPhase != .idle {
                noteScrollActivity()
            }
        }
        .overlay(alignment: .trailing) {
            if showsIndicator, axes.contains(.vertical) {
                indicator(axis: .vertical, metrics: verticalMetrics)
            }
        }
        .overlay(alignment: .bottom) {
            if showsIndicator, axes.contains(.horizontal) {
                indicator(axis: .horizontal, metrics: horizontalMetrics)
            }
        }
        .onDisappear {
            hideTask?.cancel()
        }
    }

    private var activeScrollPosition: Binding<ScrollPosition> {
        externalScrollPosition ?? $scrollPosition
    }

    private func indicator(axis: Axis.Set, metrics: AppScrollMetrics) -> some View {
        AppScrollIndicator(
            axis: axis,
            metrics: metrics,
            color: theme.icon,
            visible: indicatorVisible(for: metrics),
            reduceMotion: reduceMotion,
            hovered: indicatorHovered,
            dragging: indicatorDragging,
            onHover: { hovering in
                indicatorHovered = hovering
            },
            onDrag: { dragging in
                indicatorDragging = dragging
                if dragging {
                    revealIndicator()
                } else {
                    noteScrollActivity()
                }
            },
            onScroll: { offset, animated in
                scrollTo(offset, axis: axis, animated: animated)
            }
        )
    }

    private func indicatorVisible(for metrics: AppScrollMetrics) -> Bool {
        guard metrics.canScroll else {
            return false
        }
        return thumbVisibility || indicatorActive || indicatorHovered || indicatorDragging
    }

    private func revealIndicator() {
        hideTask?.cancel()
        indicatorActive = true
    }

    private func noteScrollActivity() {
        revealIndicator()
        hideTask = Task {
            try? await Task.sleep(for: .milliseconds(600))
            guard !Task.isCancelled else {
                return
            }
            indicatorActive = false
        }
    }

    private func scrollTo(_ offset: CGFloat, axis: Axis.Set, animated: Bool) {
        let point = axis == .vertical
            ? CGPoint(x: 0, y: offset)
            : CGPoint(x: offset, y: 0)
        if animated, !reduceMotion {
            withAnimation(.easeOut(duration: 0.15)) {
                activeScrollPosition.wrappedValue.scrollTo(point: point)
            }
        } else {
            activeScrollPosition.wrappedValue.scrollTo(point: point)
        }
        noteScrollActivity()
    }
}

private struct AppScrollIndicator: View {
    let axis: Axis.Set
    let metrics: AppScrollMetrics
    let color: Color
    let visible: Bool
    let reduceMotion: Bool
    let hovered: Bool
    let dragging: Bool
    let onHover: (Bool) -> Void
    let onDrag: (Bool) -> Void
    let onScroll: (CGFloat, Bool) -> Void

    @State private var dragStartThumbOffset: CGFloat?
    @State private var trackPressLocation: CGFloat?

    var body: some View {
        GeometryReader { proxy in
            let trackLength =
                axis == .vertical
                ? proxy.size.height - 4
                : proxy.size.width - 4
            let geometry = metrics.indicatorGeometry(
                trackLength: max(trackLength, 0)
            )

            ZStack(alignment: .topLeading) {
                Capsule()
                    .fill(color.opacity(hovered || dragging ? 0.62 : 0.5))
                    .frame(
                        width: axis == .vertical ? thickness : geometry.length,
                        height: axis == .vertical ? geometry.length : thickness
                    )
                    .offset(
                        x: axis == .vertical ? 2 : geometry.offset + 2,
                        y: axis == .vertical ? geometry.offset + 2 : 2
                    )
                    .opacity(visible ? 1 : 0)
                    .animation(
                        reduceMotion ? nil : .easeOut(duration: 0.3),
                        value: visible
                    )
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .animation(.easeOut(duration: 0.2), value: hovered || dragging)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        handleGestureChanged(
                            value,
                            geometry: geometry,
                            trackLength: max(trackLength, 0)
                        )
                    }
                    .onEnded { value in
                        handleGestureEnded(
                            value,
                            geometry: geometry
                        )
                    }
            )
        }
        .frame(
            width: axis == .vertical ? 16 : nil,
            height: axis == .vertical ? nil : 16
        )
        .padding(axis == .vertical ? .trailing : .bottom, 1)
        .contentShape(Rectangle())
        .onHover(perform: onHover)
        .accessibilityHidden(true)
    }

    private var thickness: CGFloat {
        8
    }

    private func handleGestureChanged(
        _ value: DragGesture.Value,
        geometry: (length: CGFloat, offset: CGFloat),
        trackLength: CGFloat
    ) {
        let startLocation = mainAxisValue(value.startLocation) - 2
        if dragStartThumbOffset == nil, trackPressLocation == nil {
            if startLocation >= geometry.offset,
                startLocation <= geometry.offset + geometry.length
            {
                dragStartThumbOffset = geometry.offset
                onDrag(true)
            } else {
                trackPressLocation = startLocation
            }
        }

        guard let dragStartThumbOffset else {
            return
        }
        let translation = mainAxisValue(value.translation)
        let thumbOffset = dragStartThumbOffset + translation
        onScroll(
            metrics.contentOffset(
                forThumbOffset: thumbOffset,
                trackLength: trackLength
            ),
            false
        )
    }

    private func handleGestureEnded(
        _ value: DragGesture.Value,
        geometry: (length: CGFloat, offset: CGFloat)
    ) {
        if dragStartThumbOffset != nil {
            onDrag(false)
        } else if trackPressLocation != nil {
            let location = mainAxisValue(value.location) - 2
            if location < geometry.offset {
                onScroll(metrics.pageOffset(direction: -1), true)
            } else if location > geometry.offset + geometry.length {
                onScroll(metrics.pageOffset(direction: 1), true)
            }
        }
        dragStartThumbOffset = nil
        trackPressLocation = nil
    }

    private func mainAxisValue(_ point: CGPoint) -> CGFloat {
        axis == .vertical ? point.y : point.x
    }

    private func mainAxisValue(_ size: CGSize) -> CGFloat {
        axis == .vertical ? size.height : size.width
    }
}

private struct AppScrollMetrics: Equatable {
    static let zero = AppScrollMetrics(
        offset: 0,
        contentLength: 0,
        viewportLength: 0
    )

    let offset: CGFloat
    let contentLength: CGFloat
    let viewportLength: CGFloat

    var canScroll: Bool {
        contentLength - viewportLength > 0.5
    }

    func indicatorGeometry(
        trackLength: CGFloat
    ) -> (length: CGFloat, offset: CGFloat) {
        guard canScroll, trackLength > 0 else {
            return (trackLength, 0)
        }

        let maximumOffset = contentLength - viewportLength
        let viewportFraction = min(viewportLength / contentLength, 1)
        let minimumThumbLength: CGFloat = 24
        let thumbLength = min(
            trackLength,
            max(minimumThumbLength, trackLength * viewportFraction)
        )
        let travel = max(trackLength - thumbLength, 0)

        let leadingOverscroll = max(-offset, 0)
        let trailingOverscroll = max(offset - maximumOffset, 0)
        if leadingOverscroll > 0 || trailingOverscroll > 0 {
            let overscroll = max(leadingOverscroll, trailingOverscroll)
            let compressedLength = max(minimumThumbLength, thumbLength - overscroll)
            return (
                compressedLength,
                leadingOverscroll > 0
                    ? 0
                    : max(trackLength - compressedLength, 0)
            )
        }

        let clampedOffset = min(max(offset, 0), maximumOffset)
        return (
            thumbLength,
            maximumOffset > 0 ? travel * clampedOffset / maximumOffset : 0
        )
    }

    func contentOffset(
        forThumbOffset thumbOffset: CGFloat,
        trackLength: CGFloat
    ) -> CGFloat {
        let geometry = indicatorGeometry(trackLength: trackLength)
        let travel = max(trackLength - geometry.length, 0)
        guard travel > 0 else {
            return 0
        }
        return min(max(thumbOffset, 0), travel) / travel
            * (contentLength - viewportLength)
    }

    func pageOffset(direction: CGFloat) -> CGFloat {
        let maximumOffset = max(contentLength - viewportLength, 0)
        return min(
            max(offset + direction * viewportLength, 0),
            maximumOffset
        )
    }
}

private struct AppScrollMetricsPair: Equatable {
    let vertical: AppScrollMetrics
    let horizontal: AppScrollMetrics
}
