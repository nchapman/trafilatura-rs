# frozen_string_literal: true

require_relative "test_helper"

ARTICLE_HTML = <<~HTML
  <html>
  <head>
    <title>Test Article Title</title>
    <meta name="author" content="Jane Doe">
    <meta property="og:description" content="A test article about web extraction.">
    <meta property="og:site_name" content="TestSite">
  </head>
  <body>
    <nav><a href="/">Home</a> | <a href="/about">About</a></nav>
    <article>
      <h1>Test Article Title</h1>
      <p>This is the first paragraph of the article. It contains enough text to
      exceed the minimum extraction thresholds that the trafilatura algorithm uses
      to determine whether content is substantial enough to be considered an article.
      The algorithm analyzes DOM structure and text density to find main content.</p>
      <p>The second paragraph adds more content to ensure the algorithm can properly
      identify this as the main content of the page. Trafilatura uses multiple
      extraction strategies including readability and justext as fallbacks to ensure
      robust content extraction across different page structures and layouts.</p>
      <p>A third paragraph further strengthens the signal that this is genuine article
      content rather than boilerplate navigation or sidebar material. The algorithm
      compares candidate nodes and selects the one with the highest content score,
      while preserving the original structure of the text content.</p>
      <p>Finally, a fourth paragraph ensures we are well above the default minimum
      extraction size of two hundred and fifty characters, making this a reliable
      test fixture for the extraction algorithm across all three language bindings
      used in this test suite.</p>
    </article>
    <footer>Copyright 2024</footer>
  </body>
  </html>
HTML

class TestExtractSimple < Minitest::Test
  def test_basic_extraction
    result = Trafilatura.extract_simple(ARTICLE_HTML)
    assert_includes result.content_text, "first paragraph"
    refute_empty result.content_text
  end
end

class TestExtract < Minitest::Test
  def test_default_options_matches_extract_simple
    a = Trafilatura.extract_simple(ARTICLE_HTML)
    b = Trafilatura.extract(ARTICLE_HTML, Trafilatura.default_options)
    assert_equal a.content_text, b.content_text
  end

  def test_with_focus
    opts = Trafilatura.default_options
    result = Trafilatura.extract(
      ARTICLE_HTML,
      Trafilatura::ExtractionOptions.new(
        config: opts.config,
        original_url: opts.original_url,
        target_language: opts.target_language,
        enable_fallback: opts.enable_fallback,
        focus: Trafilatura::ExtractionFocus::FAVOR_RECALL,
        exclude_comments: opts.exclude_comments,
        exclude_tables: opts.exclude_tables,
        include_images: opts.include_images,
        include_links: opts.include_links,
        deduplicate: opts.deduplicate,
        require_essential_metadata: opts.require_essential_metadata,
        max_tree_size: opts.max_tree_size,
        prune_selector: opts.prune_selector,
        html_date_mode: opts.html_date_mode,
        html_date_override: opts.html_date_override
      )
    )
    refute_empty result.content_text
  end

  def test_exclude_comments
    opts = Trafilatura.default_options
    result = Trafilatura.extract(
      ARTICLE_HTML,
      Trafilatura::ExtractionOptions.new(
        config: opts.config,
        original_url: opts.original_url,
        target_language: opts.target_language,
        enable_fallback: opts.enable_fallback,
        focus: opts.focus,
        exclude_comments: true,
        exclude_tables: opts.exclude_tables,
        include_images: opts.include_images,
        include_links: opts.include_links,
        deduplicate: opts.deduplicate,
        require_essential_metadata: opts.require_essential_metadata,
        max_tree_size: opts.max_tree_size,
        prune_selector: opts.prune_selector,
        html_date_mode: opts.html_date_mode,
        html_date_override: opts.html_date_override
      )
    )
    assert_equal "", result.comments_text
  end
end

class TestValidation < Minitest::Test
  def test_valid_url
    opts = Trafilatura.default_options
    result = Trafilatura.extract(
      ARTICLE_HTML,
      Trafilatura::ExtractionOptions.new(
        config: opts.config,
        original_url: "https://example.com/article",
        target_language: opts.target_language,
        enable_fallback: opts.enable_fallback,
        focus: opts.focus,
        exclude_comments: opts.exclude_comments,
        exclude_tables: opts.exclude_tables,
        include_images: opts.include_images,
        include_links: opts.include_links,
        deduplicate: opts.deduplicate,
        require_essential_metadata: opts.require_essential_metadata,
        max_tree_size: opts.max_tree_size,
        prune_selector: opts.prune_selector,
        html_date_mode: opts.html_date_mode,
        html_date_override: opts.html_date_override
      )
    )
    refute_empty result.content_text
  end

  def test_invalid_url_raises_parse_error
    opts = Trafilatura.default_options
    assert_raises(Trafilatura::TrafilaturaError::ParseError) do
      Trafilatura.extract(
        ARTICLE_HTML,
        Trafilatura::ExtractionOptions.new(
          config: opts.config,
          original_url: "not a url",
          target_language: opts.target_language,
          enable_fallback: opts.enable_fallback,
          focus: opts.focus,
          exclude_comments: opts.exclude_comments,
          exclude_tables: opts.exclude_tables,
          include_images: opts.include_images,
          include_links: opts.include_links,
          deduplicate: opts.deduplicate,
          require_essential_metadata: opts.require_essential_metadata,
          max_tree_size: opts.max_tree_size,
          prune_selector: opts.prune_selector,
          html_date_mode: opts.html_date_mode,
          html_date_override: opts.html_date_override
        )
      )
    end
  end

  def test_invalid_date_raises_parse_error
    opts = Trafilatura.default_options
    assert_raises(Trafilatura::TrafilaturaError::ParseError) do
      Trafilatura.extract(
        ARTICLE_HTML,
        Trafilatura::ExtractionOptions.new(
          config: opts.config,
          original_url: opts.original_url,
          target_language: opts.target_language,
          enable_fallback: opts.enable_fallback,
          focus: opts.focus,
          exclude_comments: opts.exclude_comments,
          exclude_tables: opts.exclude_tables,
          include_images: opts.include_images,
          include_links: opts.include_links,
          deduplicate: opts.deduplicate,
          require_essential_metadata: opts.require_essential_metadata,
          max_tree_size: opts.max_tree_size,
          prune_selector: opts.prune_selector,
          html_date_mode: opts.html_date_mode,
          html_date_override: "01/01/2024"
        )
      )
    end
  end
end

class TestMetadata < Minitest::Test
  def test_metadata_extraction
    result = Trafilatura.extract_simple(ARTICLE_HTML)
    assert_equal "Test Article Title", result.metadata.title
  end
end

class TestHtmlOutput < Minitest::Test
  def test_content_html_present
    result = Trafilatura.extract_simple(ARTICLE_HTML)
    refute_empty result.content_html
    assert_includes result.content_html, "<"
  end
end

class TestReadableDocument < Minitest::Test
  def test_create_readable_document
    result = Trafilatura.extract_simple(ARTICLE_HTML)
    doc = Trafilatura.create_readable_document(result)
    assert_includes doc, "<html"
    assert_includes doc, "content-body"
  end
end

class TestDefaultOptions < Minitest::Test
  def test_default_option_values
    opts = Trafilatura.default_options
    assert_nil opts.original_url
    assert_nil opts.target_language
    assert_equal false, opts.enable_fallback
    assert_equal Trafilatura::ExtractionFocus::BALANCED, opts.focus
    assert_equal false, opts.exclude_comments
    assert_equal false, opts.exclude_tables
    assert_equal false, opts.include_images
    assert_equal false, opts.include_links
    assert_equal false, opts.deduplicate
    assert_equal false, opts.require_essential_metadata
    assert_nil opts.max_tree_size
    assert_nil opts.prune_selector
    assert_equal Trafilatura::HtmlDateMode::AUTOMATIC, opts.html_date_mode
    assert_nil opts.html_date_override
  end
end

class TestDefaultConfig < Minitest::Test
  def test_default_config_values
    config = Trafilatura.default_config
    assert_equal 250, config.min_extracted_size
    assert_equal 1, config.min_extracted_comment_size
    assert_equal 1, config.min_output_size
    assert_equal 1, config.min_output_comment_size
  end
end
