import SwiftUI

struct SecuritySettingsView: View {
  @Environment(AccountSession.self) private var session
  @State private var presentsPasscodeSetup = false
  @State private var operationError: String?

  private var security: SecurityService {
    session.security
  }

  var body: some View {
    Form {
      Section {
        Toggle(
          "Screen Passcode",
          isOn: Binding(
            get: { security.hasPasscode },
            set: togglePasscode
          )
        )

        if security.hasPasscode {
          Picker(
            "Auto-Lock",
            selection: Binding(
              get: { security.autoLockMinutes },
              set: setAutoLock
            )
          ) {
            ForEach(SecurityService.supportedAutoLockMinutes, id: \.self) { minutes in
              Text(autoLockLabel(minutes))
                .tag(minutes)
            }
          }
        }
      } footer: {
        Text(
          security.hasPasscode
            ? "Mixin locks after it remains inactive for the selected duration."
            : "Require a six-digit passcode to open your Mixin conversations."
        )
      }

      if security.hasPasscode {
        Section {
          Toggle(
            "Touch ID",
            isOn: Binding(
              get: { security.biometricEnabled },
              set: setBiometric
            )
          )
        } footer: {
          Text("Use Touch ID to unlock Mixin on this Mac.")
        }
      }
    }
    .formStyle(.grouped)
    .settingsFormLayout()
    .navigationTitle("Security")
    .sheet(isPresented: $presentsPasscodeSetup) {
      SetPasscodeSheet { passcode in
        try await security.setPasscode(passcode)
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
      "Never"
    case 1:
      "In 1 minute"
    case 5:
      "In 5 minutes"
    case 60:
      "In 1 hour"
    case 300:
      "In 5 hours"
    default:
      "\(minutes) minutes"
    }
  }
}

private struct SetPasscodeSheet: View {
  @Environment(\.dismiss) private var dismiss
  let onSave: (String) async throws -> Void
  @State private var firstPasscode: String?
  @State private var passcode = ""
  @State private var error: String?
  @State private var saving = false
  @FocusState private var passcodeFocused: Bool

  var body: some View {
    VStack(spacing: 0) {
      HStack {
        Spacer()
        Button {
          dismiss()
        } label: {
          Image(systemName: "xmark.circle.fill")
            .font(.title2)
            .foregroundStyle(.secondary)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Close")
      }

      Image(systemName: "lock.circle")
        .font(.system(size: 54))
        .foregroundStyle(.secondary)
        .padding(.top, 8)

      Text(
        firstPasscode == nil
          ? "Enter a new six-digit passcode"
          : "Enter the passcode again"
      )
      .font(.title3.weight(.semibold))
      .padding(.top, 18)

      SixDigitPasscodeField(
        value: $passcode,
        isFocused: $passcodeFocused
      ) { value in
        submit(value)
      }
      .disabled(saving)
      .padding(.top, 32)

      Text(error ?? " ")
        .foregroundStyle(.red)
        .padding(.top, 16)

      if saving {
        ProgressView()
          .controlSize(.small)
          .padding(.top, 8)
      }
    }
    .padding(24)
    .padding(.bottom, 44)
    .frame(width: 430)
    .onAppear {
      passcodeFocused = true
    }
    .onChange(of: passcodeFocused) {
      if !passcodeFocused, !saving {
        passcodeFocused = true
      }
    }
  }

  private func submit(_ value: String) {
    guard !saving else {
      return
    }
    if let firstPasscode {
      guard value == firstPasscode else {
        self.firstPasscode = nil
        passcode = ""
        error = "Passcodes did not match. Start again."
        passcodeFocused = true
        return
      }
      saving = true
      error = nil
      Task {
        do {
          try await onSave(value)
          dismiss()
        } catch {
          self.error = MixinErrorPresenter.message(for: error)
          passcode = ""
          self.firstPasscode = nil
          saving = false
          passcodeFocused = true
        }
      }
    } else {
      firstPasscode = value
      passcode = ""
      error = nil
      passcodeFocused = true
    }
  }
}
