// @generated automatically by Diesel CLI.

diesel::table! {
    addresses (address_id) {
        address_id -> Text,
        #[sql_name = "type"]
        type_ -> Text,
        asset_id -> Text,
        destination -> Text,
        label -> Text,
        updated_at -> Timestamp,
        reserve -> Text,
        fee -> Text,
        tag -> Nullable<Text>,
        dust -> Nullable<Text>,
    }
}

diesel::table! {
    apps (app_id) {
        app_id -> Text,
        app_number -> Text,
        home_uri -> Text,
        redirect_uri -> Text,
        name -> Text,
        icon_url -> Text,
        category -> Nullable<Text>,
        description -> Text,
        app_secret -> Text,
        capabilities -> Nullable<Text>,
        creator_id -> Text,
        resource_patterns -> Nullable<Text>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    assets (asset_id) {
        asset_id -> Text,
        symbol -> Text,
        name -> Text,
        icon_url -> Text,
        balance -> Text,
        destination -> Text,
        tag -> Nullable<Text>,
        price_btc -> Text,
        price_usd -> Text,
        chain_id -> Text,
        change_usd -> Text,
        change_btc -> Text,
        confirmations -> Integer,
        asset_key -> Nullable<Text>,
        reserve -> Nullable<Text>,
    }
}

diesel::table! {
    chains (chain_id) {
        chain_id -> Text,
        name -> Text,
        symbol -> Text,
        icon_url -> Text,
        threshold -> Integer,
    }
}

diesel::table! {
    circle_conversations (conversation_id, circle_id) {
        conversation_id -> Text,
        circle_id -> Text,
        user_id -> Nullable<Text>,
        created_at -> Timestamp,
        pin_time -> Nullable<Timestamp>,
    }
}

diesel::table! {
    circles (circle_id) {
        circle_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        ordered_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    conversations (conversation_id) {
        conversation_id -> Text,
        owner_id -> Nullable<Text>,
        category -> Nullable<Text>,
        name -> Nullable<Text>,
        icon_url -> Nullable<Text>,
        announcement -> Nullable<Text>,
        code_url -> Nullable<Text>,
        pay_type -> Nullable<Text>,
        created_at -> Timestamp,
        pin_time -> Nullable<Timestamp>,
        last_message_id -> Nullable<Text>,
        last_message_created_at -> Nullable<Integer>,
        last_read_message_id -> Nullable<Text>,
        unseen_message_count -> Nullable<Integer>,
        status -> Integer,
        draft -> Nullable<Text>,
        mute_until -> Nullable<Timestamp>,
        expire_in -> Nullable<Timestamp>,
    }
}

diesel::table! {
    expired_messages (message_id) {
        message_id -> Text,
        expire_in -> Timestamp,
        expire_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    favorite_apps (app_id, user_id) {
        app_id -> Text,
        user_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    fiats (code) {
        code -> Text,
        rate -> Double,
    }
}

diesel::table! {
    flood_messages (message_id) {
        message_id -> Text,
        data -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    hyperlinks (hyperlink) {
        hyperlink -> Text,
        site_name -> Text,
        site_title -> Text,
        site_description -> Nullable<Text>,
        site_image -> Nullable<Text>,
    }
}

diesel::table! {
    inscription_collections (collection_hash) {
        collection_hash -> Text,
        supply -> Text,
        unit -> Text,
        symbol -> Text,
        name -> Text,
        icon_url -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    inscription_items (inscription_hash) {
        inscription_hash -> Text,
        collection_hash -> Text,
        sequence -> Integer,
        content_type -> Text,
        content_url -> Text,
        occupied_by -> Nullable<Text>,
        occupied_at -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    jobs (job_id) {
        job_id -> Text,
        action -> Text,
        created_at -> Timestamp,
        order_id -> Nullable<Integer>,
        priority -> Integer,
        user_id -> Nullable<Text>,
        blaze_message -> Nullable<Text>,
        conversation_id -> Nullable<Text>,
        resend_message_id -> Nullable<Text>,
        run_count -> Integer,
    }
}

diesel::table! {
    message_mentions (message_id) {
        message_id -> Text,
        conversation_id -> Text,
        has_read -> Nullable<Bool>,
    }
}

diesel::table! {
    messages (message_id) {
        message_id -> Text,
        conversation_id -> Text,
        user_id -> Text,
        category -> Text,
        content -> Nullable<Text>,
        media_url -> Nullable<Text>,
        media_mime_type -> Nullable<Text>,
        media_size -> Nullable<Integer>,
        media_duration -> Nullable<Text>,
        media_width -> Nullable<Integer>,
        media_height -> Nullable<Integer>,
        media_hash -> Nullable<Text>,
        thumb_image -> Nullable<Text>,
        media_key -> Nullable<Text>,
        media_digest -> Nullable<Text>,
        media_status -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamp,
        action -> Nullable<Text>,
        participant_id -> Nullable<Text>,
        snapshot_id -> Nullable<Text>,
        hyperlink -> Nullable<Text>,
        name -> Nullable<Text>,
        album_id -> Nullable<Text>,
        sticker_id -> Nullable<Text>,
        shared_user_id -> Nullable<Text>,
        media_waveform -> Nullable<Text>,
        quote_message_id -> Nullable<Text>,
        quote_content -> Nullable<Text>,
        thumb_url -> Nullable<Text>,
        caption -> Nullable<Text>,
    }
}

diesel::table! {
    messages_history (message_id) {
        message_id -> Text,
    }
}

diesel::table! {
    offsets (key) {
        key -> Text,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    participant_session (conversation_id, user_id, session_id) {
        conversation_id -> Text,
        user_id -> Text,
        session_id -> Text,
        sent_to_server -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
        public_key -> Nullable<Text>,
    }
}

diesel::table! {
    participants (conversation_id, user_id) {
        conversation_id -> Text,
        user_id -> Text,
        role -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    pin_messages (message_id) {
        message_id -> Text,
        conversation_id -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    properties (key, group) {
        key -> Text,
        group -> Text,
        value -> Text,
    }
}

diesel::table! {
    resend_session_messages (message_id, user_id, session_id) {
        message_id -> Text,
        user_id -> Text,
        session_id -> Text,
        status -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    safe_snapshots (snapshot_id) {
        snapshot_id -> Text,
        #[sql_name = "type"]
        type_ -> Text,
        asset_id -> Text,
        amount -> Text,
        user_id -> Text,
        opponent_id -> Text,
        memo -> Text,
        transaction_hash -> Text,
        created_at -> Timestamp,
        trace_id -> Nullable<Text>,
        confirmations -> Nullable<Integer>,
        opening_balance -> Nullable<Text>,
        closing_balance -> Nullable<Text>,
        withdrawal -> Nullable<Text>,
        deposit -> Nullable<Text>,
        inscription_hash -> Nullable<Text>,
    }
}

diesel::table! {
    sent_session_sender_keys (conversation_id, user_id, session_id) {
        conversation_id -> Text,
        user_id -> Text,
        session_id -> Text,
        sent_to_server -> Integer,
        sender_key_id -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    snapshots (snapshot_id) {
        snapshot_id -> Text,
        trace_id -> Nullable<Text>,
        #[sql_name = "type"]
        type_ -> Text,
        asset_id -> Text,
        amount -> Text,
        created_at -> Integer,
        opponent_id -> Nullable<Text>,
        transaction_hash -> Nullable<Text>,
        sender -> Nullable<Text>,
        receiver -> Nullable<Text>,
        memo -> Nullable<Text>,
        confirmations -> Nullable<Integer>,
        snapshot_hash -> Nullable<Text>,
        opening_balance -> Nullable<Text>,
        closing_balance -> Nullable<Text>,
    }
}

diesel::table! {
    sticker_albums (album_id) {
        album_id -> Text,
        name -> Text,
        icon_url -> Text,
        created_at -> Integer,
        update_at -> Integer,
        ordered_at -> Integer,
        user_id -> Text,
        category -> Text,
        description -> Text,
        banner -> Nullable<Text>,
        added -> Nullable<Bool>,
        is_verified -> Bool,
    }
}

diesel::table! {
    sticker_relationships (album_id, sticker_id) {
        album_id -> Text,
        sticker_id -> Text,
    }
}

diesel::table! {
    stickers (sticker_id) {
        sticker_id -> Text,
        album_id -> Nullable<Text>,
        name -> Text,
        asset_url -> Text,
        asset_type -> Text,
        asset_width -> Integer,
        asset_height -> Integer,
        created_at -> Integer,
        last_use_at -> Nullable<Integer>,
    }
}

diesel::table! {
    tokens (asset_id) {
        asset_id -> Text,
        kernel_asset_id -> Text,
        symbol -> Text,
        name -> Text,
        icon_url -> Text,
        price_btc -> Text,
        price_usd -> Text,
        chain_id -> Text,
        change_usd -> Text,
        change_btc -> Text,
        confirmations -> Integer,
        asset_key -> Text,
        dust -> Text,
        collection_hash -> Nullable<Text>,
    }
}

diesel::table! {
    transcript_messages (transcript_id, message_id) {
        transcript_id -> Text,
        message_id -> Text,
        user_id -> Nullable<Text>,
        user_full_name -> Nullable<Text>,
        category -> Text,
        created_at -> Timestamp,
        content -> Nullable<Text>,
        media_url -> Nullable<Text>,
        media_name -> Nullable<Text>,
        media_size -> Nullable<Integer>,
        media_width -> Nullable<Integer>,
        media_height -> Nullable<Integer>,
        media_mime_type -> Nullable<Text>,
        media_duration -> Nullable<Text>,
        media_status -> Nullable<Text>,
        media_waveform -> Nullable<Text>,
        thumb_image -> Nullable<Text>,
        thumb_url -> Nullable<Text>,
        media_key -> Nullable<Text>,
        media_digest -> Nullable<Text>,
        media_created_at -> Nullable<Integer>,
        sticker_id -> Nullable<Text>,
        shared_user_id -> Nullable<Text>,
        mentions -> Nullable<Text>,
        quote_id -> Nullable<Text>,
        quote_content -> Nullable<Text>,
        caption -> Nullable<Text>,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Text,
        identity_number -> Text,
        relationship -> Nullable<Text>,
        full_name -> Nullable<Text>,
        avatar_url -> Nullable<Text>,
        phone -> Nullable<Text>,
        is_verified -> Nullable<Bool>,
        created_at -> Nullable<Timestamp>,
        mute_until -> Nullable<Timestamp>,
        has_pin -> Nullable<Integer>,
        app_id -> Nullable<Text>,
        biography -> Nullable<Text>,
        is_scam -> Nullable<Integer>,
        code_url -> Nullable<Text>,
        code_id -> Nullable<Text>,
        is_deactivated -> Nullable<Bool>,
    }
}

diesel::joinable!(messages -> conversations (conversation_id));
diesel::joinable!(participants -> conversations (conversation_id));

diesel::allow_tables_to_appear_in_same_query!(
    addresses,
    apps,
    assets,
    chains,
    circle_conversations,
    circles,
    conversations,
    expired_messages,
    favorite_apps,
    fiats,
    flood_messages,
    hyperlinks,
    inscription_collections,
    inscription_items,
    jobs,
    message_mentions,
    messages,
    messages_history,
    offsets,
    participant_session,
    participants,
    pin_messages,
    properties,
    resend_session_messages,
    safe_snapshots,
    sent_session_sender_keys,
    snapshots,
    sticker_albums,
    sticker_relationships,
    stickers,
    tokens,
    transcript_messages,
    users,
);
