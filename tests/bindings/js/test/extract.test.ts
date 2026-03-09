import { describe, it, expect } from 'vitest';
import {
  Trafilatura,
  TrafilaturaError,
  type ExtractionFocus,
  type HtmlDateMode,
} from '../lib/trafilatura.js';

const {
  extractSimple,
  extract,
  defaultOptions,
  defaultConfig,
  createReadableDocument,
} = Trafilatura;

const articleHtml = `
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
</html>`;

describe('extractSimple', () => {
  it('basic extraction', () => {
    const result = extractSimple(articleHtml);
    expect(result.contentText).toContain('first paragraph');
    expect(result.contentText.length).toBeGreaterThan(0);
  });
});

describe('extract', () => {
  it('default options matches extractSimple', () => {
    const a = extractSimple(articleHtml);
    const b = extract(articleHtml, defaultOptions());
    expect(a.contentText).toBe(b.contentText);
  });

  it('with focus', () => {
    const opts = defaultOptions();
    opts.focus = 'FavorRecall' satisfies ExtractionFocus;
    const result = extract(articleHtml, opts);
    expect(result.contentText.length).toBeGreaterThan(0);
  });

  it('exclude comments', () => {
    const opts = defaultOptions();
    opts.excludeComments = true;
    const result = extract(articleHtml, opts);
    expect(result.commentsText).toBe('');
  });
});

describe('validation', () => {
  it('valid url', () => {
    const opts = defaultOptions();
    opts.originalUrl = 'https://example.com/article';
    const result = extract(articleHtml, opts);
    expect(result.contentText.length).toBeGreaterThan(0);
  });

  it('invalid url throws TrafilaturaError', () => {
    const opts = defaultOptions();
    opts.originalUrl = 'not a url';
    expect(() => extract(articleHtml, opts)).toThrow(TrafilaturaError);
  });

  it('invalid url error has correct variant', () => {
    const opts = defaultOptions();
    opts.originalUrl = 'not a url';
    try {
      extract(articleHtml, opts);
      expect.unreachable('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(TrafilaturaError);
      const err = e as TrafilaturaError;
      expect(err.variant.tag).toBe('ParseError');
    }
  });

  it('invalid date throws TrafilaturaError', () => {
    const opts = defaultOptions();
    opts.htmlDateOverride = '01/01/2024';
    expect(() => extract(articleHtml, opts)).toThrow(TrafilaturaError);
  });
});

describe('metadata', () => {
  it('metadata extraction', () => {
    const result = extractSimple(articleHtml);
    expect(result.metadata.title).toBe('Test Article Title');
  });
});

describe('HTML output', () => {
  it('content html present', () => {
    const result = extractSimple(articleHtml);
    expect(result.contentHtml.length).toBeGreaterThan(0);
    expect(result.contentHtml).toContain('<');
  });
});

describe('createReadableDocument', () => {
  it('creates readable document', () => {
    const result = extractSimple(articleHtml);
    const doc = createReadableDocument(result);
    expect(doc).toContain('<html');
    expect(doc).toContain('content-body');
  });
});

describe('defaultOptions', () => {
  it('default option values', () => {
    const opts = defaultOptions();
    expect(opts.originalUrl).toBeNull();
    expect(opts.targetLanguage).toBeNull();
    expect(opts.enableFallback).toBe(false);
    expect(opts.focus).toBe('Balanced' satisfies ExtractionFocus);
    expect(opts.excludeComments).toBe(false);
    expect(opts.excludeTables).toBe(false);
    expect(opts.includeImages).toBe(false);
    expect(opts.includeLinks).toBe(false);
    expect(opts.deduplicate).toBe(false);
    expect(opts.requireEssentialMetadata).toBe(false);
    expect(opts.maxTreeSize).toBeNull();
    expect(opts.pruneSelector).toBeNull();
    expect(opts.htmlDateMode).toBe('Automatic' satisfies HtmlDateMode);
    expect(opts.htmlDateOverride).toBeNull();
  });
});

describe('defaultConfig', () => {
  it('default config values', () => {
    const config = defaultConfig();
    expect(config.minExtractedSize).toBe(250);
    expect(config.minExtractedCommentSize).toBe(1);
    expect(config.minOutputSize).toBe(1);
    expect(config.minOutputCommentSize).toBe(1);
  });
});
