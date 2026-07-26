import AppKit
import SwiftUI

struct LogViewerSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var lines: [String] = []
    @State private var loading = true
    @State private var error: String?
    let desktop: SwiftDesktopHandle

    var body: some View {
        NavigationStack {
            Group {
                if loading {
                    ProgressView("Loading logs…")
                } else if let error {
                    ContentUnavailableView(
                        "Unable to Load Logs",
                        systemImage: "exclamationmark.triangle",
                        description: Text(error)
                    )
                } else if lines.isEmpty {
                    ContentUnavailableView(
                        "No Logs",
                        systemImage: "doc.text.magnifyingglass"
                    )
                } else {
                    ScrollViewReader { proxy in
                        ScrollView {
                            LazyVStack(alignment: .leading, spacing: 3) {
                                ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                                    Text(line)
                                        .font(.system(.caption, design: .monospaced))
                                        .textSelection(.enabled)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .id(index)
                                }
                            }
                            .padding()
                        }
                        .onAppear {
                            if let last = lines.indices.last {
                                proxy.scrollTo(last, anchor: .bottom)
                            }
                        }
                    }
                }
            }
            .navigationTitle("Logs")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") {
                        dismiss()
                    }
                }
                ToolbarItem {
                    Button {
                        Task {
                            await load()
                        }
                    } label: {
                        Label("Refresh", systemImage: "arrow.clockwise")
                    }
                }
                ToolbarItem {
                    Button {
                        openDirectory()
                    } label: {
                        Label("Open Directory", systemImage: "folder")
                    }
                }
            }
            .task {
                await load()
            }
        }
        .frame(minWidth: 760, minHeight: 560)
    }

    private func load() async {
        loading = true
        error = nil
        do {
            let directory = try desktop.logDirectory()
            let urls = try FileManager.default.contentsOfDirectory(
                at: URL(fileURLWithPath: directory),
                includingPropertiesForKeys: [.contentModificationDateKey],
                options: [.skipsHiddenFiles]
            )
            let logFiles = urls.filter { $0.pathExtension == "log" }
            let latest = try logFiles.max {
                let first = try $0.resourceValues(forKeys: [.contentModificationDateKey])
                    .contentModificationDate ?? .distantPast
                let second = try $1.resourceValues(forKeys: [.contentModificationDateKey])
                    .contentModificationDate ?? .distantPast
                return first < second
            }
            if let latest {
                let contents = try String(contentsOf: latest, encoding: .utf8)
                lines = contents.components(separatedBy: .newlines)
            } else {
                lines = []
            }
        } catch {
            self.error = MixinErrorPresenter.message(for: error)
        }
        loading = false
    }

    private func openDirectory() {
        guard let path = try? desktop.logDirectory() else {
            return
        }
        NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: path)
    }
}
