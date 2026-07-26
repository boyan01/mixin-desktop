enum HomeSection: Hashable {
    case chats
    case contacts
    case groups
    case bots
    case strangers
    case circle(String)
    case settings

    var conversationFilter: (category: String, circleID: String?) {
        switch self {
        case .chats:
            ("chats", nil)
        case .contacts:
            ("contacts", nil)
        case .groups:
            ("groups", nil)
        case .bots:
            ("bots", nil)
        case .strangers:
            ("strangers", nil)
        case let .circle(circleID):
            ("chats", circleID)
        case .settings:
            ("chats", nil)
        }
    }
}
