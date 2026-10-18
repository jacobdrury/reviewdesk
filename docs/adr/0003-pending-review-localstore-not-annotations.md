# Pending review drafts in LocalStore (not in-memory Annotations)

First-slice **Annotations** were session-only in memory ([ADR-0001](0001-localstore-turso-local-first.md) consequences). For day-to-day Provider writeback we supersede that clause: the viewer’s **Pending review** and its draft **Review comments** live in **LocalStore**, write through to the Provider’s pending review on save, and clear after Submit or Discard. Conversation and the Diff surface share one **Thread** set; product language does not use Annotation for Provider-backed remarks.

**Status:** Accepted  
**Date:** 2026-10-18  
**Context:** [Grill review submit, threads, and suggestions model](https://github.com/jacobdrury/reviewdesk/issues/54)
