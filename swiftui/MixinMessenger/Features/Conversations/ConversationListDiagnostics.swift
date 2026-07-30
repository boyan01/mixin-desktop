import Foundation

enum ConversationListDiagnostics {
    static func rowHover(
        conversationID: String,
        hovering: Bool,
        selectedConversationID: String?
    ) {
        AppLogger.debug(
            "ConversationList row_hover id=\(conversationID) hovering=\(hovering) selected=\(selectedConversationID ?? "nil")"
        )
    }

    static func rowActivated(
        conversationID: String,
        selectedConversationID: String?
    ) {
        AppLogger.debug(
            "ConversationList row_activate id=\(conversationID) selected_before=\(selectedConversationID ?? "nil")"
        )
    }

    static func selectionWillChange(
        from currentConversationID: String?,
        to conversationID: String
    ) {
        AppLogger.debug(
            "ConversationList selection_will_change from=\(currentConversationID ?? "nil") to=\(conversationID)"
        )
    }

    static func selectionDidChange(
        selectedConversationID: String?,
        route: String
    ) {
        AppLogger.debug(
            "ConversationList selection_did_change selected=\(selectedConversationID ?? "nil") route=\(route)"
        )
    }

}
