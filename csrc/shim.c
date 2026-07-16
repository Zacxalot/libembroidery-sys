/* Thin read-side accessors for embroidermaker's importer.
 *
 * EmbPattern is large and its exact layout (EmbSettings/EmbHoop/etc.) is awkward
 * to mirror in Rust. These shims hand back the linked-list heads so the Rust side
 * can walk `next` in O(n) instead of paying the O(n^2) of embStitchList_getAt. */
#include "emb-pattern.h"

EmbStitchList* emb_pattern_stitch_list(EmbPattern* p) { return p->stitchList; }
EmbThreadList* emb_pattern_thread_list(EmbPattern* p) { return p->threadList; }
