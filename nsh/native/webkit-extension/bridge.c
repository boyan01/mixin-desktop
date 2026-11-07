/* Native provenance for the app-owned WebKitGTK 6 browser. */
#include <webkit/webkit-web-process-extension.h>
#include <jsc/jsc.h>
#include <string.h>

#define DOCUMENT_KEY "mixin-bot-native-document"
#define CONTEXT_KEY "mixin-bot-native-context"
#define MAX_MESSAGE_BYTES (1024 * 1024)

typedef struct {
    GWeakRef page;
    GWeakRef frame;
    GWeakRef context;
    guint64 token;
    guint64 generation;
    GQueue queued;
    gsize queued_bytes;
} Document;

typedef struct { char *uri; char *body; } QueuedMessage;

static void queued_free(gpointer data)
{
    QueuedMessage *message = data;
    g_free(message->uri); g_free(message->body); g_free(message);
}

static guint64 next_document;
static char *process_epoch;

static void document_free(gpointer data)
{
    Document *document = data;
    WebKitWebPage *page = g_weak_ref_get(&document->page);
    if (page) {
        if (g_object_get_data(G_OBJECT(page), DOCUMENT_KEY) == document)
            g_object_set_data(G_OBJECT(page), DOCUMENT_KEY, NULL);
        g_object_unref(page);
    }
    g_weak_ref_clear(&document->page);
    g_weak_ref_clear(&document->frame);
    g_weak_ref_clear(&document->context);
    g_queue_clear_full(&document->queued, queued_free);
    g_free(document);
}

/* A Frame can survive navigation. Its current context must still be the exact
 * context which owns this native function; Frame identity alone is insufficient. */
static gboolean document_current(Document *document, WebKitWebPage **page_out,
                                 WebKitFrame **frame_out, JSCContext **context_out)
{
    WebKitWebPage *page = g_weak_ref_get(&document->page);
    WebKitFrame *frame = g_weak_ref_get(&document->frame);
    JSCContext *context = g_weak_ref_get(&document->context);
    JSCContext *current = frame ? webkit_frame_get_js_context_for_script_world(
        frame, webkit_script_world_get_default()) : NULL;
    gboolean valid = page && frame && context && current == context
        && webkit_frame_is_main_frame(frame)
        && g_object_get_data(G_OBJECT(page), DOCUMENT_KEY) == document;
    g_clear_object(&current);
    if (!valid) { g_clear_object(&page); g_clear_object(&frame); g_clear_object(&context); return FALSE; }
    *page_out = page; *frame_out = frame; *context_out = context;
    return TRUE;
}

static void send_to_view(WebKitWebPage *page, const char *name, GVariant *parameters)
{
    WebKitUserMessage *message = g_object_ref_sink(webkit_user_message_new(name, parameters));
    webkit_web_page_send_message_to_view(page, message, NULL, NULL, NULL);
    g_object_unref(message);
}

static void post(const char *body, gpointer data)
{
    Document *document = data;
    if (!body || strnlen(body, MAX_MESSAGE_BYTES + 1) > MAX_MESSAGE_BYTES) return;
    WebKitWebPage *page; WebKitFrame *frame; JSCContext *context;
    if (!document_current(document, &page, &frame, &context)) return;
    const char *uri = webkit_frame_get_uri(frame);
    if (uri && (g_str_has_prefix(uri, "https://") || g_str_has_prefix(uri, "http://"))) {
        if (document->generation) send_to_view(page, "mixin-bot-message", g_variant_new("(ststs)", process_epoch, document->token, uri, document->generation, body));
        else if (g_queue_get_length(&document->queued) < 32 && document->queued_bytes + strlen(body) <= MAX_MESSAGE_BYTES) {
            QueuedMessage *queued = g_new0(QueuedMessage, 1);
            queued->uri = g_strdup(uri); queued->body = g_strdup(body);
            g_queue_push_tail(&document->queued, queued); document->queued_bytes += strlen(body);
        }
    }
    g_object_unref(context); g_object_unref(frame); g_object_unref(page);
}

static gboolean probe(WebKitWebPage *page, WebKitUserMessage *message, gpointer unused)
{
    (void)unused;
    if (g_strcmp0(webkit_user_message_get_name(message), "mixin-bot-probe")) return FALSE;
    GVariant *parameters = webkit_user_message_get_parameters(message);
    Document *document = g_object_get_data(G_OBJECT(page), DOCUMENT_KEY);
    WebKitWebPage *current_page; WebKitFrame *frame; JSCContext *context;
    if (document && parameters && g_variant_is_of_type(parameters, G_VARIANT_TYPE_UINT64)
        && document_current(document, &current_page, &frame, &context)) {
        guint64 generation = g_variant_get_uint64(parameters);
        if (generation >= document->generation) {
            document->generation = generation;
            const char *uri = webkit_frame_get_uri(frame);
            /* Repeat the native announcement in the same IPC stream as this ACK.
             * UI load-start notification and the first announcement can interleave. */
            send_to_view(page, "mixin-bot-document", g_variant_new("(st)", process_epoch, document->token));
            if (uri) send_to_view(page, "mixin-bot-ready", g_variant_new("(stst)", process_epoch, document->token, uri, document->generation));
            while (!g_queue_is_empty(&document->queued)) {
                QueuedMessage *queued = g_queue_pop_head(&document->queued);
                if (uri && !g_strcmp0(uri, queued->uri)) send_to_view(page, "mixin-bot-message", g_variant_new("(ststs)", process_epoch, document->token, queued->uri, document->generation, queued->body));
                queued_free(queued);
            }
            document->queued_bytes = 0;
        }
        g_object_unref(context); g_object_unref(frame); g_object_unref(current_page);
    }
    WebKitUserMessage *reply = g_object_ref_sink(webkit_user_message_new("mixin-bot-probe-done", NULL));
    webkit_user_message_send_reply(message, reply); g_object_unref(reply);
    return TRUE;
}

static void page_created(WebKitWebProcessExtension *extension, WebKitWebPage *page, gpointer unused)
{
    (void)extension; (void)unused;
    g_signal_connect(page, "user-message-received", G_CALLBACK(probe), NULL);
}

static void window_cleared(WebKitScriptWorld *world, WebKitWebPage *page, WebKitFrame *frame, gpointer unused)
{
    (void)unused;
    if (!webkit_frame_is_main_frame(frame)) return;
    JSCContext *context = webkit_frame_get_js_context_for_script_world(frame, world);
    if (!context) return;
    Document *document = g_new0(Document, 1);
    g_weak_ref_init(&document->page, page); g_weak_ref_init(&document->frame, frame); g_weak_ref_init(&document->context, context);
    document->token = ++next_document;
    g_object_set_data(G_OBJECT(page), DOCUMENT_KEY, document);
    /* The getter returns an owned wrapper; JS's global function does not retain
     * that GObject. The page owns only its current context, while Document stays
     * weak to avoid a Context -> JS function -> Document -> Context cycle. Set
     * the new Document first so disposal of the prior context cannot clear it. */
    g_object_set_data_full(G_OBJECT(page), CONTEXT_KEY, g_object_ref(context), g_object_unref);
    JSCValue *function = jsc_value_new_function(context, "__mixinNativePost", G_CALLBACK(post), document, document_free, G_TYPE_NONE, 1, G_TYPE_STRING);
    JSCValue *global = jsc_context_get_global_object(context);
    jsc_value_object_define_property_data(global, "__mixinNativePost", 0, function);
    g_object_unref(global); g_object_unref(function);
    send_to_view(page, "mixin-bot-document", g_variant_new("(st)", process_epoch, document->token));
    g_object_unref(context);
}

G_MODULE_EXPORT void webkit_web_process_extension_initialize_with_user_data(WebKitWebProcessExtension *extension, const GVariant *data)
{
    /* GLib's type/get APIs take non-const pointers but do not mutate the value. */
    GVariant *initialization = (GVariant *)data;
    if (!initialization || !g_variant_is_of_type(initialization, G_VARIANT_TYPE_UINT32) || g_variant_get_uint32(initialization) != 1) return;
    process_epoch = g_uuid_string_random();
    g_signal_connect(extension, "page-created", G_CALLBACK(page_created), NULL);
    g_signal_connect(webkit_script_world_get_default(), "window-object-cleared", G_CALLBACK(window_cleared), NULL);
}
