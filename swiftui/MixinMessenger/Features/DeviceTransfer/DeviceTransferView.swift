import SwiftUI

struct DeviceTransferCoordinatorView<Content: View>: View {
    @Bindable var controller: DeviceTransferController
    let content: Content

    init(
        controller: DeviceTransferController,
        @ViewBuilder content: () -> Content
    ) {
        self.controller = controller
        self.content = content()
    }

    var body: some View {
        content
            .sheet(
                isPresented: Binding(
                    get: { controller.sheetMode != nil },
                    set: { presented in
                        if !presented {
                            controller.dismissPresentedSheet()
                        }
                    }
                )
            ) {
                switch controller.sheetMode {
                case .setup:
                    DeviceTransferSetupView(controller: controller)
                case let .progress(direction):
                    DeviceTransferProgressView(
                        controller: controller,
                        direction: direction
                    )
                    .interactiveDismissDisabled()
                case nil:
                    EmptyView()
                }
            }
            .alert(item: Binding(
                get: { controller.alert },
                set: { value in
                    if value == nil {
                        controller.dismissAlert()
                    }
                }
            )) { alert in
                switch alert.kind {
                case .approval:
                    Alert(
                        title: Text(alert.title),
                        message: Text(alert.message),
                        primaryButton: .default(Text("Confirm")) {
                            controller.respondToApproval(approved: true)
                        },
                        secondaryButton: .cancel {
                            controller.respondToApproval(approved: false)
                        }
                    )
                case .notice:
                    Alert(
                        title: Text(alert.title),
                        message: Text(alert.message),
                        dismissButton: .default(Text("OK")) {
                            controller.dismissAlert()
                        }
                    )
                }
            }
    }
}

private struct DeviceTransferSetupView: View {
    @Bindable var controller: DeviceTransferController

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            page
                .frame(minHeight: 300)
        }
        .frame(width: 440)
    }

    private var header: some View {
        HStack {
            if controller.setupPage != .choices {
                Button {
                    controller.goBack()
                } label: {
                    Image(systemName: "chevron.left")
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Back")
            } else {
                Color.clear.frame(width: 16, height: 16)
            }

            Spacer()
            Text(title)
                .font(.headline)
            Spacer()

            Button {
                controller.dismissSetup()
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Close")
        }
        .padding(18)
    }

    @ViewBuilder
    private var page: some View {
        switch controller.setupPage {
        case .choices:
            choices
        case let .explanation(direction):
            explanation(direction)
        case let .waiting(direction):
            waiting(direction)
        }
    }

    private var title: String {
        switch controller.setupPage {
        case .choices:
            "Chat Backup and Restore"
        case let .explanation(direction), let .waiting(direction):
            direction.title
        }
    }

    private var choices: some View {
        VStack(spacing: 14) {
            transferChoice(
                direction: .restore,
                description: "Receive chat history from another signed-in device."
            )
            transferChoice(
                direction: .backup,
                description: "Send this Mac's chat history to another signed-in device."
            )
        }
        .padding(24)
    }

    private func transferChoice(
        direction: DeviceTransferDirection,
        description: String
    ) -> some View {
        Button {
            controller.showExplanation(direction)
        } label: {
            HStack(spacing: 16) {
                Image(systemName: direction.symbolName)
                    .font(.title2)
                    .frame(width: 36)
                VStack(alignment: .leading, spacing: 4) {
                    Text(direction.title)
                        .font(.headline)
                    Text(description)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.leading)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.tertiary)
            }
            .padding(16)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
    }

    private func explanation(_ direction: DeviceTransferDirection) -> some View {
        VStack(spacing: 22) {
            Image(systemName: "arrow.left.arrow.right.circle")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text(explanationText(direction))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 340)

            Button(direction == .restore ? "Restore Chats" : "Back Up Chats") {
                controller.begin(direction)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .disabled(controller.commandInFlight)
        }
        .padding(32)
    }

    private func waiting(_ direction: DeviceTransferDirection) -> some View {
        VStack(spacing: 22) {
            ProgressView()
                .controlSize(.large)

            Text("Waiting for the other device to connect…")
                .font(.headline)

            Text(waitingText(direction))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 340)

            Button("Cancel", role: .cancel) {
                controller.dismissSetup()
            }
        }
        .padding(32)
    }

    private func explanationText(_ direction: DeviceTransferDirection) -> String {
        switch direction {
        case .restore:
            "Keep both devices online. Existing chats on this Mac will be updated with the history sent by your other device."
        case .backup:
            "Keep both devices online. This Mac will prepare its chat history and securely send it to your other device."
        }
    }

    private func waitingText(_ direction: DeviceTransferDirection) -> String {
        switch direction {
        case .restore:
            "On the other device, choose to sync chats to this Mac and approve the request."
        case .backup:
            "On the other device, choose to sync chats from this Mac and approve the request."
        }
    }
}

private struct DeviceTransferProgressView: View {
    @Bindable var controller: DeviceTransferController
    let direction: DeviceTransferDirection

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: direction.symbolName)
                .font(.system(size: 58))
                .foregroundStyle(.secondary)

            Text("Transferring Chats")
                .font(.title3.weight(.semibold))

            Text(String(format: "%.2f%%", controller.progress))
                .font(.system(.title2, design: .rounded).monospacedDigit())

            ProgressView(value: controller.progress, total: 100)
                .frame(width: 320)

            Text(
                DeviceTransferController.formatNetworkSpeed(
                    controller.bytesPerSecond
                )
            )
            .font(.subheadline.monospacedDigit())
            .foregroundStyle(.secondary)

            Text("Keep Mixin open and both devices connected until the transfer finishes.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 340)

            Button("Cancel Transfer", role: .cancel) {
                controller.cancelTransfer(direction)
            }
        }
        .padding(40)
        .frame(width: 440)
    }
}
