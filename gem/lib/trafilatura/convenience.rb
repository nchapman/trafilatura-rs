# frozen_string_literal: true

module Trafilatura
  FOCUS_MAP = {
    balanced: ExtractionFocus::BALANCED,
    favor_recall: ExtractionFocus::FAVOR_RECALL,
    favor_precision: ExtractionFocus::FAVOR_PRECISION,
  }.freeze

  DATE_MODE_MAP = {
    automatic: HtmlDateMode::AUTOMATIC,
    fast: HtmlDateMode::FAST,
    extensive: HtmlDateMode::EXTENSIVE,
    disabled: HtmlDateMode::DISABLED,
  }.freeze

  KNOWN_OPTIONS = %i[
    config original_url target_language enable_fallback focus
    exclude_comments exclude_tables include_images include_links
    deduplicate require_essential_metadata max_tree_size
    prune_selector html_date_mode html_date_override
  ].freeze

  # Extract with keyword arguments instead of constructing ExtractionOptions manually.
  # Only pass the options you want to change — everything else uses defaults.
  #
  #   result = Trafilatura.extract_with(html, focus: :favor_recall, include_links: true)
  #
  def self.extract_with(html, **overrides)
    unknown = overrides.keys - KNOWN_OPTIONS
    raise ArgumentError, "Unknown extraction options: #{unknown.inspect}" unless unknown.empty?

    if overrides[:focus].is_a?(Symbol)
      overrides[:focus] = FOCUS_MAP.fetch(overrides[:focus])
    end
    if overrides[:html_date_mode].is_a?(Symbol)
      overrides[:html_date_mode] = DATE_MODE_MAP.fetch(overrides[:html_date_mode])
    end

    defaults = default_options
    opts = ExtractionOptions.new(
      config:                     overrides.fetch(:config, defaults.config),
      original_url:               overrides.fetch(:original_url, defaults.original_url),
      target_language:            overrides.fetch(:target_language, defaults.target_language),
      enable_fallback:            overrides.fetch(:enable_fallback, defaults.enable_fallback),
      focus:                      overrides.fetch(:focus, defaults.focus),
      exclude_comments:           overrides.fetch(:exclude_comments, defaults.exclude_comments),
      exclude_tables:             overrides.fetch(:exclude_tables, defaults.exclude_tables),
      include_images:             overrides.fetch(:include_images, defaults.include_images),
      include_links:              overrides.fetch(:include_links, defaults.include_links),
      deduplicate:                overrides.fetch(:deduplicate, defaults.deduplicate),
      require_essential_metadata: overrides.fetch(:require_essential_metadata, defaults.require_essential_metadata),
      max_tree_size:              overrides.fetch(:max_tree_size, defaults.max_tree_size),
      prune_selector:             overrides.fetch(:prune_selector, defaults.prune_selector),
      html_date_mode:             overrides.fetch(:html_date_mode, defaults.html_date_mode),
      html_date_override:         overrides.fetch(:html_date_override, defaults.html_date_override)
    )
    extract(html, opts)
  end
end
