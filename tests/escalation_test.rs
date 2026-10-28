// Recall escalation & thread-forum handling.
//
// Ported from go-trafilatura v2.2.x trafilatura_test.go (test_recall_escalation*)
// and the python-2.2.0-reference.json forum samples. Python 2.3.1 carries the
// same scenarios (tests/unit_tests.py) with two Python-only cases (dedicated
// justext candidate, comment-list pruning variant) we do not port.

use trafilatura::options::{ExtractionFocus, Options};
use trafilatura::{extract, html2txt};

const ESCALATION_INTRO: &str = "<p>Introductory paragraph with regular prose content that is moderately long and clearly meaningful, providing enough text to exceed the minimum extraction size threshold used by the extractor. Introductory paragraph with regular prose content that is moderately long and clearly meaningful, providing enough text to exceed the minimum extraction size threshold used by the extractor. </p>";

const FORUM_JSONLD: &str = r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"DiscussionForumPosting","headline":"Test thread"}</script>"#;

/// Thread page: article intro plus 8 replies in a comments-classed container.
fn thread_doc(jsonld: &str, reply_tag: &str, intro: &str, wrap: bool) -> String {
    let mut replies = String::new();
    for i in 0..8 {
        replies.push_str(&format!(
            "<{reply_tag} class='comment-body'>Reply number {i} contains substantial discussion content with plenty of genuine words and enough length to be recognized as real text by a paragraph density classifier, not boilerplate at all today, said the commenter.</{reply_tag}>"
        ));
    }
    let wrapper = if wrap { "article" } else { "div" };
    format!("<html><head>{jsonld}</head><body><{wrapper}>{intro}</{wrapper}><div id='comments' class='comments-area'>{replies}</div></body></html>")
}

fn text_of(html: &str, opts: Options) -> String {
    extract(html, &opts)
        .map(|r| r.content_text)
        .unwrap_or_default()
}

#[test]
fn test_recall_escalation() {
    // A short balanced extraction covering little of the page is retried in
    // recall mode: wild paragraphs block the baseline rescue while the bulk of
    // the content sits in bare divs.
    let mut doc = String::from("<html><body>");
    for i in 0..3 {
        doc.push_str(&format!(
            "<p>Wild paragraph number {i} directly under body, with enough words to pass the paragraph checks in place.</p>"
        ));
    }
    for i in 0..30 {
        doc.push_str(&format!(
            "<div>Main content block {i} living in a bare div element with plenty of meaningful words to matter here today.</div>"
        ));
    }
    doc.push_str("</body></html>");

    let text = text_of(&doc, Options::default());
    assert_eq!(text.matches("Wild paragraph").count(), 3, "got: {text}");
    assert_eq!(
        text.matches("Main content block").count(),
        30,
        "got: {text}"
    );
}

#[test]
fn test_recall_escalation_comment_scoping() {
    // Comment containers must be stripped before the retry runs on a blog
    // (reader replies are not content) but kept on a thread-forum, where those
    // same containers hold the actual posts. The "fast" case is the rule-based
    // retry alone (no external fallback).
    for (name, jsonld, fast, expect_replies) in [
        ("blog_excludes_comments", "", false, 0),
        ("forum_keeps_posts", FORUM_JSONLD, false, 8),
        ("blog_excludes_comments_fast", "", true, 0),
    ] {
        let mut opts = Options::default();
        opts.exclude_comments = true;
        opts.enable_fallback = !fast;
        let text = text_of(&thread_doc(jsonld, "div", ESCALATION_INTRO, true), opts);
        assert!(text.contains("Introductory paragraph"), "{name}: {text}");
        assert_eq!(
            text.matches("Reply number").count(),
            expect_replies,
            "{name}: {text}"
        );
    }
}

#[test]
fn test_recall_escalation_no_comment_doubling() {
    // With comments included on a thread-forum page, posts must not appear in
    // both the body and the comments channel. <p> replies are capturable as
    // comments, <div> replies are not — either way: exactly one occurrence
    // per post overall.
    for reply_tag in ["p", "div"] {
        let mut opts = Options::default();
        opts.enable_fallback = true;
        let text = text_of(
            &thread_doc(FORUM_JSONLD, reply_tag, ESCALATION_INTRO, true),
            opts,
        );
        assert!(
            text.contains("Introductory paragraph"),
            "{reply_tag}: {text}"
        );
        assert_eq!(
            text.matches("Reply number").count(),
            8,
            "{reply_tag}: {text}"
        );
    }
}

#[test]
fn test_dfp_long_opening_post_keeps_replies() {
    // The forum re-route empties the captured comments betting on the
    // escalation to restore the posts — a thread whose opening post alone
    // exceeds the escalation length gate must not lose every reply.
    let mut long_intro = String::new();
    for i in 0..16 {
        long_intro.push_str(&format!(
            "<p>Opening post paragraph {i} lays out the question in full detail, with context, history and several worked examples that make the thread starter alone longer than the escalation length gate, so no rescue pass can be relied upon for the replies.</p>"
        ));
    }
    let text = text_of(
        &thread_doc(FORUM_JSONLD, "p", &long_intro, true),
        Options::default(),
    );
    assert!(text.contains("Opening post paragraph"), "got: {text}");
    assert_eq!(text.matches("Reply number").count(), 8, "got: {text}");
}

#[test]
fn test_dfp_precision_keeps_posts() {
    // Precision mode closes the rescue passes; captured forum posts must
    // survive regardless via the salvage.
    let mut opts = Options::default();
    opts.focus = ExtractionFocus::FavorPrecision;
    let text = text_of(&thread_doc(FORUM_JSONLD, "p", ESCALATION_INTRO, true), opts);
    assert_eq!(text.matches("Reply number").count(), 8, "got: {text}");
}

#[test]
fn test_recall_escalation_blog_comment_leak() {
    // On a wrapper-less blog layout with comments included, an uncapturable
    // div-based reply thread must not leak into the body through the retry.
    for fast in [false, true] {
        let mut opts = Options::default();
        opts.enable_fallback = !fast;
        let text = text_of(&thread_doc("", "div", ESCALATION_INTRO, false), opts);
        assert!(
            text.contains("Introductory paragraph"),
            "fast={fast}: {text}"
        );
        assert_eq!(
            text.matches("Reply number").count(),
            0,
            "fast={fast}: {text}"
        );
    }
}

#[test]
fn test_main_pass_excludes_comments_when_disabled() {
    // With comments excluded, comment containers must be pruned before the
    // main pass (not only in precision mode), including WordPress-style
    // ol/ul comment lists on the raw tree.
    let intro = "<p>Short intro under the escalation and rescue thresholds here.</p>";
    let mut replies = String::new();
    for i in 0..8 {
        replies.push_str(&format!(
            "<div>Reader comment number {i} that must never appear when comments are excluded, long enough to matter.</div>"
        ));
    }
    let items = replies.replace("<div>", "<li>").replace("</div>", "</li>");
    let docs = [
        format!("<html><body><article>{intro}</article><div id='comments' class='comments-area'>{replies}</div></body></html>"),
        format!("<html><body><article>{intro}</article><ol class='commentlist'>{items}</ol></body></html>"),
        format!("<html><body><article>{intro}</article><ul class='comment-list'>{items}</ul></body></html>"),
    ];
    for doc in &docs {
        for fast in [false, true] {
            let mut opts = Options::default();
            opts.exclude_comments = true;
            opts.enable_fallback = !fast;
            let text = text_of(doc, opts);
            assert!(text.contains("Short intro"), "fast={fast}: {text}");
            assert_eq!(
                text.matches("Reader comment number").count(),
                0,
                "fast={fast}: {text}"
            );
        }
    }
}

#[test]
fn test_main_pass_excludes_details_wrapped_comments() {
    // Comment containers wrapped in <details> must also be pruned when
    // comments are excluded; a <details> without a comment id/class is kept.
    let body = "<article>".to_owned()
        + &"<p>Real article paragraph with enough content to be extracted normally here.</p>"
            .repeat(3)
        + "</article>";
    let mut comments = String::from("<details id='comments'><summary>Comments</summary>");
    for i in 0..6 {
        comments.push_str(&format!(
            "<p>Reader comment number {i} that must never leak into the body text.</p>"
        ));
    }
    comments.push_str("</details>");

    for fast in [false, true] {
        let mut opts = Options::default();
        opts.exclude_comments = true;
        opts.enable_fallback = !fast;
        let text = text_of(&format!("<html><body>{body}{comments}</body></html>"), opts);
        assert!(
            text.contains("Real article paragraph"),
            "fast={fast}: {text}"
        );
        assert_eq!(
            text.matches("Reader comment number").count(),
            0,
            "fast={fast}: {text}"
        );
    }

    let faq = "<details class='faq'><summary>More</summary><p>Kept expandable content paragraph that is genuine.</p></details>";
    let text = text_of(
        &format!("<html><body>{body}{faq}</body></html>"),
        Options::default(),
    );
    assert!(text.contains("Kept expandable content"), "got: {text}");
}

#[test]
fn test_escalation_retry_no_comment_capture() {
    // Ported from Python's test_escalation_retry_no_comment_capture: the
    // retry must not capture comments — a container matching the comments
    // selector had its text captured and deleted inside the retry, so it
    // vanished from both channels.
    let mut intro = String::new();
    for i in 0..4 {
        intro.push_str(&format!(
            "<li>Point number {i} of the short visible article summary text here.</li>"
        ));
    }
    let mut cont = String::new();
    for i in 0..12 {
        cont.push_str(&format!(
            "<p>Continuation paragraph {i} with substantial article prose that only the recall retry can recover from inside the form wrapper element.</p>"
        ));
    }
    let mut replies = String::new();
    for i in 0..8 {
        replies.push_str(&format!(
            "<p>Reply {i} ZQXJKVREPLY says something reasonably long about the topic at hand in this discussion thread below the article body.</p>"
        ));
    }
    let doc = format!(
        "<html><body><article><h1>Title of the page</h1><ul>{intro}</ul><form>{cont}<div class='user-comment-area'>{replies}</div></form></article></body></html>"
    );
    let text = text_of(&doc, Options::default());
    assert!(text.contains("Continuation paragraph 5"), "got: {text}");
    assert!(text.contains("ZQXJKVREPLY"), "got: {text}");
}

#[test]
fn test_forum_detection_via_extraction() {
    // forum_thread_page is internal; observe it through the pipeline instead:
    // a forum page routes captured replies into the body, a blog does not.
    // The fine-grained regex matrix below is exercised in the lib tests.
    let mut body = String::from("<article>");
    body.push_str(ESCALATION_INTRO);
    body.push_str("</article>");
    let mut replies = String::new();
    for i in 0..8 {
        replies.push_str(&format!(
            "<p class='comment-body'>Reply number {i} contains substantial discussion content with plenty of genuine words and enough length to be recognized as real text by a paragraph density classifier, not boilerplate at all today.</p>"
        ));
    }
    let forum = format!(
        "<html><head>{FORUM_JSONLD}</head><body>{body}<div id='comments' class='comments-area'>{replies}</div></body></html>"
    );
    let text = text_of(&forum, Options::default());
    assert_eq!(text.matches("Reply number").count(), 8, "got: {text}");
}

// ---------------------------------------------------------------------------
// html2txt public API smoke (details in lib baseline tests)
// ---------------------------------------------------------------------------

#[test]
fn test_html2txt_public() {
    assert_eq!(
        html2txt("<html><body><p>First block.</p><p>Second block.</p></body></html>"),
        "First block. Second block."
    );
    assert_eq!(html2txt("123"), "");
}
