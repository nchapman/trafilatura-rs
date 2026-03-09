import 'dart:io';

import 'package:test/test.dart';
import 'package:trafilatura_dart_test/trafilatura.dart';

const articleHtml = '''
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
''';

void main() {
  setUpAll(() {
    final projectRoot = Directory.current.path.replaceAll(
      RegExp(r'/tests/bindings/dart$'),
      '',
    );
    final libDir = '$projectRoot/uniffi/target/release';
    final String libPath;
    if (Platform.isMacOS) {
      libPath = '$libDir/libtrafilatura_uniffi.dylib';
    } else if (Platform.isLinux) {
      libPath = '$libDir/libtrafilatura_uniffi.so';
    } else {
      throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
    }
    configureDefaultBindings(libraryPath: libPath);
  });

  group('extractSimple', () {
    test('basic extraction', () {
      final result = extractSimple(articleHtml);
      expect(result.contentText, contains('first paragraph'));
      expect(result.contentText.isNotEmpty, isTrue);
    });
  });

  group('extract', () {
    test('default options matches extractSimple', () {
      final a = extractSimple(articleHtml);
      final b = extract(articleHtml, defaultOptions());
      expect(a.contentText, equals(b.contentText));
    });

    test('with focus', () {
      final opts = defaultOptions().copyWith(
        focus: ExtractionFocus.favorRecall,
      );
      final result = extract(articleHtml, opts);
      expect(result.contentText.isNotEmpty, isTrue);
    });

    test('exclude comments', () {
      final opts = defaultOptions().copyWith(excludeComments: true);
      final result = extract(articleHtml, opts);
      expect(result.commentsText, equals(''));
    });
  });

  group('validation', () {
    test('valid url', () {
      final opts = defaultOptions().copyWith(
        originalUrl: 'https://example.com/article',
      );
      final result = extract(articleHtml, opts);
      expect(result.contentText.isNotEmpty, isTrue);
    });

    test('invalid url throws', () {
      final opts = defaultOptions().copyWith(originalUrl: 'not a url');
      expect(
        () => extract(articleHtml, opts),
        throwsA(isA<TrafilaturaErrorExceptionParseError>()),
      );
    });

    test('invalid date throws', () {
      final opts = defaultOptions().copyWith(
        htmlDateOverride: '01/01/2024',
      );
      expect(
        () => extract(articleHtml, opts),
        throwsA(isA<TrafilaturaErrorExceptionParseError>()),
      );
    });
  });

  group('metadata', () {
    test('metadata extraction', () {
      final result = extractSimple(articleHtml);
      expect(result.metadata.title, equals('Test Article Title'));
    });
  });

  group('HTML output', () {
    test('content html present', () {
      final result = extractSimple(articleHtml);
      expect(result.contentHtml.isNotEmpty, isTrue);
      expect(result.contentHtml, contains('<'));
    });
  });

  group('createReadableDocument', () {
    test('creates readable document', () {
      final result = extractSimple(articleHtml);
      final doc = createReadableDocument(result);
      expect(doc, contains('<html'));
      expect(doc, contains('content-body'));
    });
  });

  group('defaultOptions', () {
    test('default option values', () {
      final opts = defaultOptions();
      expect(opts.originalUrl, isNull);
      expect(opts.targetLanguage, isNull);
      expect(opts.enableFallback, isFalse);
      expect(opts.focus, equals(ExtractionFocus.balanced));
      expect(opts.excludeComments, isFalse);
      expect(opts.excludeTables, isFalse);
      expect(opts.includeImages, isFalse);
      expect(opts.includeLinks, isFalse);
      expect(opts.deduplicate, isFalse);
      expect(opts.requireEssentialMetadata, isFalse);
      expect(opts.maxTreeSize, isNull);
      expect(opts.pruneSelector, isNull);
      expect(opts.htmlDateMode, equals(HtmlDateMode.automatic));
      expect(opts.htmlDateOverride, isNull);
    });
  });

  group('defaultConfig', () {
    test('default config values', () {
      final config = defaultConfig();
      expect(config.minExtractedSize, equals(250));
      expect(config.minExtractedCommentSize, equals(1));
      expect(config.minOutputSize, equals(1));
      expect(config.minOutputCommentSize, equals(1));
    });
  });
}
