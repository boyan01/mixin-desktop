import SwiftUI

struct NetworkStatusView: View {
    @Environment(AccountSession.self) private var session
    @Environment(\.mixinTheme) private var theme

    var body: some View {
        VStack(spacing: 0) {
            if !session.connected, session.connectedBefore {
                HStack(spacing: 12) {
                    Image(systemName: "exclamationmark")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(.white)
                        .frame(width: 20, height: 20)
                        .background(theme.warning, in: Circle())
                    Text("Network connection failed")
                        .font(.system(size: 14))
                        .foregroundStyle(theme.text)
                    Spacer()
                    Button("Retry") {
                        session.retryConnection()
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 14))
                    .foregroundStyle(theme.accent)
                }
                .padding(.horizontal, 22)
                .padding(.vertical, 14)
                .background(theme.warning.opacity(0.2))
                .transition(.move(edge: .top).combined(with: .opacity))
            }
            if !session.connected {
                ProgressView()
                    .progressViewStyle(.linear)
                    .controlSize(.small)
                    .tint(theme.accent)
                    .frame(height: 2)
            }
        }
        .animation(.easeOut(duration: 0.2), value: session.connected)
    }
}
