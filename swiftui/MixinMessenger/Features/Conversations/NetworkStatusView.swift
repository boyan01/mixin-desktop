import SwiftUI

struct NetworkStatusView: View {
    @Environment(AccountSession.self) private var session

    var body: some View {
        VStack(spacing: 0) {
            if !session.connected, session.connectedBefore {
                HStack(spacing: 12) {
                    Image(systemName: "exclamationmark")
                        .font(.caption.bold())
                        .foregroundStyle(.white)
                        .frame(width: 20, height: 20)
                        .background(.orange, in: Circle())
                    Text("Network connection failed")
                    Spacer()
                    Button("Retry") {
                        session.retryConnection()
                    }
                    .buttonStyle(.link)
                }
                .padding(.horizontal, 22)
                .frame(height: 48)
                .background(.orange.opacity(0.16))
                .transition(.move(edge: .top).combined(with: .opacity))
            }
            if !session.connected {
                ProgressView()
                    .progressViewStyle(.linear)
                    .controlSize(.small)
                    .frame(height: 2)
            }
        }
        .animation(.easeOut(duration: 0.2), value: session.connected)
    }
}
