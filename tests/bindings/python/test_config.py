"""Tests for default_options() and default_config()."""

from trafilatura_uniffi import (
    ExtractionFocus,
    HtmlDateMode,
    default_options,
    default_config,
)


def test_default_config_values():
    # Defaults from trafilatura::Config::default() — update if core defaults change
    config = default_config()
    assert config.min_extracted_size == 250
    assert config.min_extracted_comment_size == 1
    assert config.min_output_size == 1
    assert config.min_output_comment_size == 1


def test_default_options_values():
    opts = default_options()
    assert opts.original_url is None
    assert opts.target_language is None
    assert opts.enable_fallback is False
    assert opts.focus == ExtractionFocus.BALANCED
    assert opts.exclude_comments is False
    assert opts.exclude_tables is False
    assert opts.include_images is False
    assert opts.include_links is False
    assert opts.deduplicate is False
    assert opts.has_essential_metadata is False
    assert opts.max_tree_size is None
    assert opts.prune_selector is None
    assert opts.html_date_mode == HtmlDateMode.DEFAULT
    assert opts.html_date_override is None
