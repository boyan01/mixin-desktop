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
            page
        }
        .frame(maxWidth: 400)
        .background(Color.clear)
    }

    private var header: some View {
        HStack(spacing: 0) {
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
                .font(.system(size: 16, weight: .semibold))
            Spacer()

            Button {
                controller.dismissSetup()
            } label: {
                Image(systemName: "xmark")
                    .frame(width: 40, height: 40)
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Close")
        }
        .frame(height: 64)
        .padding(.horizontal, 8)
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
        case let .explanation(direction):
            direction.title
        case let .waiting(direction):
            direction == .restore
                ? "restore from other device"
                : "backup to other device"
        }
    }

    private var choices: some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 20)
            transferChoice(
                direction: .restore
            )
            Spacer().frame(height: 16)
            transferChoice(direction: .backup)
            Spacer().frame(height: 32)
        }
    }

    private func transferChoice(
        direction: DeviceTransferDirection
    ) -> some View {
        Button {
            controller.showExplanation(direction)
        } label: {
            HStack {
                Text(direction == .restore
                    ? "sync from other device"
                    : "sync to other device")
                    .font(.system(size: 16))
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.tertiary)
            }
            .padding(.leading, 16)
            .padding(.trailing, 10)
            .padding(.vertical, 17)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
        .padding(.horizontal, 20)
    }

    private func explanation(_ direction: DeviceTransferDirection) -> some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 32)
            Image("DeviceTransfer")
                .resizable()
                .frame(width: 74, height: 87)
            Spacer().frame(height: 20)
            Text(explanationText(direction))
                .font(.system(size: 14))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 36)
            Spacer().frame(height: 40)
            Button(direction == .restore ? "restore chat" : "backup chat") {
                controller.begin(direction)
            }
            .buttonStyle(.plain)
            .font(.system(size: 16))
            .foregroundStyle(Color.accentColor)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.leading, 16)
            .padding(.vertical, 17)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
            .padding(.horizontal, 16)
            .disabled(controller.commandInFlight)
            Spacer().frame(height: 40)
        }
    }

    private func waiting(_ direction: DeviceTransferDirection) -> some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 32)
            Image(
                direction == .restore
                    ? "TransferFromPhone"
                    : "TransferToPhone"
            )
                .resizable()
                .frame(
                    width: direction == .restore ? 72 : 69,
                    height: direction == .restore ? 72 : 70
                )
            Spacer().frame(height: 20)
            Text(waitingText(direction))
                .font(.system(size: 14))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 36)
            Spacer().frame(height: 40)
            Button("Cancel", role: .cancel) {
                controller.dismissSetup()
            }
            Spacer().frame(height: 40)
        }
    }

    private func explanationText(_ direction: DeviceTransferDirection) -> String {
        switch direction {
        case .restore:
            "restore chat tip"
        case .backup:
            "tips for backup to other device"
        }
    }

    private func waitingText(_ direction: DeviceTransferDirection) -> String {
        switch direction {
        case .restore:
            "waiting other device connection"
        case .backup:
            "restore waiting other device"
        }
    }
}

private struct DeviceTransferProgressView: View {
  @Bindable var controller: DeviceTransferController
  let direction: DeviceTransferDirection
  @Environment(\.mixinTheme) private var theme

  var body: some View {
        VStack(spacing: 0) {
            Spacer().frame(height: 40)
            Image(
                direction == .restore
                    ? "TransferFromPhone"
                    : "TransferToPhone"
            )
                .resizable()
                .renderingMode(.template)
                .foregroundStyle(theme.secondaryText)
                .frame(
                    width: direction == .restore ? 72 : 69,
                    height: direction == .restore ? 72 : 70
                )

            Spacer().frame(height: 38)
            HStack(spacing: 2) {
                Text("Transferring Chats")
                if controller.progress > 0 {
                    Text(String(format: "(%.2f%%)", controller.progress))
                }
            }
            .font(.system(size: 18))
            .foregroundStyle(theme.text)

            Spacer().frame(height: 16)
            Text(
                "Please do not turn off the screen and keep the Mixin running "
                    + "in the foreground while syncing."
            )
                .font(.system(size: 14))
                .foregroundStyle(theme.secondaryText)
                .multilineTextAlignment(.center)
            Spacer().frame(height: 18)
            Text(
                DeviceTransferController.formatNetworkSpeed(
                    controller.bytesPerSecond
                )
            )
            .font(.system(size: 14))
            .foregroundStyle(theme.secondaryText)

            Spacer().frame(height: 32)
            Button("Cancel", role: .cancel) {
                controller.cancelTransfer(direction)
            }
            .font(.system(size: 14))
            .foregroundStyle(theme.accent)
        }
        .padding(.horizontal, 40)
        .padding(.vertical, 20)
        .frame(width: 420)
    }
}
