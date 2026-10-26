# ADR-0003: Chat Language Clarification

Date: 2026-10-26.
Status: accepted.
Basis: explicit user clarification.

## Context

ADR-0002 required English for development reports without distinguishing stored
artifacts from conversational replies.

## Decision

All created and stored project content remains English-only. Chat may use the user's
language. This supersedes only the conversational-report interpretation of ADR-0002.
External user data remains unchanged under the existing read-only integration rule.

## Alternatives

English-only conversation adds a restriction the user explicitly did not request.

## Consequences

Agent instructions distinguish chat from stored reports. No history rewrite is needed:
this is a new clarification, not a change to the English repository baseline.
