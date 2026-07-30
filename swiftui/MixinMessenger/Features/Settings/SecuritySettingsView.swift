import SwiftUI

struct SecuritySettingsView: View {
  @Environment(AccountSession.self) private var session
  @Environment(\.mixinTheme) private var theme
  @State private var presentsPasscodeSetup = false
  @State private var operationError: String?

  private var security: SecurityService {
    session.security
  }

  var body: some View {
    AppScrollView {
      VStack(spacing: 0) {
        Spacer().frame(height: 40)
        securityGroup {
          VStack(spacing: 0) {
            securitySwitchCell("Screen Passcode", value: Binding(
              get: { security.hasPasscode }, set: togglePasscode
            ))
            if security.hasPasscode {
              Menu {
                ForEach(SecurityService.supportedAutoLockMinutes, id: \.self) { minutes in
                  Button(autoLockLabel(minutes)) { setAutoLock(minutes) }
                }
              } label: {
                HStack(spacing: 4) {
                  Text("Auto Lock")
                    .font(.system(size: 16))
                    .foregroundStyle(theme.text)
                  Spacer(minLength: 4)
                  Text(autoLockLabel(security.autoLockMinutes))
                    .font(.system(size: 14))
                    .foregroundStyle(theme.secondaryText)
                  Image("SettingsArrow")
                    .resizable()
                    .renderingMode(.template)
                    .foregroundStyle(theme.secondaryText)
                    .frame(width: 30, height: 30)
                }
                .padding(.leading, 16)
                .padding(.trailing, 10)
                .padding(.vertical, 17)
                .contentShape(Rectangle())
              }
              .buttonStyle(.plain)
            }
          }
        }
        if security.hasPasscode {
          securityGroup {
            securitySwitchCell("Biometric", value: Binding(
              get: { security.biometricEnabled }, set: setBiometric
            ))
          }
        }
        if let operationError {
          securityGroup {
            Text(operationError)
              .font(.system(size: 14))
              .foregroundStyle(theme.destructive)
              .padding(.horizontal, 16)
              .padding(.vertical, 17)
          }
        }
      }
    }
    .background(theme.background)
    .navigationTitle("Security")
    .sheet(isPresented: $presentsPasscodeSetup) {
      SetPasscodeSheet { passcode in
        Task {
          do {
            try await security.setPasscode(passcode)
          } catch {
            operationError = MixinErrorPresenter.message(for: error)
          }
        }
      }
    }
    .alert(
      "Unable to Change Security Settings",
      isPresented: Binding(
        get: { operationError != nil },
        set: { if !$0 { operationError = nil } }
      )
    ) {
      Button("OK") {
        operationError = nil
      }
    } message: {
      Text(operationError ?? "")
    }
  }

  private func securityGroup<Content: View>(
    @ViewBuilder content: () -> Content
  ) -> some View {
    content()
      .background(theme.settingCellBackground)
      .clipShape(RoundedRectangle(cornerRadius: 8))
      .padding(.horizontal, 10)
      .padding(.bottom, 10)
      .frame(maxWidth: 600)
  }

  private func securitySwitchCell(_ title: String, value: Binding<Bool>) -> some View {
    HStack {
      Text(title)
        .font(.system(size: 16))
        .foregroundStyle(theme.text)
      Spacer(minLength: 4)
      Toggle(title, isOn: value)
        .labelsHidden()
        .scaleEffect(0.7)
    }
    .padding(.leading, 16)
    .padding(.trailing, 10)
    .padding(.vertical, 17)
  }

  private func togglePasscode(_ enabled: Bool) {
    if enabled {
      presentsPasscodeSetup = true
      return
    }
    Task {
      do {
        try await security.setPasscode(nil)
      } catch {
        operationError = MixinErrorPresenter.message(for: error)
      }
    }
  }

  private func setBiometric(_ enabled: Bool) {
    Task {
      do {
        try await security.setBiometricEnabled(enabled)
      } catch {
        operationError = MixinErrorPresenter.message(for: error)
      }
    }
  }

  private func setAutoLock(_ minutes: Int) {
    Task {
      do {
        try await security.setAutoLockMinutes(minutes)
      } catch {
        operationError = MixinErrorPresenter.message(for: error)
      }
    }
  }

  private func autoLockLabel(_ minutes: Int) -> String {
    switch minutes {
    case 0:
      "Disabled"
    case 1:
      "1 minute"
    case 5:
      "5 minutes"
    case 60:
      "1 hour"
    case 300:
      "5 hours"
    default:
      "\(minutes) minutes"
    }
  }
}

private struct SetPasscodeSheet: View {
  @Environment(\.dismiss) private var dismiss
  @Environment(MixinNoticeCenter.self) private var noticeCenter
  let onSave: (String) -> Void
  @State private var firstPasscode: String?
  @State private var passcode = ""
  @FocusState private var passcodeFocused: Bool

  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Spacer()
        Button {
          dismiss()
        } label: {
          Image(systemName: "xmark")
        }
        .buttonStyle(MixinActionButtonStyle())
        .accessibilityLabel("Close")
      }
      .padding(.top, 12)
      .padding(.trailing, 12)

      Text(
        firstPasscode == nil
          ? "Set a passcode to unlock Mixin Messenger"
          : "Enter again to confirm the passcode"
      )
      .font(.system(size: 18, weight: .semibold))
      .multilineTextAlignment(.center)

      SixDigitPasscodeField(
        value: $passcode,
        isFocused: $passcodeFocused
      ) { value in
        submit(value)
      }
      .frame(width: 215)
      .padding(.top, 40)
    }
    .padding(.top, 20)
    .padding(.bottom, 80)
    .frame(width: 430)
    .onAppear {
      passcodeFocused = true
    }
    .onChange(of: passcodeFocused) {
      if !passcodeFocused {
        passcodeFocused = true
      }
    }
  }

  private func submit(_ value: String) {
    if let firstPasscode {
      guard value == firstPasscode else {
        self.firstPasscode = nil
        passcode = ""
        noticeCenter.show("Passcode incorrect", kind: .failure)
        passcodeFocused = true
        return
      }
      dismiss()
      onSave(value)
    } else {
      firstPasscode = value
      passcode = ""
      passcodeFocused = true
    }
  }
}
