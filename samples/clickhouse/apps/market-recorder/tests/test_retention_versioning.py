"""PLAN §5: temporary policy changes are versioned, not mutated.

"A changed mapping requires a new explicit migration or table, not silent
reinterpretation of old events." So editing a table's retention has to mint a
**new** policy id carrying the new values, while the superseded policy keeps
describing what already-written rows used.

Mutating the policy in place would be wrong twice over: it reinterprets history,
and it would not even take effect for that table's rows, because the ingester
freezes a layout→storage binding on first sight and deliberately never
recomputes it. That is why the actor *re-declares* the table on a retention
change — the new declaration gets a new policy id and a new layout, and the
ingester therefore forms a binding that carries the new values.

Driven through the real actor rather than the session, because the
re-declaration is the actor's job.
"""

from __future__ import annotations

from pathlib import Path

from market_recorder.actor import RecordingActor


def _config(default_row_ttl: str, rule_row_ttl: str = "24h") -> str:
    """A config whose *default* row retention is the knob under test.

    Note which knob that is. `config::resolve` returns `defaults` for a rule
    that is not enabled — the `Some(r) if r.enabled` arm is the only one that
    reads the rule's own retention — so for a disabled temporary table (which
    `book_debug` is, deliberately) the effective retention comes from
    `defaults`, and editing the rule's `row_ttl` changes nothing. Getting this
    wrong is how the first version of this test passed a no-op edit and
    reported the versioning as broken.
    """
    return f"""api_version: recording/v1
defaults:
  temporary_enabled: false
  row_ttl: {default_row_ttl}
  idle_table_ttl: 7d
rules:
  - process: market-recorder
    instance: "*"
    table: book_debug
    enabled: false
    row_ttl: {rule_row_ttl}
    idle_table_ttl: 7d
"""


def test_a_retention_change_is_versioned_not_mutated(tmp_path: Path) -> None:
    cfg = tmp_path / "recording.yaml"
    cfg.write_text(_config("24h"))

    actor = RecordingActor("binance", "retention-0")
    # `poll_config` reads this, so pointing it at a temp file exercises the
    # same path the in-cluster watcher drives.
    actor.config_path = cfg

    actor.poll_config()
    first = actor.session.declared_policy_count()
    assert first > 0, "the first poll declares the table's policy"

    # Re-resolving the same values is not a change: nothing should version.
    actor.poll_config()
    assert actor.session.declared_policy_count() == first, (
        "an unchanged config must not declare another policy"
    )

    # A retention edit is a change, and versioning means a NEW policy rather
    # than an in-place edit of the old one.
    cfg.write_text(_config("1h"))
    actor.poll_config()
    assert actor.session.declared_policy_count() > first, (
        "a retention edit must mint a new policy, not rewrite the existing one"
    )
