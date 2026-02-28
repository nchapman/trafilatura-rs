// Port of go-trafilatura/settings.go and tag-converter.go

use std::collections::HashSet;
use std::sync::LazyLock;

pub(crate) static TAGS_TO_CLEAN: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // important
        "aside", "embed", "footer", "form", "head", "iframe", "menu", "object", "script",
        // other content
        "applet", "audio", "canvas", "figure", "map", "picture", "svg", "video",
        // secondary
        "area", "blink", "button", "datalist", "dialog", "frame", "frameset", "fieldset", "link",
        "input", "ins", "label", "legend", "marquee", "math", "menuitem", "nav", "noscript",
        "optgroup", "option", "output", "param", "progress", "rp", "rt", "rtc", "select", "source",
        "style", "track", "textarea", "time", "use",
    ]
    .into_iter()
    .collect()
});

pub(crate) static TAGS_TO_STRIP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "abbr", "acronym", "address", "bdi", "bdo", "big", "cite", "data", "dfn", "font", "hgroup",
        "img", "ins", "mark", "meta", "ruby", "small", "template", "tbody", "tfoot", "thead",
    ]
    .into_iter()
    .collect()
});

pub(crate) static EMPTY_TAGS_TO_REMOVE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "article",
        "b",
        "blockquote",
        "dd",
        "div",
        "dt",
        "em",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "i",
        "li",
        "main",
        "p",
        "pre",
        "q",
        "section",
        "span",
        "strong",
    ]
    .into_iter()
    .collect()
});

pub(crate) static TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "blockquote",
        "code",
        "del",
        "s",
        "strike",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "em",
        "i",
        "b",
        "strong",
        "u",
        "kbd",
        "samp",
        "tt",
        "var",
        "sub",
        "sup",
        "br",
        "hr",
        "ul",
        "ol",
        "dl",
        "p",
        "pre",
        "q",
        "details",
        "summary",
    ]
    .into_iter()
    .collect()
});

pub(crate) static FORMAT_TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "em", "i", "b", "strong", "u", "kbd", "samp", "tt", "var", "sub", "sup",
    ]
    .into_iter()
    .collect()
});

pub(crate) static VALID_TAG_CATALOG: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a",
        "abbr",
        "address",
        "area",
        "b",
        "base",
        "bdo",
        "blockquote",
        "body",
        "br",
        "button",
        "caption",
        "cite",
        "code",
        "col",
        "colgroup",
        "dd",
        "del",
        "dfn",
        "div",
        "dl",
        "dt",
        "em",
        "fieldset",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "hr",
        "html",
        "i",
        "iframe",
        "img",
        "input",
        "ins",
        "kbd",
        "label",
        "legend",
        "li",
        "link",
        "map",
        "menu",
        "meta",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "pre",
        "q",
        "s",
        "samp",
        "script",
        "select",
        "small",
        "span",
        "strong",
        "style",
        "sub",
        "sup",
        "table",
        "tbody",
        "td",
        "textarea",
        "tfoot",
        "th",
        "thead",
        "title",
        "tr",
        "u",
        "ul",
        "var",
        "article",
        "aside",
        "audio",
        "canvas",
        "command",
        "datalist",
        "details",
        "embed",
        "figcaption",
        "figure",
        "footer",
        "header",
        "mark",
        "meter",
        "nav",
        "output",
        "progress",
        "rp",
        "rt",
        "ruby",
        "section",
        "source",
        "summary",
        "time",
        "track",
        "video",
        "wbr",
    ]
    .into_iter()
    .collect()
});

pub(crate) static ELEMENT_WITH_SIZE_ATTR: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["table", "th", "td", "hr", "pre"].into_iter().collect());

// Tag category lists from tag-converter.go
// Used by element handlers in main-extractor.go for dispatch

pub(crate) static XML_LIST_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["ul", "ol", "dl"].into_iter().collect());

pub(crate) static XML_QUOTE_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["blockquote", "pre", "q"].into_iter().collect());

pub(crate) static XML_HEAD_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    ["h1", "h2", "h3", "h4", "h5", "h6", "summary"]
        .into_iter()
        .collect()
});

pub(crate) static XML_LB_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["br", "hr", "lb"].into_iter().collect());

pub(crate) static XML_HI_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "em", "i", "b", "strong", "u", "kbd", "samp", "tt", "var", "sub", "sup", "mark",
    ]
    .into_iter()
    .collect()
});

pub(crate) static XML_REF_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["a"].into_iter().collect());

pub(crate) static XML_GRAPHIC_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["img"].into_iter().collect());

pub(crate) static XML_ITEM_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["li", "dt", "dd"].into_iter().collect());

pub(crate) static XML_CELL_TAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["td", "th"].into_iter().collect());

/// Allowlist of HTML attributes retained during post-cleaning.
/// Taken from go-domdistiller (via go-trafilatura/settings.go).
pub(crate) static ALLOWED_ATTRIBUTES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "abbr",
        "accept-charset",
        "accept",
        "accesskey",
        "action",
        "align",
        "alink",
        "allow",
        "allowfullscreen",
        "allowpaymentrequest",
        "alt",
        "archive",
        "as",
        "async",
        "autocapitalize",
        "autocomplete",
        "autocorrect",
        "autofocus",
        "autoplay",
        "autopictureinpicture",
        "axis",
        "background",
        "behavior",
        "bgcolor",
        "border",
        "bordercolor",
        "capture",
        "cellpadding",
        "cellspacing",
        "char",
        "challenge",
        "charoff",
        "charset",
        "checked",
        "cite",
        "class",
        "classid",
        "clear",
        "code",
        "codebase",
        "codetype",
        "color",
        "cols",
        "colspan",
        "compact",
        "content",
        "contenteditable",
        "controls",
        "controlslist",
        "conversiondestination",
        "coords",
        "crossorigin",
        "csp",
        "data",
        "datetime",
        "declare",
        "decoding",
        "default",
        "defer",
        "dir",
        "direction",
        "dirname",
        "disabled",
        "disablepictureinpicture",
        "disableremoteplayback",
        "disallowdocumentaccess",
        "download",
        "draggable",
        "elementtiming",
        "enctype",
        "end",
        "enterkeyhint",
        "event",
        "exportparts",
        "face",
        "for",
        "form",
        "formaction",
        "formenctype",
        "formmethod",
        "formnovalidate",
        "formtarget",
        "frame",
        "frameborder",
        "headers",
        "height",
        "hidden",
        "high",
        "href",
        "hreflang",
        "hreftranslate",
        "hspace",
        "http-equiv",
        "id",
        "imagesizes",
        "imagesrcset",
        "importance",
        "impressiondata",
        "impressionexpiry",
        "incremental",
        "inert",
        "inputmode",
        "integrity",
        "is",
        "ismap",
        "keytype",
        "kind",
        "invisible",
        "label",
        "lang",
        "language",
        "latencyhint",
        "leftmargin",
        "link",
        "list",
        "loading",
        "longdesc",
        "loop",
        "low",
        "lowsrc",
        "manifest",
        "marginheight",
        "marginwidth",
        "max",
        "maxlength",
        "mayscript",
        "media",
        "method",
        "min",
        "minlength",
        "multiple",
        "muted",
        "name",
        "nohref",
        "nomodule",
        "nonce",
        "noresize",
        "noshade",
        "novalidate",
        "nowrap",
        "object",
        "open",
        "optimum",
        "part",
        "pattern",
        "placeholder",
        "playsinline",
        "ping",
        "policy",
        "poster",
        "preload",
        "pseudo",
        "readonly",
        "referrerpolicy",
        "rel",
        "reportingorigin",
        "required",
        "resources",
        "rev",
        "reversed",
        "role",
        "rows",
        "rowspan",
        "rules",
        "sandbox",
        "scheme",
        "scope",
        "scrollamount",
        "scrolldelay",
        "scrolling",
        "select",
        "selected",
        "shadowroot",
        "shadowrootdelegatesfocus",
        "shape",
        "size",
        "sizes",
        "slot",
        "span",
        "spellcheck",
        "src",
        "srcset",
        "srcdoc",
        "srclang",
        "standby",
        "start",
        "step",
        "style",
        "summary",
        "tabindex",
        "target",
        "text",
        "title",
        "topmargin",
        "translate",
        "truespeed",
        "trusttoken",
        "type",
        "usemap",
        "valign",
        "value",
        "valuetype",
        "version",
        "vlink",
        "vspace",
        "virtualkeyboardpolicy",
        "webkitdirectory",
        "width",
        "wrap",
    ]
    .into_iter()
    .collect()
});
