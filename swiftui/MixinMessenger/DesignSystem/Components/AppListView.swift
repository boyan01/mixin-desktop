import SwiftUI

enum AppListSelectionDisabled: Hashable {}

struct AppListView<SelectionValue, Content>: View
where SelectionValue: Hashable, Content: View {
  private let selection: Binding<SelectionValue?>?
  private let content: Content

  init(
    selection: Binding<SelectionValue?>,
    @ViewBuilder content: () -> Content
  ) {
    self.selection = selection
    self.content = content()
  }

  var body: some View {
    Group {
      if let selection {
        List(selection: selection) {
          content
        }
      } else {
        List {
          content
        }
      }
    }
    .appListScrollStyle()
  }
}

extension AppListView where SelectionValue == AppListSelectionDisabled {
  init(@ViewBuilder content: () -> Content) {
    self.selection = nil
    self.content = content()
  }

  init<Data, RowContent>(
    _ data: Data,
    @ViewBuilder rowContent: @escaping (Data.Element) -> RowContent
  )
  where
    Content == ForEach<Data, Data.Element.ID, RowContent>,
    Data: RandomAccessCollection,
    Data.Element: Identifiable,
    RowContent: View
  {
    self.init {
      ForEach(data, content: rowContent)
    }
  }

  init<Data, ID, RowContent>(
    _ data: Data,
    id: KeyPath<Data.Element, ID>,
    @ViewBuilder rowContent: @escaping (Data.Element) -> RowContent
  )
  where
    Content == ForEach<Data, ID, RowContent>,
    Data: RandomAccessCollection,
    ID: Hashable,
    RowContent: View
  {
    self.init {
      ForEach(data, id: id, content: rowContent)
    }
  }
}

extension AppListView {
  init<Data, RowContent>(
    _ data: Data,
    selection: Binding<SelectionValue?>,
    @ViewBuilder rowContent: @escaping (Data.Element) -> RowContent
  )
  where
    Content == ForEach<Data, Data.Element.ID, RowContent>,
    Data: RandomAccessCollection,
    Data.Element: Identifiable,
    RowContent: View
  {
    self.init(selection: selection) {
      ForEach(data, content: rowContent)
    }
  }
}
